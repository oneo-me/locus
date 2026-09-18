#import <AppKit/AppKit.h>
#import <WebKit/WebKit.h>
#import <QuartzCore/QuartzCore.h>

typedef bool (*OneoHitTest)(void *, double, double, bool);

@interface OneoWebViewRouter : NSView
@property(nonatomic, weak) NSView *gpuiView;
@property(nonatomic, weak) NSView *webView;
@property(nonatomic) void *context;
@property(nonatomic) OneoHitTest route;
@end

@implementation OneoWebViewRouter
- (NSView *)viewAtWindowPoint:(NSPoint)point mouseDown:(BOOL)down {
    if (self.hidden || !self.route || !self.gpuiView || !self.webView) return nil;
    NSPoint local = [self.gpuiView convertPoint:point fromView:nil];
    double y = self.gpuiView.isFlipped ? local.y : NSHeight(self.gpuiView.bounds) - local.y;
    if (!self.route(self.context, local.x, y, down)) return nil;
    NSView *parent = self.webView.superview;
    return [self.webView hitTest:[parent convertPoint:point fromView:nil]];
}
- (NSView *)hitTest:(NSPoint)point {
    if (self.hidden || !self.route || !self.gpuiView || !self.webView) return nil;
    NSEventType type = NSApp.currentEvent.type;
    BOOL down = type == NSEventTypeLeftMouseDown || type == NSEventTypeRightMouseDown ||
                type == NSEventTypeOtherMouseDown;
    return [self viewAtWindowPoint:[self.superview convertPoint:point toView:nil] mouseDown:down];
}
@end

// Mouse movement follows AppKit's first responder, independently of hitTest:.
@interface OneoWebViewPointer : NSObject
@property(nonatomic, weak) NSView *gpuiView;
@property(nonatomic, strong) NSHashTable<OneoWebViewRouter *> *routers;
@property(nonatomic, strong) id monitor;
@property(nonatomic, weak) OneoWebViewRouter *activeRouter;
@property(nonatomic, weak) NSWindow *cursorWindow;
@property(nonatomic) NSEventMask forwardedButtons;
- (NSEvent *)handleEvent:(NSEvent *)event;
- (void)windowResignedKey:(NSNotification *)notification;
- (void)restoreCursor;
@end

