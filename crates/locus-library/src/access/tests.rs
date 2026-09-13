use super::*;

#[test]
fn content_round_trip_preserves_bytes_and_rejects_stale_writes() {
    let root = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    library
        .create_note(
            "locus://note",
            "---\r\ntitle: Note\r\ncustom: [1, 2]\r\n---\r\nBody\r\n",
        )
        .unwrap();
    let target = ContentTarget::new("locus://note", Resource::Note);
    let (original, version) = library.read_text(&target).unwrap();
    library
        .patch_text(&target, &version, "Body", "Updated")
        .unwrap();
    assert!(library.write_text(&target, "stale", &version).is_err());
    assert_eq!(
        library.read_text(&target).unwrap().0,
        original.replace("Body", "Updated")
    );
    let attachment = ContentTarget::new(
        "locus://note",
        Resource::Attachment {
            name: "file.bin".into(),
        },
    );
    let bytes = [0, 128, 255, 42];
    let result = library
        .write_content(&attachment, &mut bytes.as_slice(), "missing")
        .unwrap();
    assert_eq!(
        library
            .read_chunk(&attachment, result["version"].as_str().unwrap(), 0, 20)
            .unwrap(),
        bytes
    );
    assert!(!result.to_string().contains(root.path().to_str().unwrap()));
}

#[test]
fn archives_directories_and_canvas_remain_operable_without_native_paths() {
    let root = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    library.create_directory("locus://目录").unwrap();
    library
        .create_note(
            "locus://目录/笔记",
            "---\ntitle: 标题\ncustom: [one, two]\n---\nBody",
        )
        .unwrap();
    let note = ContentTarget::new("locus://目录/笔记", Resource::Note);
    let version = library.stat(&note).unwrap()["version"]
        .as_str()
        .unwrap()
        .to_owned();
    library.set_archived(note.uri(), &version, true).unwrap();
    assert_eq!(
        library.list("locus://目录", 0, 50).unwrap()["entries"][0]["metadata"]["archived"],
        true
    );
    assert_eq!(library.search("标题", 0, 10).unwrap()["total"], 1);
    let canvas = ContentTarget::new(note.uri(), Resource::Canvas);
    library
        .write_text(&canvas, "<div>Canvas</div>", "missing")
        .unwrap();
    assert_eq!(
        crate::load_canvas_template(root.path(), note.uri()).unwrap(),
        "<div>Canvas</div>"
    );
    let manifest = library.export_manifest("locus://目录").unwrap();
    assert!(!manifest.to_string().contains(root.path().to_str().unwrap()));
    assert_eq!(manifest["entries"].as_array().unwrap().len(), 4);
    let current = library.stat(&note).unwrap()["version"]
        .as_str()
        .unwrap()
        .to_owned();
    library.set_archived(note.uri(), &current, false).unwrap();
    let tree = crate::LibraryTree::load(Some(root.path()));
    assert!(tree.node("目录/笔记").is_some());
}

#[test]
fn moving_tree_updates_real_markdown_links_and_preserves_code() {
    let root = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    library.create_directory("locus://group").unwrap();
    library
        .create_note(
            "locus://source",
            "[Other](../other/index.md)\n![Image](assets/image.png)",
        )
        .unwrap();
    library
        .create_note("locus://source/child", "Child")
        .unwrap();
    library.create_note("locus://other", "[Note](locus://source)\n[Child](../source/child/index.md)\n[Reference][x]\n\n[x]: locus://source\n\n`[Code](locus://source)`").unwrap();
    fs::create_dir(root.path().join("source/assets")).unwrap();
    fs::write(root.path().join("source/assets/image.png"), [1, 2]).unwrap();
    let plan = library
        .plan_move("locus://source", "locus://group/destination")
        .unwrap();
    library
        .move_tree(
            "locus://source",
            "locus://group/destination",
            plan["version"].as_str().unwrap(),
        )
        .unwrap();
    assert!(!root.path().join("source").exists());
    assert!(
        root.path()
            .join("group/destination/child/index.md")
            .exists()
    );
    let (body, _) = library
        .read_text(&ContentTarget::new("locus://other", Resource::Note))
        .unwrap();
    assert!(body.contains("[Note](locus://group/destination)"), "{body}");
    assert!(
        body.contains("../group/destination/child/index.md"),
        "{body}"
    );
    assert!(body.contains("[x]: locus://group/destination"), "{body}");
    assert!(body.contains("`[Code](locus://source)`"), "{body}");
    let moved = library
        .read_text(&ContentTarget::new(
            "locus://group/destination",
            Resource::Note,
        ))
        .unwrap()
        .0;
    assert!(moved.contains("../../other/index.md"), "{moved}");
    assert!(moved.contains("(assets/image.png)"));
}

