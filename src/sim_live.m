// Live iOS Simulator panes on macOS. Two halves, both compiled into canopy:
//
// - The helper (`canopy --sim-helper ...`, run by Canopy under sandbox-exec) opens a booted
//   simulator through CoreSimulator, hands its framebuffer IOSurface to Canopy over a Mach port,
//   pings Canopy on stdout once per presented frame, and sends the input it reads on stdin to the
//   guest over DTUHID.
// - The host functions (canopy_sim_*) run in Canopy: the Mach rendezvous, the IOSurface ->
//   CVPixelBuffer wrap that GPUI's surface() draws, a BGRA copy for screenshots, and the
//   bezel artwork rendered from Xcode's DeviceKit PDFs.
//
// The CoreSimulator selectors, the DTUHID wire format and the _4sim XPC calls follow facebook/idb
// (MIT, see THIRD-PARTY-NOTICES): FBSimulatorControl/Framebuffer/FramebufferSurface.swift,
// HID/DTUHIDModels.swift, HID/SimulatorDTUHIDConnection.swift and XPC/SimulatorXPCConnection.swift.
// Facts verified on this toolchain are in docs/canopy/harness-notes/xcode-simctl.md.

#import <CoreGraphics/CoreGraphics.h>
#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>
#import <IOSurface/IOSurface.h>
#import <ImageIO/ImageIO.h>
#import <dlfcn.h>
#import <mach/mach.h>
#import <servers/bootstrap.h>
#import <xpc/xpc.h>

// Private CoreSimulator selectors, sent dynamically; every call can raise through the remote proxy.
@interface NSObject (CanopySim)
+ (id)sharedServiceContextForDeveloperDir:(id)dir error:(id *)error;
- (id)defaultDeviceSetWithError:(id *)error;
- (NSDictionary *)devicesByUDID;
- (unsigned long long)state;
- (id)io;
- (NSArray *)ioPorts;
- (id)descriptor;
- (unsigned short)displayClass;
- (id)framebufferSurface;
- (void)registerScreenCallbacksWithUUID:(NSUUID *)uuid callbackQueue:(dispatch_queue_t)queue frameCallback:(void (^)(void))frame surfacesChangedCallback:(void (^)(id, id))surfaces propertiesChangedCallback:(void (^)(id))properties;
- (void)unregisterScreenCallbacksWithUUID:(NSUUID *)uuid;
- (unsigned int)lookup:(NSString *)service error:(NSError **)error;
@end

// One Mach message: msgh_id 1 carries the framebuffer surface as a port descriptor.
typedef struct {
  mach_msg_header_t header;
  mach_msg_body_t body;
  mach_msg_port_descriptor_t surface;
} SurfaceMessage;

typedef struct {
  SurfaceMessage message;
  mach_msg_trailer_t trailer;
} SurfaceReceive;

#pragma mark - Helper

static const char *const kDigitizer = "com.apple.coredevice.feature.remote.hid.digitizer";

// Lines on stdout are the helper's whole protocol to Canopy; one writer at a time.
static void say(const char *format, ...) {
  static dispatch_once_t once;
  static NSLock *lock;
  dispatch_once(&once, ^{ lock = [NSLock new]; });
  va_list args;
  va_start(args, format);
  [lock lock];
  vfprintf(stdout, format, args);
  fputc('\n', stdout);
  fflush(stdout);
  [lock unlock];
  va_end(args);
}

static void fail(const char *reason) {
  say("error %s", reason);
  exit(1);
}

// The dtuhidd envelope (idb DTUHIDMessage): integers ride as uint64.
static xpc_object_t envelope(const char *type, const char *service, xpc_object_t payload, bool barrier) {
  xpc_object_t message = xpc_dictionary_create(NULL, NULL, 0);
  xpc_dictionary_set_string(message, "messageType", type);
  xpc_dictionary_set_bool(message, "isBarrier", barrier);
  xpc_dictionary_set_string(message, "featureIdentifier", service);
  xpc_dictionary_set_value(message, "payload", payload);
  return message;
}

