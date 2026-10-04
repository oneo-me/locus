use super::*;
use gpui_kit::TestAppContext;
use gpui_kit::component::Root;
use gpui_kit::test::TestWindowExt;

#[gpui_kit::test]
fn about_navigation_disables_missing_updater_and_links_support_keyboard(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut settings = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let return_focus = cx.focus_handle();
        let view = cx.new(|cx| {
            Settings::new(
                window,
                SettingsInit {
                    preferences: Preferences::default(),
                    load_error: None,
                    mcp_server_state: McpServerState::Stopped,
                    archived_notes: Vec::new(),
                    archive_load_error: None,
                },
                return_focus,
                cx,
            )
        });
        settings = Some(view.clone());
        Root::new(view, window, cx)
    });
    let settings = settings.unwrap();
    cx.simulate_resize(gpui_kit::size(px(980.), px(820.)));
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.click("settings-about", cx);
        assert_eq!(settings.read(cx).page, SettingsPage::About);
        assert!(window.find("check-for-updates").visible());
        assert!(!settings.read(cx).updater.read(cx).is_check_enabled());
        window.click("check-for-updates", cx);
        assert_eq!(
            settings.read(cx).updater.read(cx).status(),
            &locus_updater::UpdateStatus::Unavailable
        );
    });
    let license = cx
        .debug_bounds("about-license")
        .expect("license link is rendered");
    cx.simulate_click(license.center(), gpui_kit::Modifiers::default());
    assert_eq!(
        cx.opened_url().as_deref(),
        Some("https://github.com/oneo-me/locus/blob/main/LICENSE")
    );
    for key in ["tab", "enter"] {
        let keystroke = gpui_kit::Keystroke::parse(key).unwrap();
        cx.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    }
    assert_eq!(
        cx.opened_url().as_deref(),
        Some("https://github.com/oneo-me/locus")
    );
}

#[test]
fn about_status_copy_is_localized_and_does_not_confuse_cancellation_with_latest() {
    use super::about::update_description;
    use locus_updater::UpdateStatus;
    for language in [Language::English, Language::Chinese] {
        let missing = update_description(&UpdateStatus::Unavailable, language);
        assert!(!missing.is_empty());
        let available = update_description(&UpdateStatus::Available("0.2026.2".into()), language);
        assert!(available.contains("0.2026.2"));
        let error = update_description(&UpdateStatus::Failed("Offline".into()), language);
        assert!(error.contains("Offline"));
        let no_update = update_description(
            &UpdateStatus::NoUpdate("Requires newer macOS".into()),
            language,
        );
        assert_eq!(no_update, "Requires newer macOS");
    }
}
