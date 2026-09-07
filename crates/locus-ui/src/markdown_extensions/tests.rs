use super::*;
use gpui_kit::{Context, Modifiers, Render, TestAppContext, point};
use std::cell::RefCell;

struct CanvasPlaceholder {
    opened: Rc<RefCell<Vec<(String, String)>>>,
    header: String,
}

impl Render for CanvasPlaceholder {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let opened = self.opened.clone();
        let tree = LibraryTree::load(None);
        let extensions = LocusMarkdownExtensions::new(&[], &tree, Language::English)
            .on_open_canvas(move |title, source, _, _| {
                opened
                    .borrow_mut()
                    .push((title.to_owned(), source.to_owned()));
            });
        div().id("canvas-test").w(px(320.)).tab_group().child(
            extensions
                .render_code_block(
                    "canvas",
                    &self.header,
                    0,
                    "<script>run()</script>",
                    Theme::global(cx),
                )
                .unwrap(),
        )
    }
}

#[gpui_kit::test]
fn canvas_placeholder_opens_the_original_source_once_per_activation(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::init);
    let opened = Rc::new(RefCell::new(Vec::new()));
    let (_, cx) = cx.add_window_view(|_, _| CanvasPlaceholder {
        opened: opened.clone(),
        header: "暖暮 | 色彩、字体与交互组件".to_owned(),
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_click(point(px(10.), px(10.)), Modifiers::default());
    assert_eq!(
        opened.borrow().as_slice(),
        &[(
            "暖暮 | 色彩、字体与交互组件".to_owned(),
            "<script>run()</script>".to_owned()
        )]
    );
    cx.update(|window, cx| {
        window.focus_next(cx);
        assert!(window.focused(cx).is_some());
        window.draw(cx).clear(cx);
    });
    let keystroke = gpui_kit::Keystroke::parse("enter").unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    assert_eq!(opened.borrow().len(), 2);
}

#[gpui_kit::test]
fn canvas_description_wraps_inside_the_bordered_button(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::init);
    let (_, cx) = cx.add_window_view(|_, _| CanvasPlaceholder {
        opened: Rc::new(RefCell::new(Vec::new())),
        header: format!("暖暮 | {}", "色彩、字体和交互组件的完整演示。".repeat(6)),
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let button = cx.debug_bounds("canvas-placeholder").unwrap();
    let description = cx.debug_bounds("canvas-description").unwrap();
    assert_eq!(button.size.width, px(320.));
    assert!(description.size.height > px(32.));
    assert!(description.bottom() < button.bottom());
    assert!(description.right() < button.right());
}

#[gpui_kit::test]
fn incomplete_canvas_metadata_cannot_open_a_preview(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::init);
    let opened = Rc::new(RefCell::new(Vec::new()));
    let (_, cx) = cx.add_window_view(|_, _| CanvasPlaceholder {
        opened: opened.clone(),
        header: "暖暮".to_owned(),
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_click(point(px(20.), px(20.)), Modifiers::default());
    assert!(opened.borrow().is_empty());
}
