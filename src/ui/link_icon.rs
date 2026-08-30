use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use futures::AsyncReadExt;
use gpui::{App, Asset, ImageCacheError, Img, RenderImage, Window, img};
use http_client::{AsyncBody, HttpClient, Url};
use regex::Regex;
use reqwest_client::ReqwestClient;

use crate::config::LINK_ICON_CACHE_DIRECTORY;
use crate::storage::data_directory;

const HTML_LIMIT: usize = 1024 * 1024;
const ICON_LIMIT: usize = 4 * 1024 * 1024;
const ICON_MAX_DIMENSION: u32 = 4096;
const ICON_RENDER_SIZE: u32 = 64;
const ICON_MAX_ALLOCATION: u64 = 64 * 1024 * 1024;
const CACHE_VERSION: u8 = 2;
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

static LINK_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<link\b[^>]*>").expect("link tag regex should compile"));
static ATTRIBUTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)\b([a-z_:][-a-z0-9_:.]*)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+))"#)
        .expect("HTML attribute regex should compile")
});
static LINK_HTTP_CLIENT: LazyLock<Arc<ReqwestClient>> = LazyLock::new(|| {
    Arc::new(
        ReqwestClient::proxy_user_agent_and_read_timeout(
            None,
            concat!("Locus/", env!("CARGO_PKG_VERSION")),
            Some(Duration::from_secs(15)),
        )
        .expect("failed to initialize link icon HTTP client"),
    )
});

#[derive(Clone, Eq, Hash, PartialEq)]
struct RemoteLinkIconSource(String);

enum RemoteLinkIconLoader {}

impl Asset for RemoteLinkIconLoader {
    type Source = RemoteLinkIconSource;
    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    fn load(
        source: Self::Source,
        cx: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let renderer = cx.svg_renderer();
        async move {
            for cache_path in cached_icon_paths(&source.0).map_err(asset_error)? {
                if let Ok(bytes) = fs::read(&cache_path) {
                    match prepare_icon(&bytes, renderer.clone()) {
                        Ok(icon) => return Ok(icon),
                        Err(_) => {
                            let _ = fs::remove_file(&cache_path);
                        }
                    }
                }
            }

            let (icon_url, bytes, icon) = fetch_icon_on_runtime(source.0.clone(), renderer)
                .await
                .map_err(asset_error)?;
            let cache_path = icon_cache_path(&source.0, &icon_url, &bytes).map_err(asset_error)?;
            let _ = write_cached_icon(&cache_path, &bytes);
            Ok(icon)
        }
    }
}

pub fn remote_link_icon(url: &str) -> Img {
    let source = RemoteLinkIconSource(url.to_owned());
    img(move |window: &mut Window, cx: &mut App| {
        window.use_asset::<RemoteLinkIconLoader>(&source, cx)
    })
}

async fn fetch_icon_on_runtime(
    page: String,
    renderer: gpui::SvgRenderer,
) -> Result<(Url, Vec<u8>, Arc<RenderImage>), String> {
    let client = LINK_HTTP_CLIENT.clone();
    reqwest_client::runtime()
        .spawn(async move { fetch_icon(client.as_ref(), &page, renderer).await })
        .await
        .map_err(|error| error.to_string())?
}

async fn fetch_icon(
    client: &dyn HttpClient,
    page: &str,
    renderer: gpui::SvgRenderer,
) -> Result<(Url, Vec<u8>, Arc<RenderImage>), String> {
    let page_url = Url::parse(page).map_err(|error| error.to_string())?;
    if !matches!(page_url.scheme(), "http" | "https") {
        return Err("远端链接只支持 HTTP 和 HTTPS".to_owned());
    }
    let fallback = page_url
        .join("/favicon.ico")
        .map_err(|error| error.to_string())?;
    let discovered = match fetch_bytes(client, page_url.as_str(), HTML_LIMIT).await {
        Ok(html) => icon_url(&page_url, &String::from_utf8_lossy(&html)),
        Err(_) => None,
    };

    if let Some(icon_url) = discovered.as_ref().filter(|url| **url != fallback)
        && let Ok(bytes) = fetch_bytes(client, icon_url.as_str(), ICON_LIMIT).await
        && let Ok(icon) = prepare_icon(&bytes, renderer.clone())
    {
        return Ok((icon_url.clone(), bytes, icon));
    }
    let bytes = fetch_bytes(client, fallback.as_str(), ICON_LIMIT).await?;
    let icon = prepare_icon(&bytes, renderer)?;
    Ok((fallback, bytes, icon))
}