static xpc_object_t key_event(const char *service, uint64_t usage, uint64_t state, bool barrier) {
  xpc_object_t payload = xpc_dictionary_create(NULL, NULL, 0);
  xpc_dictionary_set_uint64(payload, "usageCode", usage);
  xpc_dictionary_set_uint64(payload, "state", state);
  return envelope("IndigoKeyboardButtonEvent", service, payload, barrier);
}

static xpc_object_t point(double x, double y) {
  xpc_object_t p = xpc_dictionary_create(NULL, NULL, 0);
  xpc_dictionary_set_double(p, "x", x);
  xpc_dictionary_set_double(p, "y", y);
  return p;
}

// A host connection to a guest service (idb SimulatorXPCConnection): the private _4sim endpoint,
// marked sim-to-host, without which the service sees the peer but never a payload.
static xpc_connection_t guest_connection(id device, const char *service) {
  typedef xpc_object_t (*EndpointFromPort)(mach_port_t, uint64_t, uint64_t);
  typedef void (*EnableSim2Host)(xpc_connection_t);
  EndpointFromPort endpoint_from_port = (EndpointFromPort)dlsym(RTLD_DEFAULT, "xpc_endpoint_create_mach_port_4sim");
  EnableSim2Host enable_sim2host = (EnableSim2Host)dlsym(RTLD_DEFAULT, "xpc_connection_enable_sim2host_4sim");
  if (!endpoint_from_port || !enable_sim2host) return NULL;
  NSError *error = nil;
  unsigned int port = 0;
  @try {
    port = [device lookup:@(service) error:&error];
  } @catch (NSException *e) {
    return NULL;
  }
  if (port == MACH_PORT_NULL) return NULL;
  xpc_object_t endpoint = endpoint_from_port(port, 0, 0);
  if (!endpoint) return NULL;
  xpc_connection_t connection = xpc_connection_create_from_endpoint((xpc_endpoint_t)endpoint);
  if (!connection) return NULL;
  enable_sim2host(connection);
  xpc_connection_set_target_queue(connection, dispatch_queue_create("dev.canopy.sim.hid", DISPATCH_QUEUE_SERIAL));
  xpc_connection_set_event_handler(connection, ^(__unused xpc_object_t event) {});
  xpc_connection_resume(connection);
  return connection;
}

// True once dtuhidd answers a barrier. The lookup succeeds whether or not the daemon can run, so an
// answered barrier is the only proof; idb retries for a daemon still starting after a boot.
static bool alive(xpc_connection_t connection, const char *service) {
  for (int attempt = 0; attempt < 5; attempt++) {
    dispatch_semaphore_t answered = dispatch_semaphore_create(0);
    __block bool ok = false;
    xpc_connection_send_message_with_reply(connection, key_event(service, 0, 2, true), dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^(xpc_object_t reply) {
      ok = xpc_get_type(reply) == XPC_TYPE_DICTIONARY;
      dispatch_semaphore_signal(answered);
    });
    if (dispatch_semaphore_wait(answered, dispatch_time(DISPATCH_TIME_NOW, 4 * NSEC_PER_SEC)) == 0 && ok) return true;
    [NSThread sleepForTimeInterval:4];
  }
  return false;
}

// The simulator's main display (displayClass 0), as its IO port descriptor.
static id main_screen(id device) {
  @try {
    for (id port in [[device io] ioPorts]) {
      id descriptor = [port descriptor];
      if (![descriptor respondsToSelector:@selector(framebufferSurface)] || ![descriptor respondsToSelector:@selector(state)]) continue;
      id state = [descriptor performSelector:@selector(state)];
      if ([state respondsToSelector:@selector(displayClass)] && [state displayClass] == 0) return descriptor;
    }
  } @catch (NSException *e) {
  }
  return nil;
}