#[test]
fn delete_requires_current_tree_version_and_explicit_recursive_scope() {
    let root = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    library.create_note("locus://note", "Old").unwrap();
    let version = library.inspect_tree("locus://note").unwrap()["version"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        library
            .delete_tree("locus://note", &version, false)
            .is_err()
    );
    fs::write(root.path().join("note/index.md"), "New").unwrap();
    assert!(library.delete_tree("locus://note", &version, true).is_err());
    let version = library.inspect_tree("locus://note").unwrap()["version"]
        .as_str()
        .unwrap()
        .to_owned();
    library.delete_tree("locus://note", &version, true).unwrap();
    assert!(library.delete_tree("locus://", "missing", true).is_err());
}

#[test]
fn unsafe_targets_and_reserved_attachment_names_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    library.create_note("locus://note", "Note").unwrap();
    for uri in [
        "file:///tmp",
        "locus:///tmp",
        "locus://..",
        "locus://note/%2e%2e",
        "locus://note/%2fetc",
        "locus://.secret",
        "locus://node_modules",
        "locus://C:",
        "locus://NUL.txt",
    ] {
        assert!(library.create_directory(uri).is_err(), "{uri}");
    }
    for name in ["../outside", "ICON.PNG", ".secret", "a/b", "x:stream"] {
        let target = ContentTarget::new("locus://note", Resource::Attachment { name: name.into() });
        assert!(
            library.write_text(&target, "Bad", "missing").is_err(),
            "{name}"
        );
    }
    fs::write(root.path().join("note/.hidden"), "Preserve").unwrap();
    assert!(library.inspect_tree("locus://note").is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_ancestors_and_resources_never_escape_library() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    symlink(outside.path(), root.path().join("escape")).unwrap();
    assert!(library.create_note("locus://escape/note", "Bad").is_err());
    library.create_note("locus://note", "Good").unwrap();
    fs::write(outside.path().join("secret"), "Secret").unwrap();
    fs::create_dir(root.path().join("note/assets")).unwrap();
    symlink(
        outside.path().join("secret"),
        root.path().join("note/assets/link.txt"),
    )
    .unwrap();
    let target = ContentTarget::new(
        "locus://note",
        Resource::Attachment {
            name: "link.txt".into(),
        },
    );
    assert!(library.stat(&target).is_err());
    assert!(library.write_text(&target, "Bad", "missing").is_err());
    assert_eq!(
        fs::read_to_string(outside.path().join("secret")).unwrap(),
        "Secret"
    );
}

#[test]
fn icon_upload_and_removal_keep_body_and_custom_metadata() {
    let root = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    library
        .create_note(
            "locus://note",
            "---\r\ntitle: Note\r\ncustom: [a, b]\r\n---\r\nBody\r\n",
        )
        .unwrap();
    let image = image::DynamicImage::new_rgba8(100, 50);
    let mut source = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut source, image::ImageFormat::Png)
        .unwrap();
    source.set_position(0);
    let target = ContentTarget::new("locus://note", Resource::Icon);
    let written = library
        .write_content(&target, &mut source, "missing")
        .unwrap();
    assert_eq!(
        image::image_dimensions(root.path().join("note/assets/icon.png")).unwrap(),
        (64, 32)
    );
    let description = library.describe("locus://note").unwrap();
    assert_eq!(description["metadata"]["icon"], "assets/icon.png");
    assert_eq!(description["metadata"]["custom"], json!(["a", "b"]));
    library
        .delete_content(&target, written["version"].as_str().unwrap())
        .unwrap();
    assert!(!root.path().join("note/assets/icon.png").exists());
    assert!(library.describe("locus://note").unwrap()["metadata"]["icon"].is_null());
    let text = library
        .read_text(&ContentTarget::new("locus://note", Resource::Note))
        .unwrap()
        .0;
    assert!(text.ends_with("---\r\nBody\r\n"));
}

#[test]
fn move_preserves_frontmatter_labels_titles_and_code_block_links() {
    let root = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    library.create_note("locus://source", "Source").unwrap();
    let content = "---\ncustom: '[label](locus://source)'\n---\n[locus://source](locus://source \"locus://source\")\n\n```md\n[x](locus://source)\n```\n";
    library.create_note("locus://other", content).unwrap();
    let plan = library
        .plan_move("locus://source", "locus://renamed")
        .unwrap();
    library
        .move_tree(
            "locus://source",
            "locus://renamed",
            plan["version"].as_str().unwrap(),
        )
        .unwrap();
    let moved = library
        .read_text(&ContentTarget::new("locus://other", Resource::Note))
        .unwrap()
        .0;
    assert_eq!(
        moved,
        content.replace("](locus://source \"", "](locus://renamed \"")
    );
}