@implementation OneoWebViewPointer
- (void)windowResignedKey:(NSNotification *)notification {
    self.forwardedButtons = 0;
    [self restoreCursor];
}
- (void)restoreCursor {
    NSWindow *window = self.cursorWindow;
    self.cursorWindow = nil;
    self.activeRouter = nil;
    if (window) {
        [window enableCursorRects];
        [window resetCursorRects];
    }
}
- (void)forwardButtonEvent:(NSEvent *)event toWebView:(BOOL)web {
    NSEventMask button;
    SEL selector;
    BOOL down = NO;
    BOOL up = NO;
    switch (event.type) {
        case NSEventTypeLeftMouseDown:
            button = NSEventMaskLeftMouseDown; selector = @selector(mouseDown:); down = YES; break;
        case NSEventTypeRightMouseDown:
            button = NSEventMaskRightMouseDown; selector = @selector(rightMouseDown:); down = YES; break;
        case NSEventTypeOtherMouseDown:
            button = NSEventMaskOtherMouseDown; selector = @selector(otherMouseDown:); down = YES; break;
        case NSEventTypeLeftMouseUp:
            button = NSEventMaskLeftMouseDown; selector = @selector(mouseUp:); up = YES; break;
        case NSEventTypeRightMouseUp:
            button = NSEventMaskRightMouseDown; selector = @selector(rightMouseUp:); up = YES; break;
        case NSEventTypeOtherMouseUp:
            button = NSEventMaskOtherMouseDown; selector = @selector(otherMouseUp:); up = YES; break;
        case NSEventTypeLeftMouseDragged:
            button = NSEventMaskLeftMouseDown; selector = @selector(mouseDragged:); break;
        case NSEventTypeRightMouseDragged:
            button = NSEventMaskRightMouseDown; selector = @selector(rightMouseDragged:); break;
        case NSEventTypeOtherMouseDragged:
            button = NSEventMaskOtherMouseDown; selector = @selector(otherMouseDragged:); break;
        default: return;
    }
    if (down) {
        if (!web) return;
        self.forwardedButtons |= button;
    } else if (!(self.forwardedButtons & button)) {
        return;
    }
    if (up) self.forwardedButtons &= ~button;
    // Outside-click listeners live in GPUI. Balance the entire press sequence
    // there while AppKit continues delivering the original events to WebKit.
    [self.gpuiView tryToPerform:selector with:event];
}
- (NSEvent *)handleEvent:(NSEvent *)event {
    NSView *gpui = self.gpuiView;
    if (!gpui || event.window != gpui.window || !gpui.window.isKeyWindow) {
        [self restoreCursor];
        return event;
    }
    BOOL visible = NO;
    for (OneoWebViewRouter *router in self.routers) visible |= !router.hidden;
    if (!visible) {
        [self forwardButtonEvent:event toWebView:NO];
        [self restoreCursor];
        return event;
    }

    // Supplement GPUI only when AppKit will deliver the move to another responder.
    // Leave the original event intact for WebKit's native tracking-area observers.
    if (event.type == NSEventTypeMouseMoved && gpui.window.firstResponder != gpui) {
        [gpui mouseMoved:event];
    }
    OneoWebViewRouter *target = nil;
    for (OneoWebViewRouter *router in self.routers) {
        if ([router viewAtWindowPoint:event.locationInWindow mouseDown:NO]) {
            target = router;
            break;
        }
    }
    [self forwardButtonEvent:event toWebView:target != nil];
    if (target.hidden || !target.webView) target = nil;
    if (target) {
        self.activeRouter = target;
        NSWindow *window = gpui.window;
        if (window.areCursorRectsEnabled) {
            // WebKit sets its cursor asynchronously. Window-wide GPUI cursor
            // rectangles must not overwrite it while the pointer is on the page.
            [window disableCursorRects];
            self.cursorWindow = window;
            // Activation can leave a cursor rectangle selected. Clear that
            // selection while management is suspended, before WebKit updates.
            [window resetCursorRects];
        }
    } else {
        [self restoreCursor];
    }
    return event;
}
- (void)dealloc {
    [self restoreCursor];
    if (self.monitor) [NSEvent removeMonitor:self.monitor];
    [NSNotificationCenter.defaultCenter removeObserver:self];
}
@end

static OneoWebViewPointer *pointerForView(NSView *view) {
    static NSMapTable<NSView *, OneoWebViewPointer *> *pointers;
    if (!pointers) pointers = [NSMapTable weakToWeakObjectsMapTable];
    OneoWebViewPointer *pointer = [pointers objectForKey:view];
    if (!pointer) {
        pointer = [OneoWebViewPointer new];
        pointer.gpuiView = view;
        pointer.routers = [NSHashTable weakObjectsHashTable];
        __weak OneoWebViewPointer *weakPointer = pointer;
        NSEventMask events = NSEventMaskMouseMoved | NSEventMaskMouseEntered | NSEventMaskMouseExited |
            NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown | NSEventMaskOtherMouseDown |
            NSEventMaskLeftMouseUp | NSEventMaskRightMouseUp | NSEventMaskOtherMouseUp |
            NSEventMaskLeftMouseDragged | NSEventMaskRightMouseDragged | NSEventMaskOtherMouseDragged;
        pointer.monitor = [NSEvent addLocalMonitorForEventsMatchingMask:events
            handler:^NSEvent *(NSEvent *event) {
                OneoWebViewPointer *owner = weakPointer;
                return owner ? [owner handleEvent:event] : event;
            }];
        [NSNotificationCenter.defaultCenter addObserver:pointer selector:@selector(windowResignedKey:)
            name:NSWindowDidResignKeyNotification object:view.window];
        [pointers setObject:pointer forKey:view];
    }
    return pointer;
}

@interface OneoWebViewHost : NSObject
@property(nonatomic, strong) NSView *clipView;
@property(nonatomic, strong) NSView *webView;
@property(nonatomic, strong) OneoWebViewRouter *router;
@property(nonatomic, strong) OneoWebViewPointer *pointer;
@property(nonatomic, weak) NSView *gpuiView;
@property(nonatomic) BOOL visible;
@property(nonatomic) BOOL hasArea;
@end

