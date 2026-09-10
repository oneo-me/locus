use super::*;
use std::fs;

fn note(root: &Path, name: &str, source: &str) -> std::path::PathBuf {
    let directory = root.join(name);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("index.md"), "---\ntitle: Canvas\n---\n").unwrap();
    fs::write(directory.join("canvas.html"), source).unwrap();
    directory
}

#[test]
fn single_html_contains_nested_templates_data_theme_and_runtime() {
    let root = tempfile::tempdir().unwrap();
    let directory = note(
        root.path(),
        "note",
        r#"<script>locus.loadCanvas('locus://parent', {answer: 42});</script>"#,
    );
    note(
        root.path(),
        "parent",
        r#"<script>locus.loadCanvas('locus://child', document.currentScript.canvas.data);</script>"#,
    );
    note(
        root.path(),
        "child",
        "<button>Nested child</button><script>window.answer = document.currentScript.canvas.data.answer;</script>",
    );
    let export = prepare_download(root.path(), &directory, true, Dependencies::default()).unwrap();
    assert_eq!(export.files.len(), 1);
    let html = String::from_utf8(export.files["canvas.html"].clone()).unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("class=\"dark\""));
    assert!(html.contains("answer: 42"));
    assert!(html.contains("Nested child"));
    assert!(html.contains("\\u003c/script>"));
    assert!(html.contains(include_str!("../../assets/tailwindcss/browser-4.3.3.js")));
    assert!(!html.contains(root.path().to_str().unwrap()));
    let destination = root.path().join("Downloads");
    let output = export.save(&destination, "需求").unwrap();
    assert_eq!(output, destination.join("需求.html"));
    assert_eq!(fs::read_to_string(output).unwrap(), html);
    assert_eq!(fs::read_dir(destination).unwrap().count(), 1);
}

#[test]
fn directory_preserves_only_referenced_attachments_and_their_dependencies() {
    let root = tempfile::tempdir().unwrap();
    let directory = note(
        root.path(),
        "note",
        r#"<link rel=stylesheet href=style.css><script src="app.js"></script><img src="photo%20one.png?size=2"><a href="report.pdf">Report</a>"#,
    );
    fs::write(
        directory.join("style.css"),
        "@import 'nested.css'; main { background: url('photo%20one.png'); }",
    )
    .unwrap();
    fs::write(directory.join("nested.css"), "p { color: red; }").unwrap();
    fs::write(
        directory.join("app.js"),
        "fetch('data.json').then(r => r.json());",
    )
    .unwrap();
    fs::write(directory.join("data.json"), "{\"value\":42}").unwrap();
    fs::write(directory.join("photo one.png"), [0, 255, 42]).unwrap();
    fs::write(directory.join("report.pdf"), b"pdf bytes").unwrap();
    fs::write(directory.join("unrelated.txt"), "private").unwrap();
    fs::write(directory.join(".secret"), "private").unwrap();
    let export = prepare_download(root.path(), &directory, false, Dependencies::default()).unwrap();
    let destination = tempfile::tempdir().unwrap();
    export.write_files(destination.path()).unwrap();
    let names: BTreeSet<_> = fs::read_dir(destination.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        names,
        [
            "canvas.html",
            "style.css",
            "nested.css",
            "app.js",
            "data.json",
            "photo one.png",
            "report.pdf"
        ]
        .map(str::to_owned)
        .into()
    );
    assert_eq!(
        fs::read(destination.path().join("photo one.png")).unwrap(),
        [0, 255, 42]
    );
    assert_eq!(
        fs::read(destination.path().join("canvas.html")).unwrap(),
        export.files["canvas.html"]
    );
}

#[test]
fn observed_dynamic_dependencies_and_template_resources_are_included() {
    let root = tempfile::tempdir().unwrap();
    let directory = note(
        root.path(),
        "note",
        "<script>locus.loadCanvas('locus://' + 'dynamic');</script>",
    );
    note(root.path(), "dynamic", "<img src='picture.png'>");
    fs::write(directory.join("picture.png"), "caller image").unwrap();
    fs::write(root.path().join("dynamic/picture.png"), "template image").unwrap();
    let dependencies = Dependencies {
        templates: ["locus://dynamic".to_owned()].into(),
        ..Default::default()
    };
    let export = prepare_download(root.path(), &directory, false, dependencies).unwrap();
    let destination = tempfile::tempdir().unwrap();
    export.write_files(destination.path()).unwrap();
    assert_eq!(
        fs::read_to_string(destination.path().join("picture.png")).unwrap(),
        "caller image"
    );
}

