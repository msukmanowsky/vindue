// macOS platform glue: the only custom native code in the app.
// Tauri has no framework commands for controlling *other* apps' windows,
// so this module wraps the Accessibility (AX) API and CGWindowList directly.

use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::{CFString, CFStringRef};
use std::ffi::{c_char, c_void, CStr};

use crate::Target;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

const K_AX_VALUE_CG_POINT_TYPE: i64 = 1;
const K_AX_VALUE_CG_SIZE_TYPE: i64 = 2;

const K_CF_NUMBER_S_INT64_TYPE: i32 = 4;
const K_CF_NUMBER_FLOAT64_TYPE: i32 = 13;
const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

const K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 1 << 0;
const K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS: u32 = 1 << 4;
const K_CG_NULL_WINDOW_ID: u32 = 0;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
    fn AXUIElementCopyAttributeValue(
        element: CFTypeRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementSetAttributeValue(
        element: CFTypeRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> i32;
    fn AXValueCreate(value_type: i64, value_ptr: *const c_void) -> CFTypeRef;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CFTypeRef;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(array: CFTypeRef) -> i64;
    fn CFArrayGetValueAtIndex(array: CFTypeRef, idx: i64) -> *const c_void;
    fn CFDictionaryGetValue(dict: *const c_void, key: *const c_void) -> *const c_void;
    fn CFNumberGetValue(number: *const c_void, the_type: i32, value_ptr: *mut c_void) -> bool;
    fn CFStringGetCString(
        string: *const c_void,
        buffer: *mut c_char,
        buffer_size: i64,
        encoding: u32,
    ) -> bool;
}

/// Check (and optionally prompt for) Accessibility permission.
pub fn is_trusted(prompt: bool) -> bool {
    unsafe {
        if !prompt {
            return AXIsProcessTrusted();
        }
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let value = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef())
    }
}

/// kAXErrorAPIDisabled (-25211): the Accessibility API is disabled for *this*
/// process — Vindue itself lost or never had the grant. This is NOT the target
/// app's fault; distinguishing it stops us blaming the target app. Rebuilds of
/// an ad-hoc-signed binary mint a new code-signing identity and silently
/// invalidate the grant, so this can surface mid-session.
pub const AX_API_DISABLED: i32 = -25211;

/// Whether an AXError code means "Vindue has no Accessibility access".
fn is_access_disabled(err: i32) -> bool {
    err == AX_API_DISABLED
}

/// Error string for the "we lost the grant" case — points at the banner fix.
pub fn access_denied_message() -> String {
    "Vindue doesn't have Accessibility access (AXError -25211, kAXErrorAPIDisabled). \
     The grant is missing or went stale — rebuilds invalidate it. Remove the old entry \
     in System Settings → Privacy & Security → Accessibility and re-add Vindue."
        .to_string()
}

/// Human-readable failure for reading a target's focused window. Pure (no AX
/// calls) so the mapping is unit-testable; -25211 is handled separately by
/// `access_denied_message` (that one is about *our* grant, not the target).
///
/// -25205 (kAXErrorAttributeUnsupported) / -25208 (kAXErrorNotImplemented)
/// mean the target process has no AX window interface at all — typically
/// system UI. Real-world case (macOS 27, hit live): the green-button tiling
/// overlay is owned by WindowManager
/// (/System/Library/CoreServices/WindowManager.app, pid 1013 in the field
/// report); before `is_regular_app` filtering existed it could be captured as
/// the target, and applying then failed with -25205 plus a message that
/// mis-blamed a normal app.
fn focused_window_error(err: i32, pid: i32) -> String {
    match err {
        -25205 | -25208 => format!(
            "Target pid {pid} doesn't expose an Accessibility window interface (AXError {err}) — \
             system UI (e.g. the macOS tiling overlay) or an app without real windows. \
             Focus a normal app window and try again."
        ),
        -25212 => {
            format!("App pid {pid} has no focused window right now — focus it and try again.")
        }
        _ => format!(
            "Could not read focused window of pid {pid} (AXError {err}). The app may not expose Accessibility windows."
        ),
    }
}