// Hands `surface` to Canopy, which looks it up from the port.
static void send_surface(mach_port_t canopy, IOSurfaceRef surface) {
  SurfaceMessage message = {0};
  message.header.msgh_bits = MACH_MSGH_BITS(MACH_MSG_TYPE_COPY_SEND, 0) | MACH_MSGH_BITS_COMPLEX;
  message.header.msgh_size = sizeof message;
  message.header.msgh_remote_port = canopy;
  message.header.msgh_id = 1;
  message.body.msgh_descriptor_count = 1;
  message.surface.name = IOSurfaceCreateMachPort(surface);
  message.surface.disposition = MACH_MSG_TYPE_MOVE_SEND;
  message.surface.type = MACH_MSG_PORT_DESCRIPTOR;
  if (mach_msg(&message.header, MACH_SEND_MSG | MACH_SEND_TIMEOUT, sizeof message, 0, MACH_PORT_NULL, 1000, MACH_PORT_NULL) != KERN_SUCCESS) {
    mach_port_deallocate(mach_task_self(), message.surface.name);
    fail("Canopy stopped listening for the simulator screen");
  }
  say("surface %zu %zu", IOSurfaceGetWidth(surface), IOSurfaceGetHeight(surface));
}

// One input line from Canopy (see sim_live.rs `Input`):
//   t <phase 0 start|1 move|2 end> <x> <y>   a touch, normalized 0..1 from the top left
//   k <usage> <1 down|2 up>                  a keyboard usage (HID page 7)
//   b <page> <usage> <1 down|2 up>           a hardware button
//   g <ms> <x1> <y1> <x2> <y2>               a whole touch from one point to another over ms
//                                            (a tap when both are the same), timed here
static void touch(xpc_connection_t digitizer, unsigned long long phase, double x, double y) {
  xpc_object_t payload = xpc_dictionary_create(NULL, NULL, 0);
  xpc_dictionary_set_value(payload, "pointOne", point(x, y));
  xpc_dictionary_set_uint64(payload, "eventType", phase);
  xpc_dictionary_set_uint64(payload, "edge", 0);
  xpc_dictionary_set_uint64(payload, "target", 0);
  xpc_connection_send_message(digitizer, envelope("IndigoDigitizerEvent", kDigitizer, payload, false));
}

static void handle(char *line, xpc_connection_t digitizer) {
  double x, y, x2, y2;
  unsigned long long a, b, c;
  if (sscanf(line, "t %llu %lf %lf", &a, &x, &y) == 3) {
    touch(digitizer, a, x, y);
  } else if (sscanf(line, "g %llu %lf %lf %lf %lf", &a, &x, &y, &x2, &y2) == 5) {
    // One move per 60 Hz frame, so a swipe has the speed it asks for.
    unsigned long long steps = a / 16 > 1 ? a / 16 : 1;
    touch(digitizer, 0, x, y);
    for (unsigned long long i = 1; i <= steps; i++) {
      usleep((useconds_t)(a * 1000 / steps));
      double t = (double)i / steps;
      touch(digitizer, 1, x + (x2 - x) * t, y + (y2 - y) * t);
    }
    touch(digitizer, 2, x2, y2);
  } else if (sscanf(line, "k %llu %llu", &a, &b) == 2) {
    xpc_connection_send_message(digitizer, key_event(kDigitizer, a, b, false));
  } else if (sscanf(line, "b %llu %llu %llu", &a, &b, &c) == 3) {
    xpc_object_t payload = xpc_dictionary_create(NULL, NULL, 0);
    xpc_dictionary_set_uint64(payload, "usagePage", a);
    xpc_dictionary_set_uint64(payload, "usageCode", b);
    xpc_dictionary_set_uint64(payload, "state", c);
    xpc_connection_send_message(digitizer, envelope("IndigoButtonEvent", kDigitizer, payload, false));
  }
}

