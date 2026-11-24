//! Delivers agent alerts through macOS and routes notification clicks to the main window.

use std::cell::OnceCell;

use block2::{DynBlock, RcBlock};
use objc2::rc::Retained;
use objc2::runtime::{Bool, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{ClassType, MainThreadMarker, define_class};
use objc2_app_kit::NSApplication;
use objc2_foundation::{NSBundle, NSDictionary, NSError, NSString, NSUUID, ns_string};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNErrorCode, UNErrorDomain, UNMutableNotificationContent,
    UNNotification, UNNotificationDefaultActionIdentifier, UNNotificationPresentationOptions,
    UNNotificationRequest, UNNotificationResponse, UNUserNotificationCenter,
    UNUserNotificationCenterDelegate,
};
use serde_json::{Value, json};

use crate::mods::{self, Request};

define_class!(
    #[unsafe(super(NSObject))]
    struct NotificationDelegate;

    unsafe impl NSObjectProtocol for NotificationDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for NotificationDelegate {
        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn clicked(
            &self,
            _: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            complete: &DynBlock<dyn Fn()>,
        ) {
            if &*response.actionIdentifier() == unsafe { UNNotificationDefaultActionIdentifier }
                && let Some(address) = field(
                    &response.notification().request().content().userInfo(),
                    ns_string!("address"),
                )
            {
                mods::request(Request::FocusNotification { address });
            }
            complete.call(());
        }

        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn present(
            &self,
            _: &UNUserNotificationCenter,
            notification: &UNNotification,
            complete: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            // Canopy can come forward between scheduling and delivery.
            let content = notification.request().content();
            let info = content.userInfo();
            if let Some(address) = field(&info, ns_string!("address")) {
                toast(json!({
                    "title": content.title().to_string(),
                    "body": field(&info, ns_string!("toastBody")),
                    "level": field(&info, ns_string!("toastLevel")),
                    "action": { "name": "agent.focus", "args": { "terminal": address } },
                }));
            }
            complete.call((UNNotificationPresentationOptions::empty(),));
        }
    }
);

struct Notifications {
    center: Retained<UNUserNotificationCenter>,
    _delegate: Retained<NotificationDelegate>,
    authorization: OnceCell<async_channel::Receiver<()>>,
}
impl gpui::Global for Notifications {}

/// Retain the weakly held delegate from launch so existing notifications can open Canopy.
pub(crate) fn init(cx: &mut gpui::App) {
    let bundle = NSBundle::mainBundle();
    if bundle.bundleIdentifier().as_deref() != Some(ns_string!("dev.canopy.app")) {
        return;
    }
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let delegate: Retained<NotificationDelegate> =
        unsafe { objc2::msg_send![NotificationDelegate::class(), new] };
    center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    cx.set_global(Notifications {
        center,
        _delegate: delegate,
        authorization: OnceCell::new(),
    });
}

fn field(info: &NSDictionary, key: &NSString) -> Option<String> {
    info.objectForKey(key)?
        .downcast::<NSString>()
        .ok()
        .map(|s| s.to_string())
}

fn toast(notification: Value) {
    mods::request(Request::NotificationToast { notification });
}

fn app_active() -> bool {
    let mtm = MainThreadMarker::new().expect("agent alerts run on the main thread");
    NSApplication::sharedApplication(mtm).isActive()
}

/// Returns false if the app should show its usual toast immediately.
pub(crate) fn post(
    title: &str,
    body: impl FnOnce() -> String,
    address: &str,
    fallback: &Value,
    cx: &gpui::App,
) -> bool {
    let Some(native) = cx.try_global::<Notifications>() else {
        return false;
    };
    if app_active() {
        return false;
    }
    let center = native.center.clone();
    let authorized = native
        .authorization
        .get_or_init(|| {
            let (tx, rx) = async_channel::bounded::<()>(1);
            // macOS keeps the user's answer. All alerts wait for the first request to finish.
            let complete = RcBlock::new(move |_: Bool, _: *mut NSError| {
                tx.close();
            });
            center.requestAuthorizationWithOptions_completionHandler(
                UNAuthorizationOptions::Alert,
                &complete,
            );
            rx
        })
        .clone();

    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str(title));
    content.setBody(&NSString::from_str(&body()));
    let address = NSString::from_str(address);
    let toast_body = NSString::from_str(fallback["body"].as_str().unwrap_or_default());
    let toast_level = NSString::from_str(fallback["level"].as_str().unwrap_or_default());
    let info = NSDictionary::from_slices(
        &[
            ns_string!("address"),
            ns_string!("toastBody"),
            ns_string!("toastLevel"),
        ],
        &[&*address, &*toast_body, &*toast_level],
    );
    // Each value is a property-list string.
    unsafe { content.setUserInfo(info.cast_unchecked()) };
    let fallback = fallback.clone();
    cx.spawn(async move |_| {
        let _ = authorized.recv().await;
        if app_active() || !crate::settings::get().desktop_notifications {
            toast(fallback);
            return;
        }
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSUUID::new().UUIDString(),
            &content,
            None,
        );
        let complete = RcBlock::new(move |error: *mut NSError| {
            if let Some(error) = unsafe { error.as_ref() } {
                let denied = unsafe { UNErrorDomain } == Some(&*error.domain())
                    && error.code() == UNErrorCode::NotificationsNotAllowed.0;
                let mut fallback = fallback.clone();
                if !denied {
                    fallback["body"] = format!(
                        "macOS couldn't show the notification: {}",
                        error.localizedDescription()
                    )
                    .into();
                }
                toast(fallback);
            }
        });
        center.addNotificationRequest_withCompletionHandler(&request, Some(&complete));
    })
    .detach();
    true
}
