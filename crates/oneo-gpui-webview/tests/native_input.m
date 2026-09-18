#import "../src/host.m"
#include <assert.h>

@interface TestWindow : NSObject
@property(nonatomic, strong) NSResponder *firstResponder;
@property(nonatomic) BOOL areCursorRectsEnabled;
@property(nonatomic) BOOL isKeyWindow;
@property(nonatomic) BOOL selectedCursorRect;
@property(nonatomic) NSUInteger disables;
@property(nonatomic) NSUInteger enables;
@property(nonatomic) NSUInteger resets;
@end
@implementation TestWindow
- (void)disableCursorRects { self.areCursorRectsEnabled = NO; self.disables++; }
- (void)enableCursorRects { self.areCursorRectsEnabled = YES; self.enables++; }
- (void)resetCursorRects { self.resets++; self.selectedCursorRect = NO; }
- (BOOL)makeFirstResponder:(NSResponder *)responder {
    self.firstResponder = responder;
    return YES;
}
@end

@interface TestView : NSView
@property(nonatomic, strong) TestWindow *testWindow;
@property(nonatomic) NSUInteger moves;
@property(nonatomic) NSUInteger downs;
@property(nonatomic) NSUInteger ups;
@property(nonatomic) NSUInteger drags;
@end
@implementation TestView
- (NSWindow *)window { return (NSWindow *)self.testWindow; }
- (void)mouseMoved:(NSEvent *)event { self.moves++; }
- (void)mouseDown:(NSEvent *)event { self.downs++; }
- (void)mouseUp:(NSEvent *)event { self.ups++; }
- (void)mouseDragged:(NSEvent *)event { self.drags++; }
- (void)rightMouseDown:(NSEvent *)event { self.downs++; }
- (void)rightMouseUp:(NSEvent *)event { self.ups++; }
- (void)rightMouseDragged:(NSEvent *)event { self.drags++; }
- (void)otherMouseDown:(NSEvent *)event { self.downs++; }
- (void)otherMouseUp:(NSEvent *)event { self.ups++; }
- (void)otherMouseDragged:(NSEvent *)event { self.drags++; }
@end

@interface TestRouter : OneoWebViewRouter
@property(nonatomic) BOOL eligible;
@end
@implementation TestRouter
- (NSView *)viewAtWindowPoint:(NSPoint)point mouseDown:(BOOL)down {
    assert(!down);
    return !self.hidden && self.eligible ? self.webView : nil;
}
@end

@interface TestEvent : NSObject
@property(nonatomic, strong) NSWindow *window;
@property(nonatomic) NSEventType type;
@property(nonatomic) NSPoint locationInWindow;
@end
@implementation TestEvent
@end