#[test]
fn invalid_missing_and_empty_dependencies_fail_without_writing() {
    let root = tempfile::tempdir().unwrap();
    for source in [
        " ",
        "<img src='missing.png'>",
        "<img src='../outside.png'>",
        "<script>locus.loadCanvas('locus://missing');</script>",
        "<script>locus.loadCanvas('locus://empty');</script>",
        "<a href='index.md'>Note</a>",
    ] {
        let directory = note(root.path(), "note", source);
        note(root.path(), "empty", " ");
        assert!(
            prepare_download(root.path(), &directory, false, Dependencies::default()).is_err(),
            "{source}"
        );
    }
}

#[test]
fn cycles_are_deduplicated_and_online_resources_remain_online() {
    let root = tempfile::tempdir().unwrap();
    let directory = note(
        root.path(),
        "note",
        "<img src='https://example.com/map.png'><script>locus.loadCanvas('locus://a');</script>",
    );
    note(
        root.path(),
        "a",
        "<script>locus.loadCanvas('locus://b');</script>",
    );
    note(
        root.path(),
        "b",
        "<script>locus.loadCanvas('locus://a');</script>",
    );
    let export = prepare_download(root.path(), &directory, false, Dependencies::default()).unwrap();
    assert_eq!(export.files.len(), 1);
    assert!(
        String::from_utf8(export.files["canvas.html"].clone())
            .unwrap()
            .contains("https://example.com/map.png")
    );
}

#[cfg(unix)]
#[test]
fn symlinked_resources_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let directory = note(root.path(), "note", "<img src='link.png'>");
    fs::write(root.path().join("outside.png"), "secret").unwrap();
    std::os::unix::fs::symlink(root.path().join("outside.png"), directory.join("link.png"))
        .unwrap();
    assert!(prepare_download(root.path(), &directory, false, Dependencies::default()).is_err());
}

#[test]
fn directory_export_preserves_existing_files_and_rejects_conflicts() {
    let root = tempfile::tempdir().unwrap();
    let directory = note(root.path(), "note", "<p>Ready</p><img src='picture.png'>");
    fs::write(directory.join("picture.png"), [1, 2, 3]).unwrap();
    let export = prepare_download(root.path(), &directory, false, Dependencies::default()).unwrap();
    let destination = tempfile::tempdir().unwrap();
    fs::write(destination.path().join("unrelated.txt"), "keep").unwrap();
    fs::write(destination.path().join("picture.png"), "existing").unwrap();
    assert_eq!(
        export.write_files(destination.path()).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        fs::read_to_string(destination.path().join("picture.png")).unwrap(),
        "existing"
    );
    assert!(!destination.path().join("canvas.html").exists());
    fs::remove_file(destination.path().join("picture.png")).unwrap();
    export.write_files(destination.path()).unwrap();
    assert_eq!(
        fs::read(destination.path().join("canvas.html")).unwrap(),
        export.files["canvas.html"]
    );
    assert_eq!(
        fs::read_to_string(destination.path().join("unrelated.txt")).unwrap(),
        "keep"
    );
    assert!(export.write_files(destination.path()).is_err());
    assert!(export.write_files(&root.path().join("missing")).is_err());
    assert!(export.write_files(&directory).is_err());
    assert_eq!(
        fs::read_to_string(directory.join("canvas.html")).unwrap(),
        "<p>Ready</p><img src='picture.png'>"
    );
}

#[test]
fn srcset_keeps_data_urls_intact_and_collects_local_candidates() {
    let root = tempfile::tempdir().unwrap();
    let directory = note(
        root.path(),
        "note",
        "<img srcset='data:image/png;base64,AP8q 1x, large.png 2x'>",
    );
    fs::write(directory.join("large.png"), [1, 2, 3]).unwrap();
    let export = prepare_download(root.path(), &directory, false, Dependencies::default()).unwrap();
    assert_eq!(export.files.len(), 2);
}

#[test]
fn missing_script_data_and_imported_styles_fail() {
    let root = tempfile::tempdir().unwrap();
    for source in [
        "<script>fetch('missing.json')</script>",
        "<style>@import 'missing.css';</style>",
    ] {
        let directory = note(root.path(), "note", source);
        assert!(prepare_download(root.path(), &directory, false, Dependencies::default()).is_err());
    }
}

#[test]
fn unrelated_note_links_do_not_pull_other_canvases_into_export() {
    let root = tempfile::tempdir().unwrap();
    let directory = note(root.path(), "note", "<a href='locus://other'>Other</a>");
    note(root.path(), "other", "<p>Private unrelated canvas</p>");
    let export = prepare_download(root.path(), &directory, false, Dependencies::default()).unwrap();
    assert!(
        !String::from_utf8(export.files["canvas.html"].clone())
            .unwrap()
            .contains("Private unrelated canvas")
    );
}

