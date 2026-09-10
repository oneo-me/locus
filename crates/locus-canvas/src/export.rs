use base64::{Engine as _, engine::general_purpose::STANDARD};
use html5ever::tokenizer::{
    BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, states::RawKind,
};
use regex::Regex;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::LazyLock,
};

#[derive(Clone, Default)]
pub(crate) struct Dependencies {
    pub(crate) templates: BTreeSet<String>,
    pub(crate) resources: BTreeSet<String>,
}

pub struct CanvasDownload {
    files: BTreeMap<String, Vec<u8>>,
}

impl CanvasDownload {
    pub fn save(&self, parent: &Path, title: &str) -> io::Result<PathBuf> {
        if !parent.is_absolute() {
            return Err(io::Error::other("Export directory must be absolute"));
        }
        std::fs::create_dir_all(parent)?;
        let title = export_name(title);
        if self.files.len() == 1 {
            return self.save_html(parent, &title);
        }
        for number in 1.. {
            let name = if number == 1 {
                title.clone()
            } else {
                format!("{title} ({number})")
            };
            let directory = parent.join(name);
            match std::fs::create_dir(&directory) {
                Ok(()) => {
                    if let Err(error) = self.write_files(&directory) {
                        let _ = std::fs::remove_dir(&directory);
                        return Err(error);
                    }
                    return Ok(directory);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        unreachable!()
    }

    fn save_html(&self, parent: &Path, title: &str) -> io::Result<PathBuf> {
        let html = self
            .files
            .get("canvas.html")
            .ok_or_else(|| io::Error::other("Missing canvas HTML"))?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(html)?;
        file.as_file().sync_all()?;
        for number in 1.. {
            let name = if number == 1 {
                format!("{title}.html")
            } else {
                format!("{title} ({number}).html")
            };
            let path = parent.join(name);
            match std::fs::symlink_metadata(&path) {
                Ok(_) => continue,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            match file.persist_noclobber(&path) {
                Ok(_) => return Ok(path),
                Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {
                    file = error.file;
                }
                Err(error) => return Err(error.error),
            }
        }
        unreachable!()
    }

    fn write_files(&self, directory: &Path) -> io::Result<()> {
        if !directory.is_dir() {
            return Err(io::Error::other("Choose an existing directory"));
        }
        for name in self.files.keys() {
            match std::fs::symlink_metadata(directory.join(name)) {
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        format!("{name} already exists. Choose another directory."),
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        let mut pending = Vec::new();
        for (name, bytes) in &self.files {
            let mut file = tempfile::NamedTempFile::new_in(directory)?;
            file.write_all(bytes)?;
            file.as_file().sync_all()?;
            pending.push((file, directory.join(name)));
        }
        let mut created = Vec::new();
        for (file, path) in pending {
            match file.persist_noclobber(&path) {
                Ok(_) => created.push(path),
                Err(error) => {
                    for path in created {
                        if let Err(cleanup_error) = std::fs::remove_file(&path) {
                            return Err(io::Error::other(format!(
                                "{}; could not remove incomplete export {}: {cleanup_error}",
                                error.error,
                                path.display()
                            )));
                        }
                    }
                    return Err(error.error);
                }
            }
        }
        Ok(())
    }
}

fn export_name(title: &str) -> String {
    let title: String = title
        .chars()
        .take(60)
        .map(|ch| {
            if ch.is_control() || "<>:\"/\\|?*".contains(ch) {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let title = title.trim_matches(['.', ' ']);
    if title.is_empty() {
        return "canvas".to_owned();
    }
    let stem = title
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || (stem.starts_with("COM") || stem.starts_with("LPT"))
        && stem.len() == 4
        && matches!(stem.as_bytes()[3], b'1'..=b'9')
    {
        return format!("_{title}");
    }
    title.to_owned()
}

pub(crate) fn prepare_download(
    root: &Path,
    directory: &Path,
    dark: bool,
    mut dependencies: Dependencies,
) -> io::Result<CanvasDownload> {
    let source = locus_library::load_canvas(root, directory)?;
    if source.trim().is_empty() {
        return Err(io::Error::other("Canvas is empty"));
    }
    let mut templates = BTreeMap::new();
    let mut resources = BTreeMap::new();
    discover(&source, SourceKind::Html, directory, &mut dependencies)?;
    loop {
        if let Some(uri) = dependencies.templates.pop_first() {
            let key = locus_library::library_reference_path(&uri)
                .ok_or_else(|| io::Error::other("Invalid template address"))?;
            if templates.contains_key(&key) {
                continue;
            }
            let template = locus_library::load_canvas_template(root, &uri)
                .map_err(|error| io::Error::other(format!("{uri}: {error}")))?;
            if template.trim().is_empty() {
                return Err(io::Error::other(format!("Empty template: {uri}")));
            }
            discover(&template, SourceKind::Html, directory, &mut dependencies)?;
            templates.insert(key, template);
        } else if let Some(name) = dependencies.resources.pop_first() {
            if resources.contains_key(&name) {
                continue;
            }
            let bytes = locus_library::load_canvas_resource(root, directory, &name)
                .map_err(|error| io::Error::other(format!("{name}: {error}")))?;
            if matches!(
                Path::new(&name).extension().and_then(|ext| ext.to_str()),
                Some("css" | "js" | "mjs" | "json" | "svg")
            ) {
                let text = std::str::from_utf8(&bytes).map_err(io::Error::other)?;
                let kind = if name.ends_with(".svg") {
                    SourceKind::Html
                } else if name.ends_with(".css") {
                    SourceKind::Css
                } else {
                    SourceKind::Script
                };
                discover(text, kind, directory, &mut dependencies)?;
            }
            resources.insert(name, bytes);
        } else {
            break;
        }
    }
    let encoded: BTreeMap<_, _> = resources
        .iter()
        .map(|(name, bytes)| {
            (
                name,
                format!(
                    "data:{};base64,{}",
                    mime_guess::from_path(name).first_or_octet_stream(),
                    STANDARD.encode(bytes)
                ),
            )
        })
        .collect();
    let bundle = serde_json::json!({"templates": templates, "resources": encoded});
    let bundle = serde_json::to_string(&bundle)
        .map_err(io::Error::other)?
        .replace('<', "\\u003c");
    let runtime = include_str!("export.js");
    let bundle = serde_json::to_string(&bundle)
        .map_err(io::Error::other)?
        .replace('<', "\\u003c");
    let bootstrap = format!("<script>({runtime})(JSON.parse({bundle}));</script>");
    let html = super::view::canvas_document(&source, dark).replacen(
        "<script>",
        &format!("{bootstrap}<script>"),
        1,
    );
    resources.insert("canvas.html".to_owned(), html.into_bytes());
    Ok(CanvasDownload { files: resources })
}

static STRINGS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#""([^"\\]*(?:\\.[^"\\]*)*)"|'([^'\\]*(?:\\.[^'\\]*)*)'|`([^`\\]*(?:\\.[^`\\]*)*)`"#,
    )
    .unwrap()
});
static CSS_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)url\(\s*["']?([^\s"')]+)["']?\s*\)"#).unwrap());
static RESOURCE_CALL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(?:fetch|Worker|SharedWorker|importScripts)\s*\(\s*["'`]([^"'`]+)["'`]\s*[,)]"#)
        .unwrap()
});
static CSS_IMPORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)@import\s+["']([^"']+)["']"#).unwrap());
static TEMPLATE_CALL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\bloadCanvas\s*\(\s*["'`](locus://[^"'`]+)["'`]\s*[,)]"#).unwrap()
});

enum SourceKind {
    Html,
    Css,
    Script,
}

fn discover(
    source: &str,
    kind: SourceKind,
    directory: &Path,
    dependencies: &mut Dependencies,
) -> io::Result<()> {
    for captures in TEMPLATE_CALL.captures_iter(source) {
        if !captures[1].contains("${") {
            dependencies.templates.insert(captures[1].to_owned());
        }
    }
    for captures in STRINGS.captures_iter(source) {
        let value = captures.iter().skip(1).flatten().next().unwrap().as_str();
        if value.starts_with("url(") {
            discover_css(value, dependencies)?;
        }
        if let Ok(Some(name)) = local_name(value)
            && !name.contains(['/', '\\'])
            && !name.eq_ignore_ascii_case("index.md")
            && !name.eq_ignore_ascii_case("canvas.html")
            && std::fs::symlink_metadata(directory.join(&name))
                .is_ok_and(|metadata| !metadata.is_dir())
        {
            dependencies.resources.insert(name);
        }
    }
    for captures in RESOURCE_CALL.captures_iter(source) {
        if !captures[1].contains("${")
            && let Some(name) = local_name(&captures[1])?
        {
            dependencies.resources.insert(name);
        }
    }
    match kind {
        SourceKind::Css => discover_css(source, dependencies)?,
        SourceKind::Html => {
            let input = BufferQueue::default();
            input.push_back(source.into());
            let tokenizer = Tokenizer::new(ResourceUrls::default(), Default::default());
            let _ = tokenizer.feed(&input);
            tokenizer.end();
            for url in tokenizer.sink.urls.into_inner() {
                if let Some(name) = local_name(&url)? {
                    dependencies.resources.insert(name);
                }
            }
            discover_css(&tokenizer.sink.styles.into_inner(), dependencies)?;
        }
        SourceKind::Script => {}
    }
    Ok(())
}

fn discover_css(source: &str, dependencies: &mut Dependencies) -> io::Result<()> {
    for captures in CSS_URL
        .captures_iter(source)
        .chain(CSS_IMPORT.captures_iter(source))
    {
        if !captures[1].contains("${")
            && let Some(name) = local_name(&captures[1])?
        {
            dependencies.resources.insert(name);
        }
    }
    Ok(())
}

fn local_name(url: &str) -> io::Result<Option<String>> {
    let url = url.trim();
    if url.is_empty()
        || url.starts_with(['#', '?'])
        || url.starts_with("//")
        || url.split('/').next().is_some_and(|part| part.contains(':'))
    {
        return Ok(None);
    }
    let path = url.split(['?', '#']).next().unwrap_or_default();
    let path = path.strip_prefix("./").unwrap_or(path);
    let name = percent_encoding::percent_decode_str(path)
        .decode_utf8()
        .map_err(io::Error::other)?
        .into_owned();
    if name.is_empty() {
        return Ok(None);
    }
    Ok(Some(name))
}

#[derive(Default)]
struct ResourceUrls {
    urls: RefCell<Vec<String>>,
    styles: RefCell<String>,
    in_style: Cell<bool>,
}

impl TokenSink for ResourceUrls {
    type Handle = ();
    fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<()> {
        if let Token::CharacterTokens(text) = &token
            && self.in_style.get()
        {
            self.styles.borrow_mut().push_str(text);
        }
        if let Token::TagToken(tag) = token {
            if tag.name.as_ref() == "style" {
                self.in_style.set(tag.kind == TagKind::StartTag);
                self.styles.borrow_mut().push('\n');
            }
            if tag.kind != TagKind::StartTag {
                return TokenSinkResult::Continue;
            }
            for attribute in tag.attrs {
                let name = attribute.name.local.as_ref();
                if matches!(name, "src" | "poster" | "href" | "xlink:href") {
                    self.urls.borrow_mut().push(attribute.value.to_string());
                } else if name == "style" {
                    self.styles.borrow_mut().push_str(&attribute.value);
                    self.styles.borrow_mut().push('\n');
                } else if name == "srcset" {
                    self.urls.borrow_mut().extend(srcset_urls(&attribute.value));
                }
            }
            return match tag.name.as_ref() {
                "script" => TokenSinkResult::RawData(RawKind::ScriptData),
                "style" => TokenSinkResult::RawData(RawKind::Rawtext),
                _ => TokenSinkResult::Continue,
            };
        }
        TokenSinkResult::Continue
    }
}

#[cfg(test)]
mod tests;

fn srcset_urls(mut source: &str) -> Vec<String> {
    let mut urls = Vec::new();
    while !source.is_empty() {
        source = source.trim_start_matches(|ch: char| ch.is_ascii_whitespace() || ch == ',');
        let end = source
            .find(|ch: char| ch.is_ascii_whitespace())
            .unwrap_or(source.len());
        let url = &source[..end];
        source = &source[end..];
        if !url.is_empty() {
            urls.push(url.trim_end_matches(',').to_owned());
        }
        if !url.ends_with(',') {
            source = source.split_once(',').map_or("", |(_, rest)| rest);
        }
    }
    urls
}