int main(void) {
    @autoreleasepool {
        TestWindow *window = [TestWindow new];
        window.isKeyWindow = YES;
        window.areCursorRectsEnabled = YES;
        TestView *gpui = [TestView new];
        gpui.testWindow = window;
        NSView *web = [NSView new];
        TestRouter *router = [TestRouter new];
        router.webView = web;
        router.eligible = YES;
        OneoWebViewPointer *pointer = [OneoWebViewPointer new];
        pointer.gpuiView = gpui;
        pointer.routers = [NSHashTable weakObjectsHashTable];
        [pointer.routers addObject:router];
        TestEvent *event = [TestEvent new];
        event.window = (NSWindow *)window;
        event.type = NSEventTypeMouseMoved;

        // A clicked page keeps keyboard focus; movement still updates GPUI once.
        window.firstResponder = web;
        assert([pointer handleEvent:(NSEvent *)event] == (NSEvent *)event);
        assert(gpui.moves == 1 && window.firstResponder == web);
        assert(!window.areCursorRectsEnabled && window.disables == 1);
        [pointer handleEvent:(NSEvent *)event];
        assert(gpui.moves == 2 && window.disables == 1);

        // Moving onto an occluding GPUI surface restores automatic cursor updates.
        router.eligible = NO;
        assert([pointer handleEvent:(NSEvent *)event] == (NSEvent *)event);
        assert(gpui.moves == 3 && window.firstResponder == web);
        assert(window.areCursorRectsEnabled && window.enables == 1 && window.resets == 2);

        // GPUI first-responder delivery is left to AppKit, without a duplicate.
        window.firstResponder = gpui;
        [pointer handleEvent:(NSEvent *)event];
        assert(gpui.moves == 3);
        router.eligible = YES;
        [pointer handleEvent:(NSEvent *)event];
        assert(gpui.moves == 3 && window.disables == 2);

        // Enter/exit tracking events remain untouched for WebKit's own observers.
        event.type = NSEventTypeMouseExited;
        router.eligible = NO;
        assert([pointer handleEvent:(NSEvent *)event] == (NSEvent *)event);
        assert(gpui.moves == 3 && window.areCursorRectsEnabled);
        event.type = NSEventTypeMouseMoved;

        // Hidden content and events in other windows release cursor ownership.
        router.eligible = YES;
        [pointer handleEvent:(NSEvent *)event];
        router.hidden = YES;
        [pointer handleEvent:(NSEvent *)event];
        assert(window.areCursorRectsEnabled && gpui.moves == 3);
        router.hidden = NO;
        [pointer handleEvent:(NSEvent *)event];
        event.window = nil;
        [pointer handleEvent:(NSEvent *)event];
        assert(window.areCursorRectsEnabled && gpui.moves == 3);
        event.window = (NSWindow *)window;

        // Hiding the active host restores both cursor management and native focus.
        OneoWebViewHost *host = [OneoWebViewHost new];
        host.gpuiView = gpui;
        host.webView = web;
        host.router = router;
        host.pointer = pointer;
        host.clipView = [NSView new];
        host.hasArea = YES;
        host.visible = YES;
        window.firstResponder = web;
        [pointer handleEvent:(NSEvent *)event];
        host.visible = NO;
        [host updateVisibility];
        assert(window.areCursorRectsEnabled && window.firstResponder == gpui);
        host = nil;
        router.hidden = NO;

        // Window deactivation restores cursor management without another mouse event.
        [pointer handleEvent:(NSEvent *)event];
        [pointer windowResignedKey:[NSNotification notificationWithName:NSWindowDidResignKeyNotification
            object:window]];
        assert(window.areCursorRectsEnabled);

        // Reactivating through GPUI can leave its cursor rectangle selected.
        window.isKeyWindow = NO;
        NSUInteger disables = window.disables;
        [pointer handleEvent:(NSEvent *)event];
        assert(window.areCursorRectsEnabled && window.disables == disables);
        window.isKeyWindow = YES;
        window.firstResponder = gpui;
        window.selectedCursorRect = YES;
        router.eligible = NO;
        [pointer handleEvent:(NSEvent *)event];
        assert(window.selectedCursorRect && window.areCursorRectsEnabled);
        router.eligible = YES;
        [pointer handleEvent:(NSEvent *)event];
        assert(!window.selectedCursorRect && !window.areCursorRectsEnabled);
        assert(window.firstResponder == gpui);

        // AppKit can reenable rectangles during activation even while we own them.
        window.areCursorRectsEnabled = YES;
        window.selectedCursorRect = YES;
        [pointer handleEvent:(NSEvent *)event];
        assert(!window.selectedCursorRect && !window.areCursorRectsEnabled);
        [pointer restoreCursor];

        // Web clicks reach GPUI outside-click listeners, with balanced drag/up events.
        const NSEventType sequences[][3] = {
            {NSEventTypeLeftMouseDown, NSEventTypeLeftMouseDragged, NSEventTypeLeftMouseUp},
            {NSEventTypeRightMouseDown, NSEventTypeRightMouseDragged, NSEventTypeRightMouseUp},
            {NSEventTypeOtherMouseDown, NSEventTypeOtherMouseDragged, NSEventTypeOtherMouseUp},
        };
        for (NSUInteger i = 0; i < 3; i++) {
            router.eligible = YES;
            event.type = sequences[i][0];
            assert([pointer handleEvent:(NSEvent *)event] == (NSEvent *)event);
            assert(gpui.downs == i + 1);
            router.eligible = NO;
            event.type = sequences[i][1];
            [pointer handleEvent:(NSEvent *)event];
            assert(gpui.drags == i + 1);
            router.hidden = YES;
            event.type = sequences[i][2];
            assert([pointer handleEvent:(NSEvent *)event] == (NSEvent *)event);
            assert(gpui.ups == i + 1 && !pointer.forwardedButtons);
            router.hidden = NO;
            // A press starting on GPUI remains on AppKit's normal path.
            event.type = sequences[i][0];
            [pointer handleEvent:(NSEvent *)event];
            event.type = sequences[i][2];
            [pointer handleEvent:(NSEvent *)event];
            assert(gpui.downs == i + 1 && gpui.ups == i + 1);
        }
        event.type = NSEventTypeMouseMoved;
        router.eligible = YES;

        // Do not undo another owner's pre-existing cursor-management suspension.
        window.areCursorRectsEnabled = NO;
        NSUInteger enables = window.enables;
        [pointer handleEvent:(NSEvent *)event];
        [pointer restoreCursor];
        assert(window.enables == enables && !window.areCursorRectsEnabled);

        // Releasing the final host balances its suspension, even without a move.
        window.areCursorRectsEnabled = YES;
        [pointer handleEvent:(NSEvent *)event];
        pointer = nil;
        assert(window.areCursorRectsEnabled && window.enables == enables + 1);
        puts("native pointer routing: passed");
    }
    return 0;
}