/// Human-friendly display names via NSScreen.localizedName, keyed by
/// CGDisplayModelNumber — the same number tao embeds in its "Monitor #<n>"
/// placeholder names (tao's Monitor::name formats model_number(), NOT the
/// display id). Must run on the main thread (AppKit requirement).
pub fn monitor_names_main() -> Vec<(u32, String)> {
    use core_graphics::display::CGDisplay;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::NSScreen;
    use objc2_foundation::{MainThreadMarker, NSNumber, NSString};

    let Some(mtm) = MainThreadMarker::new() else {
        log::warn!("monitor_names_main: not on the main thread");
        return Vec::new();
    };
    let mut out = Vec::new();
    let screens = NSScreen::screens(mtm);
    for screen in screens.iter() {
        let name = screen.localizedName().to_string();
        let desc = screen.deviceDescription();
        let key = NSString::from_str("NSScreenNumber");
        let Some(value) = desc.objectForKey(&key) else {
            log::warn!("monitor_names_main: no NSScreenNumber for \"{name}\"");
            continue;
        };
        let number: &NSNumber = unsafe { &*(&*value as *const AnyObject as *const NSNumber) };
        let display_id = number.unsignedIntValue();
        let model = CGDisplay::new(display_id).model_number();
        out.push((model, name));
    }
    out
}

/// Whether `pid` is a regular (user-facing) app — Dock presence, real
/// windows. System agents return false: WindowManager (which draws the
/// macOS green-button tiling overlay and CAN put layer-0 windows on
/// screen — hit live on macOS 27: it got captured as the target and apply
/// failed with AXError -25205), Dock, ControlCenter, Spotlight, etc.
/// Unknown/exited pids also return false — a dead process is never a
/// valid target either.
///
/// Thread-safety: creating an NSRunningApplication and reading
/// activationPolicy is a snapshot read, safe off the main thread (only
/// KVO *observation* of its properties requires main) — the 250ms target
/// watcher relies on that.
fn is_regular_app(pid: i32) -> bool {
    use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication};
    NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        .map(|app| app.activationPolicy() == NSApplicationActivationPolicy::Regular)
        .unwrap_or(false)
}

/// Find the frontmost regular window's owning app via CGWindowList.
/// Windows are returned front-to-back; the first layer-0 window that isn't
/// ours is the window the user last interacted with. Bounds come from
/// kCGWindowBounds (points, global screen space) — no AX permission needed,
/// which lets the panel place itself and draw the highlight pre-grant.
pub fn frontmost_target() -> Option<Target> {
    unsafe {
        let list = CGWindowListCopyWindowInfo(
            K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS,
            K_CG_NULL_WINDOW_ID,
        );
        if list.is_null() {
            return None;
        }
        let result = (|| {
            let count = CFArrayGetCount(list);
            let our_pid = std::process::id() as i64;
            for i in 0..count {
                let dict = CFArrayGetValueAtIndex(list, i);
                if dict.is_null() {
                    continue;
                }
                let layer = match dict_i64(dict, "kCGWindowLayer") {
                    Some(l) => l,
                    None => continue,
                };
                if layer != 0 {
                    continue;
                }
                let pid = match dict_i64(dict, "kCGWindowOwnerPID") {
                    Some(p) if p > 0 && p != our_pid => p,
                    _ => continue,
                };
                // Look past system-agent windows (see is_regular_app): the
                // macOS tiling overlay, Dock chrome, etc. must never become
                // the target — keep scanning; the next layer-0 window is the
                // user's actual frontmost app. Side effect: an overlay popping
                // up while the panels are open no longer falsely trips the
                // watcher's click-off dismissal (it uses this same scan).
                if !is_regular_app(pid as i32) {
                    continue;
                }
                let app_name = dict_string(dict, "kCGWindowOwnerName").unwrap_or_default();
                let wid = dict_i64(dict, "kCGWindowNumber").unwrap_or(0);
                let (x, y, w, h) =
                    dict_rect(dict, "kCGWindowBounds").unwrap_or((0.0, 0.0, 0.0, 0.0));
                return Some(Target {
                    pid: pid as i32,
                    app_name,
                    wid,
                    x,
                    y,
                    w,
                    h,
                });
            }
            None
        })();
        CFRelease(list);
        result
    }
}