#[test]
fn script_url_constructors_are_not_css_dependencies() {
    let root = tempfile::tempdir().unwrap();
    let directory = note(
        root.path(),
        "note",
        r#"<script>function parse(e) { return new URL(e); } class Map { setURL(e) { this.url = e; } }</script><style>p { background: url('image.png'); }</style>"#,
    );
    fs::write(directory.join("image.png"), [1, 2, 3]).unwrap();
    let export = prepare_download(root.path(), &directory, false, Dependencies::default()).unwrap();
    assert_eq!(export.files.len(), 2);
}

#[test]
fn single_html_downloads_skip_existing_files_and_directories() {
    let root = tempfile::tempdir().unwrap();
    let source = note(root.path(), "note", "<p>Single HTML</p>");
    fs::write(source.join("unrelated.txt"), "not an export dependency").unwrap();
    let export = prepare_download(root.path(), &source, false, Dependencies::default()).unwrap();
    let parent = root.path().join("Downloads");
    let first = export.save(&parent, "旅行计划").unwrap();
    assert_eq!(first, parent.join("旅行计划.html"));
    fs::write(&first, "keep previous version").unwrap();
    fs::create_dir(parent.join("旅行计划 (2).html")).unwrap();
    let next = export.save(&parent, "旅行计划").unwrap();
    assert_eq!(next, parent.join("旅行计划 (3).html"));
    assert_eq!(fs::read(next).unwrap(), export.files["canvas.html"]);
    assert_eq!(fs::read_to_string(first).unwrap(), "keep previous version");
    assert_eq!(fs::read_dir(&parent).unwrap().count(), 3);
    assert!(export.save(Path::new("relative"), "Note").is_err());
    let blocked = root.path().join("blocked");
    fs::write(&blocked, "keep").unwrap();
    assert!(export.save(&blocked, "Note").is_err());
    assert_eq!(fs::read_to_string(blocked).unwrap(), "keep");
}

#[cfg(unix)]
#[test]
fn single_html_downloads_skip_dangling_symlinks() {
    let root = tempfile::tempdir().unwrap();
    let source = note(root.path(), "note", "<p>Single HTML</p>");
    let export = prepare_download(root.path(), &source, false, Dependencies::default()).unwrap();
    let target = root.path().join("missing");
    std::os::unix::fs::symlink(&target, root.path().join("Note.html")).unwrap();
    let output = export.save(root.path(), "Note").unwrap();
    assert_eq!(output, root.path().join("Note (2).html"));
    assert!(!target.exists());
    assert_eq!(fs::read(output).unwrap(), export.files["canvas.html"]);
}

#[test]
fn exports_create_named_directories_without_overwriting_previous_downloads() {
    let root = tempfile::tempdir().unwrap();
    let source = note(root.path(), "note", "<p>First</p><img src='image.png'>");
    fs::write(source.join("image.png"), [1, 2, 3]).unwrap();
    let export = prepare_download(root.path(), &source, false, Dependencies::default()).unwrap();
    let parent = root.path().join("Downloads");
    let first = export.save(&parent, "旅行计划").unwrap();
    assert_eq!(first, parent.join("旅行计划"));
    assert_eq!(fs::read(first.join("image.png")).unwrap(), [1, 2, 3]);
    fs::write(first.join("canvas.html"), "keep previous version").unwrap();
    let second = export.save(&parent, "旅行计划").unwrap();
    assert_eq!(second, parent.join("旅行计划 (2)"));
    assert_eq!(
        fs::read_to_string(first.join("canvas.html")).unwrap(),
        "keep previous version"
    );
    assert_eq!(
        fs::read(second.join("canvas.html")).unwrap(),
        export.files["canvas.html"]
    );
    assert!(export.save(Path::new("relative"), "Note").is_err());
}

#[test]
fn export_titles_cannot_escape_the_destination_and_work_as_file_names() {
    assert_eq!(export_name(" ../旅行/计划:一? "), "_旅行_计划_一_");
    assert_eq!(export_name("..."), "canvas");
    assert_eq!(export_name("CON.txt"), "_CON.txt");
    assert_eq!(export_name("lpt9"), "_lpt9");
    assert_eq!(export_name("正常标题"), "正常标题");
    assert!(export_name(&"旅".repeat(100)).len() < 255);
}

#[test]
fn failed_directory_export_cleans_its_new_directory() {
    let root = tempfile::tempdir().unwrap();
    let export = CanvasDownload {
        files: [
            ("canvas.html".to_owned(), vec![1]),
            ("missing/file".to_owned(), vec![1]),
        ]
        .into(),
    };
    assert!(export.save(root.path(), "Incomplete").is_err());
    assert!(!root.path().join("Incomplete").exists());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}