int canopy_sim_helper(const char *developer_dir, const char *udid, const char *rendezvous) {
  @autoreleasepool {
    if (!dlopen("/Library/Developer/PrivateFrameworks/CoreSimulator.framework/CoreSimulator", RTLD_NOW)) fail("CoreSimulator is not installed");
    mach_port_t canopy = MACH_PORT_NULL;
    if (bootstrap_look_up(bootstrap_port, rendezvous, &canopy) != KERN_SUCCESS) fail("Canopy's simulator port is gone");
    id device = nil;
    @try {
      NSError *error = nil;
      id context = [NSClassFromString(@"SimServiceContext") sharedServiceContextForDeveloperDir:@(developer_dir) error:&error];
      id set = [context defaultDeviceSetWithError:&error];
      device = [set devicesByUDID][[[NSUUID alloc] initWithUUIDString:@(udid)]];
    } @catch (NSException *e) {
      fail("CoreSimulator did not answer");
    }
    if (!device) fail("This simulator no longer exists");
    if ([device state] != 3) fail("The simulator is not booted");
    id screen = main_screen(device);
    if (!screen) fail("The simulator has no screen yet");

    // Frames and surface changes arrive on one serial queue, in order, while the render server
    // waits for the callback: keep them tiny.
    dispatch_queue_t frames = dispatch_queue_create("dev.canopy.sim.frames", dispatch_queue_attr_make_with_qos_class(DISPATCH_QUEUE_SERIAL, QOS_CLASS_USER_INTERACTIVE, 0));
    NSUUID *token = [NSUUID UUID];
    __block IOSurfaceRef shown = NULL;
    @try {
      [screen registerScreenCallbacksWithUUID:token callbackQueue:frames
        frameCallback:^{ say("f"); }
        surfacesChangedCallback:^(id framebuffer, __unused id masked) {
          IOSurfaceRef surface = (__bridge IOSurfaceRef)framebuffer;
          if (!surface || surface == shown) return;
          shown = surface;
          send_surface(canopy, surface);
        }
        propertiesChangedCallback:^(__unused id properties) {}];
    } @catch (NSException *e) {
      fail("CoreSimulator refused the screen callbacks");
    }

    xpc_connection_t digitizer = guest_connection(device, kDigitizer);
    if (!digitizer || !alive(digitizer, kDigitizer)) say("warn Touch and keys are not reaching this simulator");
    else say("input");

    // Gestures are timed here: keep App Nap from stretching them while Canopy is in the
    // background. The helper runs only while a pane shows the simulator or an agent drives it.
    id activity = [[NSProcessInfo processInfo] beginActivityWithOptions:NSActivityUserInitiated | NSActivityLatencyCritical reason:@"Live simulator pane"];
    (void)activity;

    // stdin closes when Canopy stops the helper or quits: leave cleanly then.
    char *line = NULL;
    size_t capacity = 0;
    while (getline(&line, &capacity, stdin) > 0) {
      if (digitizer) handle(line, digitizer);
    }
    @try {
      [screen unregisterScreenCallbacksWithUUID:token];
    } @catch (NSException *e) {
    }
    exit(0);
  }
}

#pragma mark - Host

// A receive right under a fresh bootstrap name the helper looks up; MACH_PORT_NULL on failure.
mach_port_t canopy_sim_listen(const char *name) {
  mach_port_t port = MACH_PORT_NULL;
  return bootstrap_check_in(bootstrap_port, name, &port) == KERN_SUCCESS ? port : MACH_PORT_NULL;
}

// Drops the receive right; launchd then forgets the name.
void canopy_sim_close(mach_port_t port) {
  mach_port_mod_refs(mach_task_self(), port, MACH_PORT_RIGHT_RECEIVE, -1);
}