#[test]
fn manifest_and_chunks_reconstruct_notes_canvas_and_binary_attachments() {
    let source = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(source.path().to_owned()));
    let imported = LibraryAccess::new(Some(destination.path().to_owned()));
    library.create_directory("locus://group").unwrap();
    library
        .create_note("locus://group/note", "---\ntitle: Note\n---\nText")
        .unwrap();
    library
        .create_note("locus://group/note/child", "Child")
        .unwrap();
    library
        .write_text(
            &ContentTarget::new("locus://group/note", Resource::Canvas),
            "<p>Canvas</p>",
            "missing",
        )
        .unwrap();
    library
        .write_content(
            &ContentTarget::new(
                "locus://group/note",
                Resource::Attachment {
                    name: "data.bin".into(),
                },
            ),
            &mut [0_u8, 255, 128].as_slice(),
            "missing",
        )
        .unwrap();
    let manifest = library.export_manifest("locus://group").unwrap();
    let entries = manifest["entries"].as_array().unwrap();
    let mut directories = entries
        .iter()
        .filter_map(|entry| entry["uri"].as_str())
        .collect::<Vec<_>>();
    directories.sort_by_key(|uri| uri.matches('/').count());
    for uri in directories {
        imported.create_directory(uri).unwrap();
    }
    let mut contents = entries
        .iter()
        .filter(|entry| entry.get("target").is_some())
        .collect::<Vec<_>>();
    contents.sort_by_key(|entry| entry["target"]["resource"]["kind"] != "note");
    for entry in contents {
        let target: ContentTarget = serde_json::from_value(entry["target"].clone()).unwrap();
        let bytes = library
            .read_chunk(&target, entry["version"].as_str().unwrap(), 0, 256 * 1024)
            .unwrap();
        imported
            .write_content(&target, &mut bytes.as_slice(), "missing")
            .unwrap();
    }
    assert_eq!(imported.export_manifest("locus://group").unwrap(), manifest);
}

#[test]
fn attachment_moves_update_markdown_links_across_notes_without_overwriting() {
    let root = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    library
        .create_note("locus://source", "![Image](assets/image.png)")
        .unwrap();
    library
        .create_note("locus://destination", "[Image](../source/assets/image.png)")
        .unwrap();
    let source = ContentTarget::new(
        "locus://source",
        Resource::Attachment {
            name: "image.png".into(),
        },
    );
    let destination = ContentTarget::new(
        "locus://destination",
        Resource::Attachment {
            name: "new.png".into(),
        },
    );
    let written = library
        .write_content(&source, &mut [1_u8, 2, 3].as_slice(), "missing")
        .unwrap();
    let result = library
        .move_attachment(&source, &destination, written["version"].as_str().unwrap())
        .unwrap();
    assert_eq!(result["updatedNotes"].as_array().unwrap().len(), 2);
    assert_eq!(
        library
            .read_text(&ContentTarget::new("locus://source", Resource::Note))
            .unwrap()
            .0,
        "![Image](../destination/assets/new.png)"
    );
    assert_eq!(
        library
            .read_text(&ContentTarget::new("locus://destination", Resource::Note))
            .unwrap()
            .0,
        "[Image](assets/new.png)"
    );
    library.write_text(&source, "Keep", "missing").unwrap();
    assert!(
        library
            .move_attachment(&source, &destination, &content_version(b"Keep"))
            .is_err()
    );
    assert_eq!(library.read_text(&source).unwrap().0, "Keep");
}

#[test]
fn reference_labels_and_code_inside_link_labels_are_not_rewritten() {
    let root = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    library.create_note("locus://source", "Source").unwrap();
    let content = "[Link][locus://source]\n\n[locus://source]: locus://source \"locus://source\"\n\n[Code `[x](locus://source)`](locus://source)";
    library.create_note("locus://other", content).unwrap();
    let plan = library.plan_move("locus://source", "locus://new").unwrap();
    library
        .move_tree(
            "locus://source",
            "locus://new",
            plan["version"].as_str().unwrap(),
        )
        .unwrap();
    let updated = library
        .read_text(&ContentTarget::new("locus://other", Resource::Note))
        .unwrap()
        .0;
    assert_eq!(
        updated,
        "[Link][locus://source]\n\n[locus://source]: locus://new \"locus://source\"\n\n[Code `[x](locus://source)`](locus://new)"
    );
}

