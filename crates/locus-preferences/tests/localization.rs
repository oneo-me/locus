use locus_preferences::localization::Language;

#[test]
fn language_changes_reuse_the_initialized_component_catalog() {
    locus_preferences::localization::init();
    for language in [Language::English, Language::Chinese, Language::English] {
        language.activate();
        assert_eq!(&*rust_i18n::locale(), language.locale());
        assert_eq!(&*gpui_component::locale(), language.locale());
    }
}

#[test]
fn consumers_resolve_the_shared_catalog_and_named_arguments() {
    assert_eq!(
        Language::English.text_with("workspace.terminal.tab", &[("id", "3".to_owned())]),
        "Terminal 3"
    );
    assert_eq!(
        Language::Chinese.text_with("workspace.terminal.close_tab", &[("id", "3".to_owned())]),
        "关闭终端 3"
    );
    assert_eq!(
        Language::English.text_with(
            "markdown.gallery.image_unavailable",
            &[("url", "图片 %{id}.png".to_owned())],
        ),
        "Image attachment is missing or unsupported: 图片 %{id}.png"
    );
    let description = Language::Chinese.text_with(
        "workspace.graph.accessibility",
        &[("edges", "7".to_owned()), ("nodes", "4".to_owned())],
    );
    assert!(description.starts_with("知识图谱包含 4 个节点和 7 条关系。"));
    assert!(!description.contains("%{"));
}