/// Returns (app element, focused window element). Caller must CFRelease both.
unsafe fn focused_window_element(pid: i32) -> Result<(CFTypeRef, CFTypeRef), String> {
    let app = AXUIElementCreateApplication(pid);
    if app.is_null() {
        return Err(format!("AXUIElementCreateApplication failed for pid {pid}"));
    }
    let attr = CFString::new("AXFocusedWindow");
    let mut window: CFTypeRef = std::ptr::null();
    let err = AXUIElementCopyAttributeValue(app, attr.as_concrete_TypeRef(), &mut window);
    if err != 0 || window.is_null() {
        CFRelease(app);
        if is_access_disabled(err) {
            return Err(access_denied_message());
        }
        // err == 0 with a null window means "no focused window right now".
        return Err(focused_window_error(
            if err == 0 { -25212 } else { err },
            pid,
        ));
    }
    Ok((app, window))
}

/// Resize/move the focused window of the given app to the rect (in points).
pub fn set_window_bounds(pid: i32, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    unsafe {
        let (app, window) = focused_window_element(pid)?;

        let position = CGPoint { x, y };
        let size = CGSize { width, height };
        let position_value = AXValueCreate(
            K_AX_VALUE_CG_POINT_TYPE,
            &position as *const CGPoint as *const c_void,
        );
        let size_value = AXValueCreate(
            K_AX_VALUE_CG_SIZE_TYPE,
            &size as *const CGSize as *const c_void,
        );
        let position_attr = CFString::new("AXPosition");
        let size_attr = CFString::new("AXSize");

        let e1 = AXUIElementSetAttributeValue(
            window,
            position_attr.as_concrete_TypeRef(),
            position_value,
        );
        let e2 = AXUIElementSetAttributeValue(window, size_attr.as_concrete_TypeRef(), size_value);

        CFRelease(position_value);
        CFRelease(size_value);
        CFRelease(window);
        CFRelease(app);

        if e1 != 0 || e2 != 0 {
            if is_access_disabled(e1) || is_access_disabled(e2) {
                return Err(access_denied_message());
            }
            return Err(format!(
                "AXError setting bounds: position={e1}, size={e2} (window may refuse resizing)"
            ));
        }
        Ok(())
    }
}

unsafe fn dict_i64(dict: *const c_void, key: &str) -> Option<i64> {
    let key = CFString::new(key);
    let value = CFDictionaryGetValue(dict, key.as_concrete_TypeRef() as *const c_void);
    if value.is_null() {
        return None;
    }
    let mut out: i64 = 0;
    if CFNumberGetValue(
        value,
        K_CF_NUMBER_S_INT64_TYPE,
        &mut out as *mut i64 as *mut c_void,
    ) {
        Some(out)
    } else {
        None
    }
}

unsafe fn dict_f64(dict: *const c_void, key: &str) -> Option<f64> {
    let key = CFString::new(key);
    let value = CFDictionaryGetValue(dict, key.as_concrete_TypeRef() as *const c_void);
    if value.is_null() {
        return None;
    }
    let mut out: f64 = 0.0;
    if CFNumberGetValue(
        value,
        K_CF_NUMBER_FLOAT64_TYPE,
        &mut out as *mut f64 as *mut c_void,
    ) {
        Some(out)
    } else {
        None
    }
}

/// Read a CGRect-valued dictionary entry (e.g. kCGWindowBounds: {X, Y, Width, Height}).
unsafe fn dict_rect(dict: *const c_void, key: &str) -> Option<(f64, f64, f64, f64)> {
    let key = CFString::new(key);
    let sub = CFDictionaryGetValue(dict, key.as_concrete_TypeRef() as *const c_void);
    if sub.is_null() {
        return None;
    }
    let x = dict_f64(sub, "X")?;
    let y = dict_f64(sub, "Y")?;
    let w = dict_f64(sub, "Width")?;
    let h = dict_f64(sub, "Height")?;
    Some((x, y, w, h))
}

unsafe fn dict_string(dict: *const c_void, key: &str) -> Option<String> {
    let key = CFString::new(key);
    let value = CFDictionaryGetValue(dict, key.as_concrete_TypeRef() as *const c_void);
    if value.is_null() {
        return None;
    }
    let mut buf = [0 as c_char; 512];
    if CFStringGetCString(
        value,
        buf.as_mut_ptr(),
        buf.len() as i64,
        K_CF_STRING_ENCODING_UTF8,
    ) {
        Some(CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned())
    } else {
        None
    }
}

