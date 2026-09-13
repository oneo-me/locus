use regex::Regex;
use std::{io, path::Path, sync::LazyLock};

pub(crate) const LIBRARY_PREFIX: &str = "_locus/resource/";

pub(crate) fn rewrite_resources(source: &str) -> String {
    static ADDRESS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"locus://[^\s\"'`<>)]*/(?:assets|canvas)/[^\s\"'`<>)]*|locus://[^\s\"'`<>)]*/index\.md(?:\?part=(?:yaml|body))?"#).unwrap()
    });
    ADDRESS
        .replace_all(source, |captures: &regex::Captures<'_>| {
            format!(
                "/{LIBRARY_PREFIX}{}",
                captures[0].strip_prefix("locus://").unwrap()
            )
        })
        .into_owned()
}

pub(crate) fn resource_key(name: &str) -> &str {
    let (path, query) = name.split_once('?').unwrap_or((name, ""));
    if path.starts_with(LIBRARY_PREFIX) && path.ends_with("/index.md") && query.starts_with("part=")
    {
        name
    } else {
        path
    }
}

pub(crate) fn read_resource(root: &Path, directory: &Path, name: &str) -> io::Result<Vec<u8>> {
    let name = resource_key(name);
    if let Some(path) = name.strip_prefix(LIBRARY_PREFIX) {
        locus_library::load_library_resource(root, &format!("locus://{path}"))
    } else {
        locus_library::load_canvas_resource(root, directory, name.split('?').next().unwrap_or(name))
    }
}

pub(crate) fn resource_mime(name: &str) -> String {
    if name.ends_with("index.md?part=yaml") {
        return "application/yaml; charset=utf-8".into();
    }
    if name
        .split('?')
        .next()
        .is_some_and(|path| path.ends_with("index.md"))
    {
        return "text/markdown; charset=utf-8".into();
    }
    mime_guess::from_path(name.split('?').next().unwrap_or(name))
        .first_or_octet_stream()
        .to_string()
}