async fn fetch_bytes(client: &dyn HttpClient, url: &str, limit: usize) -> Result<Vec<u8>, String> {
    let mut response = client
        .get(url, AsyncBody::default(), true)
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("{url} 返回 {}", response.status()));
    }
    let mut bytes = Vec::new();
    response
        .body_mut()
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| error.to_string())?;
    if bytes.len() > limit {
        return Err(format!("{url} 超过允许大小"));
    }
    Ok(bytes)
}

fn icon_url(page_url: &Url, html: &str) -> Option<Url> {
    for tag in LINK_TAG.find_iter(html) {
        let mut rel = None;
        let mut href = None;
        for captures in ATTRIBUTE.captures_iter(tag.as_str()) {
            let Some(name) = captures.get(1).map(|value| value.as_str()) else {
                continue;
            };
            let Some(value) = captures
                .get(2)
                .or_else(|| captures.get(3))
                .or_else(|| captures.get(4))
                .map(|value| value.as_str())
            else {
                continue;
            };
            if name.eq_ignore_ascii_case("rel") {
                rel = Some(value);
            } else if name.eq_ignore_ascii_case("href") {
                href = Some(value);
            }
        }
        let is_icon = rel.is_some_and(|value| {
            value
                .split_ascii_whitespace()
                .any(|token| token.to_ascii_lowercase().contains("icon"))
        });
        if !is_icon {
            continue;
        }
        let Some(href) = href else {
            continue;
        };
        let href = href.replace("&amp;", "&");
        let Ok(url) = page_url.join(&href) else {
            continue;
        };
        if matches!(url.scheme(), "http" | "https") {
            return Some(url);
        }
    }
    None
}

fn prepare_icon(bytes: &[u8], renderer: gpui::SvgRenderer) -> Result<Arc<RenderImage>, String> {
    let Ok(format) = image::guess_format(bytes) else {
        return renderer
            .render_single_frame(bytes, 1.0)
            .map_err(|error| error.to_string());
    };

    let mut limits = image::Limits::default();
    limits.max_image_width = Some(ICON_MAX_DIMENSION);
    limits.max_image_height = Some(ICON_MAX_DIMENSION);
    limits.max_alloc = Some(ICON_MAX_ALLOCATION);
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(limits);
    let image = reader.decode().map_err(|error| error.to_string())?;
    if image.width() == 0 || image.height() == 0 {
        return Err("远端图标尺寸无效".to_owned());
    }
    let image = if image.width() > ICON_RENDER_SIZE || image.height() > ICON_RENDER_SIZE {
        image.thumbnail(ICON_RENDER_SIZE, ICON_RENDER_SIZE)
    } else {
        image
    };
    let mut pixels = image.into_rgba8();
    for pixel in pixels.as_mut().as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    Ok(Arc::new(RenderImage::new(vec![image::Frame::new(pixels)])))
}

fn cached_icon_paths(page: &str) -> Result<Vec<PathBuf>, String> {
    let directory = data_directory()?.join(LINK_ICON_CACHE_DIRECTORY);
    let prefix = cache_file_prefix(page);
    let Ok(entries) = fs::read_dir(directory) else {
        return Ok(Vec::new());
    };
    let mut paths = entries
        .flatten()
        .filter(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_file())
                && entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with(&prefix) && !name.ends_with(".tmp"))
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn icon_cache_path(page: &str, icon_url: &Url, bytes: &[u8]) -> Result<PathBuf, String> {
    Ok(data_directory()?
        .join(LINK_ICON_CACHE_DIRECTORY)
        .join(cache_file_name(page, icon_url, bytes)))
}

fn cache_file_name(page: &str, icon_url: &Url, bytes: &[u8]) -> String {
    let original = icon_url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|name| !name.is_empty())
        .map(|name| {
            percent_encoding::percent_decode_str(name)
                .decode_utf8_lossy()
                .into_owned()
        })
        .map(|name| sanitize_file_name(&name))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("favicon.{}", icon_extension(bytes)));
    format!("{}{original}", cache_file_prefix(page))
}

fn cache_file_prefix(page: &str) -> String {
    let hash = stable_hash(page.as_bytes());
    format!("v{CACHE_VERSION}-{hash:016x}-")
}

fn sanitize_file_name(name: &str) -> String {
    let mut sanitized = String::new();
    for character in name.chars() {
        let character = if character.is_control() || matches!(character, '/' | '\\' | ':') {
            '_'
        } else {
            character
        };
        if sanitized.len() + character.len_utf8() > 180 {
            break;
        }
        sanitized.push(character);
    }
    sanitized.trim_matches('.').to_owned()
}