/// 64×64 PNG icon of a running app (NSRunningApplication.icon rendered into a
/// bitmap rep), or None if the pid is gone / has no icon. Main thread only —
/// the command dispatches us there.
pub fn app_icon_main(pid: i32) -> Option<Vec<u8>> {
    use objc2::AnyThread;
    use objc2_app_kit::{
        NSBitmapImageFileType, NSBitmapImageRep, NSCalibratedRGBColorSpace, NSGraphicsContext,
        NSRunningApplication,
    };
    use objc2_foundation::{MainThreadMarker, NSDictionary, NSPoint, NSRect, NSSize};

    MainThreadMarker::new()?;
    let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)?;
    let icon = app.icon()?;

    const SIZE: f64 = 64.0;
    // 64×64, 8 bits/sample, RGBA (4 samples), alpha, non-planar, calibrated
    // RGB; bytesPerRow/bitsPerPixel = 0 lets AppKit compute them.
    let rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            std::ptr::null_mut(),
            64,
            64,
            8,
            4,
            true,
            false,
            NSCalibratedRGBColorSpace,
            0,
            0,
        )
    }?;
    let ctx = NSGraphicsContext::graphicsContextWithBitmapImageRep(&rep)?;
    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(&ctx));
    icon.drawInRect(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(SIZE, SIZE)));
    NSGraphicsContext::setCurrentContext(None);
    NSGraphicsContext::restoreGraphicsState_class();

    let props = NSDictionary::new();
    let data =
        unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &props) }?;
    Some(data.to_vec())
}

/// Activate the app so panels actually receive key events. tao's
/// show_application() only calls NSApplication.unhide — without activation an
/// Accessory app's windows are visible but never key, and Esc/shortcut keys
/// would need a click first. The modern activate() is cooperative (macOS 14+)
/// and can be refused while another app is frontmost — hotkey activation
/// needs the forced variant. Main thread only.
pub fn activate_app() {
    use objc2_app_kit::NSApplication;
    use objc2_foundation::MainThreadMarker;
    let Some(mtm) = MainThreadMarker::new() else {
        log::warn!("activate_app: not on the main thread");
        return;
    };
    #[allow(deprecated)]
    NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
}

/// Make the window's WKWebView the first responder so it receives key
/// events. Neither window-key alone nor wry's container view (the
/// contentView, class WryWebViewParent) routes keys to web content — the
/// actual WKWebView (which forwards to its WKContentView) must hold first
/// responder. Takes the raw NSWindow pointer from WebviewWindow::ns_window().
pub fn focus_webview(ns_window: *mut std::ffi::c_void) {
    use objc2_app_kit::{NSApplication, NSResponder, NSWindow};
    use objc2_foundation::MainThreadMarker;
    if ns_window.is_null() {
        return;
    }
    let win: &NSWindow = unsafe { &*(ns_window as *const NSWindow) };
    if let Some(content) = win.contentView() {
        let web = find_web_view(&content).unwrap_or(content);
        let responder: &NSResponder = &web;
        win.makeFirstResponder(Some(responder));
    }
    let first = win
        .firstResponder()
        .map(|r| r.class().name().to_string_lossy().into_owned())
        .unwrap_or_default();
    let active = MainThreadMarker::new()
        .map(|mtm| NSApplication::sharedApplication(mtm).isActive())
        .unwrap_or(false);
    log::debug!(
        "focus state: keyWindow={} appActive={} firstResponder={first}",
        win.isKeyWindow(),
        active
    );
}

/// Depth-first search for the webview inside a view hierarchy — wry nests it
/// under container views. Primary check is isKindOfClass(WKWebView), which
/// sees through the KVO-renamed subclass wry creates
/// (NSKVONotifying_…WryWebView…); the WKContentView name check is a
/// fallback/diagnostic only (internal WebKit class). The WKWebView class is
/// looked up by name (WebKit lives in its own framework/crate).
fn find_web_view(
    view: &objc2_app_kit::NSView,
) -> Option<objc2::rc::Retained<objc2_app_kit::NSView>> {
    use objc2::runtime::{AnyClass, NSObjectProtocol};
    let wk_class = AnyClass::get(c"WKWebView");
    for sub in view.subviews().iter() {
        if let Some(cls) = wk_class {
            if sub.isKindOfClass(cls) {
                return Some(sub);
            }
        }
        let name = sub.class().name().to_string_lossy();
        if name.contains("WKContentView") {
            return Some(sub);
        }
        if let Some(found) = find_web_view(&sub) {
            return Some(found);
        }
    }
    None
}