@implementation OneoWebViewHost
- (BOOL)ownsFocus {
    NSResponder *responder = self.gpuiView.window.firstResponder;
    return [responder isKindOfClass:NSView.class] &&
        [(NSView *)responder isDescendantOf:self.webView];
}
- (void)updateVisibility {
    BOOL hidden = !self.visible || !self.hasArea;
    if (hidden && self.ownsFocus) [self.gpuiView.window makeFirstResponder:self.gpuiView];
    self.clipView.hidden = hidden;
    self.router.hidden = hidden;
    if (hidden && self.pointer.activeRouter == self.router) [self.pointer restoreCursor];
}
@end

void *oneo_webview_create(void *web, void *gpui, void *context, OneoHitTest route) {
    NSView *gpuiView = (__bridge NSView *)gpui;
    if (!gpuiView.superview || !gpuiView.window) return NULL;
    OneoWebViewHost *host = [OneoWebViewHost new];
    host.gpuiView = gpuiView;
    host.webView = (__bridge NSView *)web;
    host.clipView = [[NSView alloc] initWithFrame:NSZeroRect];
    host.clipView.wantsLayer = YES;
    host.clipView.layer.masksToBounds = YES;
    host.clipView.hidden = YES;
    [host.webView removeFromSuperview];
    host.webView.autoresizingMask = NSViewNotSizable;
    [host.clipView addSubview:host.webView];
    host.webView.hidden = NO;
    [gpuiView.superview addSubview:host.clipView positioned:NSWindowBelow relativeTo:gpuiView];
    host.router = [[OneoWebViewRouter alloc] initWithFrame:gpuiView.frame];
    host.router.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    host.router.gpuiView = gpuiView;
    host.router.webView = host.webView;
    host.router.context = context;
    host.router.route = route;
    host.router.hidden = YES;
    [gpuiView.superview addSubview:host.router positioned:NSWindowAbove relativeTo:gpuiView];
    host.pointer = pointerForView(gpuiView);
    [host.pointer.routers addObject:host.router];
    return (__bridge_retained void *)host;
}

void oneo_webview_layout(void *pointer, double x, double y, double width, double height,
                         double clipX, double clipY, double clipWidth, double clipHeight) {
    OneoWebViewHost *host = (__bridge OneoWebViewHost *)pointer;
    NSView *gpui = host.gpuiView;
    double fullY = gpui.isFlipped ? y : NSHeight(gpui.bounds) - y - height;
    double maskY = gpui.isFlipped ? clipY : NSHeight(gpui.bounds) - clipY - clipHeight;
    NSRect full = [gpui.superview convertRect:NSMakeRect(x, fullY, width, height) fromView:gpui];
    NSRect clip = [gpui.superview convertRect:NSMakeRect(clipX, maskY, clipWidth, clipHeight) fromView:gpui];
    NSRect web = NSMakeRect(full.origin.x - clip.origin.x, full.origin.y - clip.origin.y,
                            full.size.width, full.size.height);
    [CATransaction begin];
    [CATransaction setDisableActions:YES];
    if (!NSEqualRects(host.clipView.frame, clip)) host.clipView.frame = clip;
    if (!NSEqualRects(host.webView.frame, web)) host.webView.frame = web;
    if (!NSEqualRects(host.router.frame, gpui.frame)) host.router.frame = gpui.frame;
    host.hasArea = width > 0 && height > 0 && clipWidth > 0 && clipHeight > 0;
    [host updateVisibility];
    [CATransaction commit];
}

void oneo_webview_visible(void *pointer, bool visible) {
    OneoWebViewHost *host = (__bridge OneoWebViewHost *)pointer;
    host.visible = visible;
    [host updateVisibility];
}

void oneo_webview_focus(void *pointer, bool focused) {
    OneoWebViewHost *host = (__bridge OneoWebViewHost *)pointer;
    if (focused && host.visible && host.hasArea && !host.ownsFocus) {
        [host.gpuiView.window makeFirstResponder:host.webView];
    } else if (!focused && host.ownsFocus) {
        [host.gpuiView.window makeFirstResponder:host.gpuiView];
    }
}

void oneo_webview_destroy(void *pointer) {
    OneoWebViewHost *host = (__bridge_transfer OneoWebViewHost *)pointer;
    host.visible = NO;
    [host updateVisibility];
    [host.pointer.routers removeObject:host.router];
    [host.gpuiView.window invalidateCursorRectsForView:host.gpuiView];
    host.router.route = NULL;
    host.router.context = NULL;
    [host.router removeFromSuperview];
    [host.webView removeFromSuperview];
    [host.clipView removeFromSuperview];
}
