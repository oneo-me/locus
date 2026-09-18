# oneo-gpui-webview

macOS WebKit composition for GPUI Kit, using the published GPUI APIs. Requires macOS 13 or later and Xcode's command-line tools. The native WebKit surface sits below GPUI's rendering layer.

WebKit renders directly into its own system-managed layers. GPUI renders controls, menus and modal surfaces above it. There is no screenshot loop, CPU pixel readback, texture upload loop, browser runtime bundle, or patched GPUI dependency. `lb-wry` supplies WebKit creation, navigation, IPC and script configuration; this crate owns composition, clipping, visibility and input routing.

## Integration

Create a transparent or blurred GPUI window and give `Root` a transparent background. All ancestors covering the WebView area must also be transparent; paint toolbar, sidebar and other backgrounds in their own regions. An opaque ancestor will cover the page.

```rust,ignore
let mut view = WebView::new(
    WebViewBuilder::new().with_url("https://example.com"),
    window,
    cx,
)?;
view.set_visible(true);
let webview = cx.new(|_| view);
// Retain `webview` in the owner and render it as a child.
```

The constructor creates a hidden, opaque WebView. Configure navigation, new-window, download and IPC policy through `WebViewBuilder` before construction. The component does not provide filesystem or application commands to web content.

Call `set_visible(false)` when retaining the entity outside the displayed tree, and notify its GPUI owner after visibility changes. Dropping the owner hides the native view immediately. The native host removes its views and releases retained objects after frame and subscription references are released.

GPUI overlays must use `.occlude()` on their interactive surface, as standard menus, popovers and dialogs do. A transparent native router asks the current GPUI hitbox whether the page is eligible at the actual pointer position, then hands the original event to WebKit. It does not synthesize browser mouse or keyboard events. A window-local event monitor supplements GPUI mouse movement when another native responder has keyboard focus, while preserving AppKit/WebKit tracking events. While the pointer is on the page, automatic window cursor rectangles are suspended and reset to clear a selection left by window activation, so they cannot override WebKit cursors; leaving, hiding or releasing the page, or deactivating the window restores them. Inactive windows do not acquire cursor ownership. Hover does not transfer keyboard focus. Presses starting on the page are also delivered to GPUI so outside-click listeners can dismiss menus; their drag and release events remain paired even when the pointer leaves the page. Presses on GPUI follow normal AppKit delivery. GPUI focus subscriptions transfer first-responder ownership between the page and GPUI controls; WebKit retains text editing, selection, scrolling and IME.

## Rendering boundaries

This is native layer composition, not an API for obtaining WebView pixels or a texture handle. It supports rectangular layout and clipping, and non-overlapping WebViews. GPUI opacity, transforms and rounded masks do not transform the underlying native page. Place GPUI surfaces above the page to cover it; use explicit visibility to remove it. The approach avoids application-side frame copies but does not guarantee a particular frame rate or GPU-only rendering for every web page.

## Validation

```sh
cargo test -p oneo-gpui-webview
cargo run -p oneo-gpui-webview --example composition
```

The headless regression checks the production routing callback against GPUI hitboxes: overlay exclusion, overlay button activation, clip boundaries, focus, hidden content and a missing frame. A second regression compiles and runs a detached Objective-C fixture against the production pointer bridge, checking movement delivery, original-event preservation, focus retention, activation through GPUI, balanced cursor ownership and complete mouse-button sequences. The GPUI regression also checks outside-click dismissal over the page. Neither test creates an AppKit or WebKit window.

The example is for manual validation: keep the webpage animation visible beneath the translucent GPUI overlay, click both counters, toggle the overlay, scroll, select/copy/paste text, enter Chinese with an IME, resize and move between displays. After clicking the textarea, move onto the GPUI buttons without clicking and check their hover feedback; repeatedly cross between the link, text and GPUI controls, checking cursor changes and continued text input. Activate the window from another application by clicking GPUI first, then check the page cursors. Open a GPUI dropdown and click the page: the dropdown should dismiss and the webpage should receive the click. Also test application menus, dialogs and focus restoration in the host application. Actual native composition, accessibility, animation smoothness and resource use require a real macOS session; headless tests do not establish those results.
