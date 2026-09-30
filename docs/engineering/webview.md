# oneo-gpui-webview

macOS WebKit composition for GPUI Kit, using the published GPUI APIs. Requires macOS 13 or later and Xcode's command-line tools. The native WebKit surface sits below GPUI's rendering layer.

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

GPUI overlays must use `.occlude()` on their interactive surface, as standard menus, popovers and dialogs do. The component coordinates pointer input and focus between GPUI and WebKit; WebKit provides native text editing, selection, scrolling and IME.

## Rendering boundaries

This is native layer composition, not an API for obtaining WebView pixels or a texture handle. It supports rectangular layout and clipping, and non-overlapping WebViews. GPUI opacity, transforms and rounded masks do not transform the underlying native page. Place GPUI surfaces above the page to cover it; use explicit visibility to remove it. The approach avoids application-side frame copies but does not guarantee a particular frame rate or GPU-only rendering for every web page.

## Example

Run the composition example from the repository root:

```sh
cargo run --manifest-path source/Cargo.toml -p oneo-gpui-webview --example composition
```

It demonstrates a webpage beneath GPUI overlays.