fn icon_extension(bytes: &[u8]) -> &'static str {
    image::guess_format(bytes)
        .ok()
        .and_then(|format| format.extensions_str().first().copied())
        .unwrap_or("svg")
}

fn write_cached_icon(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let directory = path
        .parent()
        .ok_or_else(|| "远端图标缓存路径无效".to_owned())?;
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let write_result = (|| {
        let mut file = fs::File::create(&temporary).map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.flush().map_err(|error| error.to_string())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        if !path.is_file() {
            return Err(error.to_string());
        }
    }
    remove_stale_cached_icons(directory, path);
    Ok(())
}

fn remove_stale_cached_icons(directory: &Path, current: &Path) {
    let Some(current_name) = current.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    let Some(prefix_end) = current_name
        .match_indices('-')
        .nth(1)
        .map(|(index, _)| index + 1)
    else {
        return;
    };
    let prefix = &current_name[..prefix_end];
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path != current
            && entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(prefix))
        {
            let _ = fs::remove_file(path);
        }
    }
}

fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

fn asset_error(error: impl ToString) -> ImageCacheError {
    ImageCacheError::Other(Arc::new(http_client::anyhow!(error.to_string())))
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{Ipv4Addr, TcpListener};

    use super::*;

    #[test]
    fn resolves_relative_icons_and_attribute_order() {
        let page = Url::parse("https://example.com/docs/page").unwrap();
        let html =
            r#"<link href='../assets/icon.png?x=1&amp;y=2' sizes='32x32' rel='shortcut icon'>"#;
        assert_eq!(
            icon_url(&page, html).unwrap().as_str(),
            "https://example.com/assets/icon.png?x=1&y=2"
        );
    }

    #[test]
    fn ignores_non_icon_and_non_http_links() {
        let page = Url::parse("https://example.com/docs").unwrap();
        assert!(icon_url(&page, r#"<link rel="stylesheet" href="app.css">"#).is_none());
        assert!(icon_url(&page, r#"<link rel="icon" href="data:image/png;base64,x">"#).is_none());
    }

    #[test]
    fn cache_name_is_stable_and_versioned() {
        let page = "https://example.com/docs";
        let icon = Url::parse("https://example.com/assets/favicon.ico?v=2").unwrap();
        assert_eq!(
            cache_file_name(page, &icon, b"not used when the URL has a name"),
            "v2-39bfb9340ba2f7fb-favicon.ico"
        );
    }

    #[test]
    fn raster_icons_are_bounded() {
        let mut source = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(128, 64)
            .write_to(&mut source, image::ImageFormat::Png)
            .unwrap();
        let icon = prepare_icon(source.get_ref(), gpui::SvgRenderer::new(Arc::new(()))).unwrap();
        let size = icon.size(0);
        assert_eq!((size.width.0, size.height.0), (64, 32));
    }

    #[test]
    fn unnamed_icons_use_the_detected_extension() {
        let page = "https://example.com/docs";
        let icon = Url::parse("https://example.com/").unwrap();
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1, 1)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();

        assert_eq!(
            cache_file_name(page, &icon, png.get_ref()),
            "v2-39bfb9340ba2f7fb-favicon.png"
        );
    }

    #[test]
    fn cache_names_preserve_decoded_remote_file_names() {
        let icon = Url::parse("https://example.com/assets/My%20Icon.png").unwrap();
        assert_eq!(
            cache_file_name("https://example.com/docs", &icon, b""),
            "v2-39bfb9340ba2f7fb-My Icon.png"
        );
    }

    #[test]
    fn fetches_the_response_body_inside_the_tokio_runtime() {
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2, 2)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let responses = [
                (
                    "text/html",
                    br#"<link rel="icon" href="/favicon.png">"#.as_slice(),
                ),
                ("image/png", png.as_slice()),
            ];
            for (content_type, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                let _ = stream.read(&mut request).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(body).unwrap();
            }
        });

        let page = format!("http://127.0.0.1:{port}/page");
        let (icon_url, _, icon) = futures::executor::block_on(fetch_icon_on_runtime(
            page,
            gpui::SvgRenderer::new(Arc::new(())),
        ))
        .unwrap();

        assert_eq!(icon_url.path(), "/favicon.png");
        assert_eq!((icon.size(0).width.0, icon.size(0).height.0), (2, 2));
        server.join().unwrap();
    }
}
