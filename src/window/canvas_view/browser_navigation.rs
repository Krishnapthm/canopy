//! Tracks main-frame WebKit loads while keeping wry's navigation and download policies.

use std::{cell::{Cell, RefCell}, ffi::c_void, ptr};
use objc2::{DefinedClass, MainThreadOnly, Message, define_class, msg_send, rc::{Retained, Weak}, runtime::{AnyObject, Bool, NSObject, ProtocolObject, Sel}};
use objc2_foundation::{MainThreadMarker, NSError, NSKeyValueObservingOptions, NSObjectNSKeyValueObserverRegistration, NSObjectProtocol, NSString, NSURL, NSURLRequest, ns_string};
use objc2_web_kit::{WKNavigation, WKNavigationDelegate, WKWebView};

use super::browser::Event;

pub(super) const FIRST_LOAD: u64 = 1;

pub(super) struct State {
    view: Retained<WKWebView>,
    fragment: RefCell<Option<String>>,
    wry: Retained<ProtocolObject<dyn WKNavigationDelegate>>,
    send: Box<dyn Fn(Event)>,
    load: Cell<u64>,
    navigation: RefCell<Option<Retained<WKNavigation>>>,
    superseded: RefCell<Vec<Weak<WKNavigation>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "CanopyBrowserNavigation"]
    #[thread_kind = MainThreadOnly]
    #[ivars = State]
    pub(super) struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {
        #[unsafe(method(respondsToSelector:))]
        fn responds(&self, selector: Sel) -> Bool {
            // SAFETY: NSObject implements this method and the selector is supplied by WebKit.
            let own: Bool = unsafe { msg_send![super(self), respondsToSelector: selector] };
            Bool::new(own.as_bool() || self.ivars().wry.respondsToSelector(selector))
        }
    }

    impl Delegate {
        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        fn url_changed(&self, _: *const AnyObject, _: *const AnyObject, _: *const AnyObject, _: *mut c_void) {
            self.complete_fragment();
        }

        #[unsafe(method(forwardingTargetForSelector:))]
        fn forward(&self, _: Sel) -> *const AnyObject {
            Retained::as_ptr(&self.ivars().wry).cast()
        }
    }

    unsafe impl WKNavigationDelegate for Delegate {
        #[unsafe(method(webView:didStartProvisionalNavigation:))]
        fn started(&self, _: &WKWebView, navigation: Option<&WKNavigation>) {
            let Some(navigation) = navigation else { return };
            let state = self.ivars();
            if state.superseded.borrow().iter()
                .any(|old| old.load().as_deref().is_some_and(|old| ptr::eq(old, navigation)))
            {
                return;
            }
            state.fragment.take();
            if self.matches(Some(navigation)) { return; }
            // Canopy already reserved a generation for a local file waiting on its rules.
            if state.navigation.borrow().is_some() { self.expect_load(); }
            self.bind(navigation.retain());
            (state.send)(Event::Started(self.load()));
        }

        #[unsafe(method(webView:didFinishNavigation:))]
        fn finished(&self, _: &WKWebView, navigation: Option<&WKNavigation>) {
            if self.matches(navigation) { (self.ivars().send)(Event::Loaded(self.load())); }
        }

        #[unsafe(method(webView:didFailProvisionalNavigation:withError:))]
        fn failed_provisional(&self, _: &WKWebView, navigation: Option<&WKNavigation>, error: &NSError) {
            self.failed(navigation, error);
        }

        #[unsafe(method(webView:didFailNavigation:withError:))]
        fn failed_navigation(&self, _: &WKWebView, navigation: Option<&WKNavigation>, error: &NSError) {
            self.failed(navigation, error);
        }
    }
);

impl Delegate {
    pub(super) fn attach(view: &WKWebView, send: impl Fn(Event) + 'static) -> Result<Retained<Self>, String> {
        // SAFETY: the view and its delegate are initialized on the main thread.
        unsafe {
            let wry = view.navigationDelegate().ok_or("WebKit has no navigation delegate")?;
            let delegate = MainThreadMarker::from(view).alloc::<Self>().set_ivars(State {
                view: view.retain(),
                fragment: RefCell::new(None),
                wry,
                send: Box::new(send),
                load: Cell::new(FIRST_LOAD - 1),
                navigation: RefCell::new(None),
                superseded: RefCell::new(Vec::new()),
            });
            let delegate: Retained<Self> = msg_send![super(delegate), init];
            view.setNavigationDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            view.addObserver_forKeyPath_options_context(&delegate, ns_string!("URL"), NSKeyValueObservingOptions::empty(), ptr::null_mut());
            Ok(delegate)
        }
    }

    pub(super) fn load(&self) -> u64 { self.ivars().load.get() }

    pub(super) fn expect_load(&self) {
        let state = self.ivars();
        state.load.set(self.load() + 1);
        state.fragment.take();
        // WebKit can still deliver didStart for a request we have already replaced.
        let mut superseded = state.superseded.borrow_mut();
        superseded.retain(|navigation| navigation.load().is_some());
        if let Some(previous) = state.navigation.take() {
            superseded.push(Weak::new(&previous));
        }
    }

    pub(super) fn bind(&self, navigation: Retained<WKNavigation>) {
        self.ivars().navigation.replace(Some(navigation));
    }

    pub(super) fn open(&self, address: &str) -> Result<(), String> {
        let url = NSURL::URLWithString(&NSString::from_str(address)).ok_or("Invalid browser URL")?;
        self.expect_load();
        let state = self.ivars();
        let view = &state.view;
        // SAFETY: this is the main-thread web view that owns this delegate.
        unsafe {
            // A same-document fragment changes URL without a navigation finish callback.
            let address = url.absoluteString().map(|url| url.to_string()).unwrap_or_default();
            if !view.isLoading() && let Some((base, _)) = address.split_once('#')
                && self.current_url().is_some_and(|current| current.split('#').next() == Some(base))
            {
                state.fragment.replace(Some(address));
            }
            let navigation = view.loadRequest(&NSURLRequest::requestWithURL(&url)).ok_or("WebKit could not start loading the page")?;
            self.bind(navigation);
        }
        self.complete_fragment();
        Ok(())
    }

    fn current_url(&self) -> Option<String> {
        // SAFETY: the stored view is live on the main thread.
        unsafe { self.ivars().view.URL()? }.absoluteString().map(|url| url.to_string())
    }

    fn complete_fragment(&self) {
        let state = self.ivars();
        let mut fragment = state.fragment.borrow_mut();
        if fragment.as_ref().is_some_and(|address| self.current_url().as_ref() == Some(address)) {
            fragment.take();
            (state.send)(Event::Loaded(self.load()));
        }
    }

    fn matches(&self, navigation: Option<&WKNavigation>) -> bool {
        self.ivars().navigation.borrow().as_deref().zip(navigation).is_some_and(|(current, event)| ptr::eq(current, event))
    }

    fn failed(&self, navigation: Option<&WKNavigation>, error: &NSError) {
        if self.matches(navigation) {
            (self.ivars().send)(Event::LoadError(self.load(), error.localizedDescription().to_string()));
        }
    }
}

impl Drop for Delegate {
    fn drop(&mut self) {
        // SAFETY: attach registered this observer on this live main-thread view.
        unsafe { self.ivars().view.removeObserver_forKeyPath(self, ns_string!("URL")); }
    }
}