// The surface the helper announced, as a +1 BGRA CVPixelBuffer for GPUI; NULL after `timeout_ms`.
CVPixelBufferRef canopy_sim_take_surface(mach_port_t port, unsigned int timeout_ms) {
  SurfaceReceive message = {0};
  if (mach_msg(&message.message.header, MACH_RCV_MSG | MACH_RCV_TIMEOUT, 0, sizeof message, port, timeout_ms, MACH_PORT_NULL) != KERN_SUCCESS) return NULL;
  if (message.message.header.msgh_id != 1 || !(message.message.header.msgh_bits & MACH_MSGH_BITS_COMPLEX)) {
    mach_msg_destroy(&message.message.header);
    return NULL;
  }
  IOSurfaceRef surface = IOSurfaceLookupFromMachPort(message.message.surface.name);
  mach_port_deallocate(mach_task_self(), message.message.surface.name);
  if (!surface) return NULL;
  CVPixelBufferRef buffer = NULL;
  CVPixelBufferCreateWithIOSurface(kCFAllocatorDefault, surface, NULL, &buffer);
  CFRelease(surface);
  if (buffer && CVPixelBufferGetPixelFormatType(buffer) != kCVPixelFormatType_32BGRA) {
    CVPixelBufferRelease(buffer);
    return NULL;
  }
  return buffer;
}

// Writes `buffer` as a PNG (sRGB, opaque). 0 on success.
int canopy_sim_save_png(CVPixelBufferRef buffer, const char *path) {
  if (CVPixelBufferLockBaseAddress(buffer, kCVPixelBufferLock_ReadOnly) != kCVReturnSuccess) return 1;
  size_t width = CVPixelBufferGetWidth(buffer), height = CVPixelBufferGetHeight(buffer), stride = CVPixelBufferGetBytesPerRow(buffer);
  CGColorSpaceRef space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
  CGContextRef bitmap = CGBitmapContextCreate(CVPixelBufferGetBaseAddress(buffer), width, height, 8, stride, space, kCGImageAlphaNoneSkipFirst | kCGBitmapByteOrder32Little);
  CGImageRef image = bitmap ? CGBitmapContextCreateImage(bitmap) : NULL;
  CVPixelBufferUnlockBaseAddress(buffer, kCVPixelBufferLock_ReadOnly);
  int result = 1;
  if (image) {
    CFURLRef url = CFURLCreateFromFileSystemRepresentation(NULL, (const UInt8 *)path, strlen(path), false);
    CGImageDestinationRef out = CGImageDestinationCreateWithURL(url, CFSTR("public.png"), 1, NULL);
    if (out) {
      CGImageDestinationAddImage(out, image, NULL);
      result = CGImageDestinationFinalize(out) ? 0 : 1;
      CFRelease(out);
    }
    CFRelease(url);
    CGImageRelease(image);
  }
  if (bitmap) CGContextRelease(bitmap);
  CGColorSpaceRelease(space);
  return result;
}

// Copies `buffer` into `out` as RGBA rows of width*4 bytes. 0 on success.
int canopy_sim_copy_rgba(CVPixelBufferRef buffer, uint8_t *out, size_t capacity) {
  size_t width = CVPixelBufferGetWidth(buffer), height = CVPixelBufferGetHeight(buffer);
  if (capacity < width * height * 4 || CVPixelBufferLockBaseAddress(buffer, kCVPixelBufferLock_ReadOnly) != kCVReturnSuccess) return 1;
  const uint8_t *base = CVPixelBufferGetBaseAddress(buffer);
  size_t stride = CVPixelBufferGetBytesPerRow(buffer);
  for (size_t row = 0; row < height; row++) {
    const uint8_t *from = base + row * stride;
    uint8_t *to = out + row * width * 4;
    for (size_t x = 0; x < width; x++) {
      to[x * 4] = from[x * 4 + 2];
      to[x * 4 + 1] = from[x * 4 + 1];
      to[x * 4 + 2] = from[x * 4];
      to[x * 4 + 3] = 255;
    }
  }
  CVPixelBufferUnlockBaseAddress(buffer, kCVPixelBufferLock_ReadOnly);
  return 0;
}