#[test]
fn emitted_addresses_resolve_in_navigation_and_canvas_templates() {
    let root = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    for uri in [
        "locus://旅行%20计划",
        "locus://100%25",
        "locus://literal%252e",
        "locus://note%23one",
    ] {
        library.create_note(uri, "Note").unwrap();
        library
            .write_text(
                &ContentTarget::new(uri, Resource::Canvas),
                "<p>Template</p>",
                "missing",
            )
            .unwrap();
    }
    let listed = library.list("locus://", 0, 50).unwrap();
    let tree = crate::LibraryTree::load(Some(root.path()));
    for entry in listed["entries"].as_array().unwrap() {
        let uri = entry["uri"].as_str().unwrap();
        let path = crate::library_reference_path(uri).unwrap();
        assert!(tree.node(&path).is_some(), "{uri}");
        assert_eq!(
            crate::load_canvas_template(root.path(), uri).unwrap(),
            "<p>Template</p>"
        );
    }
}

#[test]
fn resource_directories_are_not_notes_and_canvas_files_round_trip() {
    let root = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    library
        .create_note(
            "locus://note",
            "---\ntitle: Note\nitems: [1, 2]\n---\nBody\n",
        )
        .unwrap();
    for uri in [
        "locus://assets",
        "locus://note/canvas",
        "locus://note/Assets/child",
    ] {
        assert!(library.create_note(uri, "Invalid").is_err());
        assert!(library.create_directory(uri).is_err());
    }
    let target = ContentTarget::new(
        "locus://note",
        Resource::CanvasFile {
            name: "scripts/.data/config.json".into(),
        },
    );
    library
        .write_text(&target, "{\"value\":42}", "missing")
        .unwrap();
    assert_eq!(library.read_text(&target).unwrap().0, "{\"value\":42}");
    assert_eq!(
        library.list("locus://note", 0, 20).unwrap()["entries"],
        json!([])
    );
    assert_eq!(library.describe("locus://note").unwrap()["childCount"], 0);
    let bytes =
        crate::load_library_resource(root.path(), "locus://note/canvas/scripts/.data/config.json")
            .unwrap();
    assert_eq!(bytes, b"{\"value\":42}");
    assert_eq!(
        crate::load_library_resource(root.path(), "locus://note/index.md?part=body").unwrap(),
        b"Body\n"
    );
    assert_eq!(
        crate::load_library_resource(root.path(), "locus://note/index.md?part=yaml").unwrap(),
        b"title: Note\nitems: [1, 2]"
    );
    let attachment = ContentTarget::new(
        "locus://note",
        Resource::Attachment {
            name: "index.md".into(),
        },
    );
    library
        .write_text(&attachment, "Attachment", "missing")
        .unwrap();
    assert_eq!(
        crate::load_library_resource(root.path(), "locus://note/assets/index.md").unwrap(),
        b"Attachment"
    );
    assert_eq!(
        crate::LibraryTree::load(Some(root.path()))
            .node("note")
            .unwrap()
            .attachments[0]
            .name,
        "index.md"
    );
    for uri in [
        "locus://note/assets/../index.md",
        "locus://note/canvas/%2e%2e/index.md",
        "locus://note/index.md?part=wrong",
    ] {
        assert!(crate::load_library_resource(root.path(), uri).is_err());
    }
}

#[test]
fn note_data_handles_empty_yaml_bom_and_crlf_without_losing_body() {
    let root = tempfile::tempdir().unwrap();
    let library = LibraryAccess::new(Some(root.path().to_owned()));
    for (name, source, yaml, body) in [
        ("empty", "---\n---\nBody\n", "", "Body\n"),
        (
            "windows",
            "\u{feff}---\r\ntitle: Note\r\nlist: [1, 2]\r\n---\r\nBody\r\n",
            "title: Note\r\nlist: [1, 2]",
            "Body\r\n",
        ),
        ("plain", "Body\n", "", "Body\n"),
    ] {
        let uri = format!("locus://{name}");
        library.create_note(&uri, source).unwrap();
        assert_eq!(
            crate::load_library_resource(root.path(), &format!("{uri}/index.md?part=yaml"))
                .unwrap(),
            yaml.as_bytes()
        );
        assert_eq!(
            crate::load_library_resource(root.path(), &format!("{uri}/index.md?part=body"))
                .unwrap(),
            body.as_bytes()
        );
        assert_eq!(
            crate::load_library_resource(root.path(), &format!("{uri}/index.md")).unwrap(),
            source.as_bytes()
        );
    }
}
