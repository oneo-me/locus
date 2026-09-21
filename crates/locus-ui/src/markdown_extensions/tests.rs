use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use gpui_kit::component::{
    Root,
    text::{SelectionFormat, TextView, TextViewState},
};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AppContext, Context, Entity, IntoElement, Render, TestAppContext, VisualTestContext, Window,
    div, prelude::*, px,
};
use oneo_gpui::Language;

use super::note_markdown_extensions;

struct NoteView {
    state: Entity<TextViewState>,
    opened: Arc<Mutex<Vec<String>>>,
}

impl Render for NoteView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let opened = self.opened.clone();
        div().w(px(640.)).child(
            TextView::new(&self.state)
                .selectable(true)
                .selection_format(SelectionFormat::Plain)
                .scrollable(false)
                .markdown_extensions(note_markdown_extensions(
                    Path::new("/notes/example"),
                    &[],
                    Language::Chinese,
                    move |url, _, _| opened.lock().unwrap().push(url.to_owned()),
                    |_, _| div().into_any_element(),
                )),
        )
    }
}

#[gpui_kit::test]
fn text_view_copies_body_and_literal_code_without_attachment_declarations(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let code = "    let 中文 = \"**literal** <tag>\";\n".repeat(30);
    let source = format!(
        "# 标题\n\n正文 **粗体** `inline`\n\n```rust\n{code}```\n\n```gallery\n![图库](assets/sea.png)\n```\n\n```attachments\n[report.pdf](assets/report.pdf)\n```\n\n| 甲 | 乙 |\n| --- | --- |\n| 一 | 二 |\n\n- [x] 完成\n\n结尾"
    );
    let state = cx.new(|cx| TextViewState::markdown(&source, cx));
    let retained = state.clone();
    let (_, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| NoteView {
            state,
            opened: Default::default(),
        });
        Root::new(view, window, cx)
    });
    let cx: &mut VisualTestContext = cx;
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let focus = retained.read(cx).focus_handle().clone();
        focus.focus(window, cx);
        window.press("secondary-a", cx);
    });
    cx.run_until_parked();
    let selected = retained.read_with(cx, |state, _| state.selected_text());
    assert!(selected.contains("标题"), "{selected}");
    assert!(selected.contains("正文 粗体 inline"), "{selected}");
    assert!(selected.contains(code.trim_end()), "{selected}");
    assert!(selected.contains("甲") && selected.contains("完成") && selected.contains("结尾"));
    assert!(
        !selected.contains("assets/") && !selected.contains("report.pdf"),
        "{selected}"
    );
    cx.update(|window, cx| window.press("secondary-c", cx));
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().as_deref(),
            Some(selected.trim_end())
        )
    });

    retained.update(cx, |state, cx| state.set_text("替换后的正文", cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    retained.update(cx, |state, cx| state.select_all(cx));
    assert_eq!(
        retained
            .read_with(cx, |state, _| state.selected_text())
            .trim(),
        "替换后的正文"
    );
}

#[gpui_kit::test]
fn inline_links_keep_labels_and_activate_by_pointer_and_keyboard(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let source = "[内部笔记](locus://collections) 与 [网站][site]\n\n[site]: https://example.com";
    let state = cx.new(|cx| TextViewState::markdown(source, cx));
    let retained = state.clone();
    let opened = Arc::new(Mutex::new(Vec::new()));
    let calls = opened.clone();
    let (_, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| NoteView { state, opened });
        Root::new(view, window, cx)
    });
    let cx: &mut VisualTestContext = cx;
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.click("note-link-0", cx);
    });
    cx.run_until_parked();
    assert_eq!(calls.lock().unwrap().as_slice(), ["locus://collections"]);
    cx.update(|window, cx| {
        let focus = retained.read(cx).focus_handle().clone();
        focus.focus(window, cx);
        window.focus_next(cx);
        window.render_frame(cx);
        assert_eq!(window.find("note-link-0").focused(), Some(true));
    });
    let keystroke = gpui_kit::Keystroke::parse("enter").unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    cx.run_until_parked();
    assert_eq!(calls.lock().unwrap().len(), 2);
    let second = format!("note-link-{}", source.find("[网站]").unwrap());
    cx.update(|window, cx| window.click(gpui_kit::SharedString::from(second.clone()), cx));
    cx.run_until_parked();
    assert_eq!(calls.lock().unwrap().last().unwrap(), "https://example.com");
    retained.update(cx, |state, cx| state.select_all(cx));
    let selected = retained.read_with(cx, |state, _| state.selected_text());
    assert_eq!(selected.trim(), "内部笔记 与 网站");
}