static CGPDFPageRef first_page(const char *path, CGPDFDocumentRef *document) {
  CFURLRef url = CFURLCreateFromFileSystemRepresentation(NULL, (const UInt8 *)path, strlen(path), false);
  *document = CGPDFDocumentCreateWithURL(url);
  CFRelease(url);
  return *document ? CGPDFDocumentGetPage(*document, 1) : NULL;
}

// Renders a DeviceKit PDF to a PNG at `scale`. With `mask` (a screen mask PDF), the mask's opaque
// area is cut out of the image inside the rect (x, y, w, h) in PDF points from the top left, so the
// live screen drawn below shows through with its real corner shape. 0 on success.
// `fill` (0xRRGGBB) paints the cut rect first, so screen corners that reach past the body's
// rounded outline take the pane's background instead of showing the square screen.
int canopy_sim_render_pdf(const char *pdf, const char *mask, double x, double y, double w, double h, double scale, unsigned int fill, const char *png) {
  CGPDFDocumentRef document = NULL, mask_document = NULL;
  CGPDFPageRef page = first_page(pdf, &document);
  if (!page) {
    if (document) CGPDFDocumentRelease(document);
    return 1;
  }
  CGRect box = CGPDFPageGetBoxRect(page, kCGPDFMediaBox);
  size_t width = (size_t)ceil(box.size.width * scale), height = (size_t)ceil(box.size.height * scale);
  CGColorSpaceRef space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
  CGContextRef bitmap = CGBitmapContextCreate(NULL, width, height, 8, 0, space, (CGBitmapInfo)kCGImageAlphaPremultipliedLast);
  int result = 1;
  if (bitmap) {
    CGContextScaleCTM(bitmap, scale, scale);
    if (mask && *mask) {
      CGContextSetRGBFillColor(bitmap, ((fill >> 16) & 255) / 255.0, ((fill >> 8) & 255) / 255.0, (fill & 255) / 255.0, 1);
      // 2 points wider: the scaled image and the screen meet without a seam.
      CGContextFillRect(bitmap, CGRectInset(CGRectMake(x, box.size.height - y - h, w, h), -2, -2));
    }
    CGContextTranslateCTM(bitmap, -box.origin.x, -box.origin.y);
    CGContextDrawPDFPage(bitmap, page);
    CGPDFPageRef mask_page = mask && *mask ? first_page(mask, &mask_document) : NULL;
    if (mask_page) {
      CGRect mask_box = CGPDFPageGetBoxRect(mask_page, kCGPDFMediaBox);
      CGContextSaveGState(bitmap);
      CGContextTranslateCTM(bitmap, box.origin.x, box.origin.y);
      // PDF space has its origin at the bottom left.
      CGContextTranslateCTM(bitmap, x, box.size.height - y - h);
      CGContextScaleCTM(bitmap, w / mask_box.size.width, h / mask_box.size.height);
      CGContextTranslateCTM(bitmap, -mask_box.origin.x, -mask_box.origin.y);
      CGContextSetBlendMode(bitmap, kCGBlendModeDestinationOut);
      CGContextDrawPDFPage(bitmap, mask_page);
      CGContextRestoreGState(bitmap);
    }
    CGImageRef image = CGBitmapContextCreateImage(bitmap);
    CFURLRef url = CFURLCreateFromFileSystemRepresentation(NULL, (const UInt8 *)png, strlen(png), false);
    CGImageDestinationRef out = CGImageDestinationCreateWithURL(url, CFSTR("public.png"), 1, NULL);
    if (out && image) {
      CGImageDestinationAddImage(out, image, NULL);
      result = CGImageDestinationFinalize(out) ? 0 : 1;
    }
    if (out) CFRelease(out);
    CFRelease(url);
    if (image) CGImageRelease(image);
    CGContextRelease(bitmap);
  }
  CGColorSpaceRelease(space);
  if (mask_document) CGPDFDocumentRelease(mask_document);
  CGPDFDocumentRelease(document);
  return result;
}