/// (pid, localized name) of every regular (dock-visible) running app except
/// ourselves, sorted by name — the header app-picker's data source.
/// Main thread only (NSWorkspace).
pub fn running_apps_main() -> Vec<(i32, String)> {
    use objc2_app_kit::{NSApplicationActivationPolicy, NSWorkspace};
    use objc2_foundation::MainThreadMarker;

    let Some(_mtm) = MainThreadMarker::new() else {
        log::warn!("running_apps_main: not on the main thread");
        return Vec::new();
    };
    let mut out = Vec::new();
    let our_pid = std::process::id() as i32;
    for app in NSWorkspace::sharedWorkspace().runningApplications().iter() {
        if app.activationPolicy() != NSApplicationActivationPolicy::Regular {
            continue;
        }
        let pid = app.processIdentifier();
        if pid == our_pid {
            continue;
        }
        let Some(name) = app.localizedName() else {
            continue;
        };
        out.push((pid, name.to_string()));
    }
    out.sort_by_key(|x| x.1.to_lowercase());
    out
}

/// Frontmost on-screen window of a specific app (app-picker retargeting).
/// Same CGWindowList scan as frontmost_target, filtered by pid.
pub fn target_for_pid(wanted: i32) -> Option<Target> {
    // Same guard as frontmost_target: never target system agents (they have
    // no AX window interface — apply would fail with AXError -25205). Also
    // covers REST/MCP explicit-pid targeting.
    if !is_regular_app(wanted) {
        return None;
    }
    unsafe {
        let list = CGWindowListCopyWindowInfo(
            K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS,
            K_CG_NULL_WINDOW_ID,
        );
        if list.is_null() {
            return None;
        }
        let result = (|| {
            let count = CFArrayGetCount(list);
            for i in 0..count {
                let dict = CFArrayGetValueAtIndex(list, i);
                if dict.is_null() {
                    continue;
                }
                if dict_i64(dict, "kCGWindowLayer") != Some(0) {
                    continue;
                }
                let pid = match dict_i64(dict, "kCGWindowOwnerPID") {
                    Some(p) if p == wanted as i64 => p,
                    _ => continue,
                };
                let app_name = dict_string(dict, "kCGWindowOwnerName").unwrap_or_default();
                let wid = dict_i64(dict, "kCGWindowNumber").unwrap_or(0);
                let (x, y, w, h) =
                    dict_rect(dict, "kCGWindowBounds").unwrap_or((0.0, 0.0, 0.0, 0.0));
                return Some(Target {
                    pid: pid as i32,
                    app_name,
                    wid,
                    x,
                    y,
                    w,
                    h,
                });
            }
            None
        })();
        CFRelease(list);
        result
    }
}

/// Current bounds (points, global screen space) of a window by CGWindowNumber
/// — lets the watcher follow the target's moves/resizes without AX.
pub fn window_bounds(wid: i64) -> Option<(f64, f64, f64, f64)> {
    unsafe {
        let list = CGWindowListCopyWindowInfo(
            K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY | K_CG_WINDOW_LIST_EXCLUDE_DESKTOP_ELEMENTS,
            K_CG_NULL_WINDOW_ID,
        );
        if list.is_null() {
            return None;
        }
        let result = (|| {
            let count = CFArrayGetCount(list);
            for i in 0..count {
                let dict = CFArrayGetValueAtIndex(list, i);
                if dict.is_null() {
                    continue;
                }
                if dict_i64(dict, "kCGWindowNumber") == Some(wid) {
                    return dict_rect(dict, "kCGWindowBounds");
                }
            }
            None
        })();
        CFRelease(list);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::focused_window_error;

    #[test]
    fn focused_window_error_maps_codes() {
        // System UI / no AX window interface — the macOS tiling overlay
        // (WindowManager) case that motivated the mapping.
        let m = focused_window_error(-25205, 1013);
        assert!(m.contains("doesn't expose an Accessibility window interface"));
        assert!(m.contains("1013"));
        assert!(m.contains("-25205"));
        assert!(focused_window_error(-25208, 7).contains("doesn't expose"));
        // App exists but nothing is focused.
        assert!(focused_window_error(-25212, 7).contains("no focused window"));
        // Everything else keeps the generic target-app hint.
        assert!(focused_window_error(-25204, 7).contains("may not expose Accessibility windows"));
    }
}
