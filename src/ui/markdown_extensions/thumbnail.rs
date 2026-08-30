use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use gpui::{
    Animation, AnimationExt, AnyElement, App, Asset, ImageCacheError, ObjectFit, RenderImage,
    SharedString, StyledImage, Transformation, Window, div, img, percentage, prelude::*, px,
};
use image::{
    DynamicImage, GenericImageView, ImageDecoder, ImageFormat, ImageReader, metadata::Orientation,
};

use crate::config::THUMBNAIL_CACHE_DIRECTORY;
use crate::document_library::Attachment;
use crate::storage::data_directory;
use crate::theme::Theme;
use crate::ui::{Icon, icon};

const THUMBNAIL_MAX_SIZE: u32 = 640;
const THUMBNAIL_CACHE_VERSION: u8 = 1;
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

#[derive(Clone, Eq, Hash, PartialEq)]
struct ThumbnailSource {
    path: std::path::PathBuf,
    modified: Option<std::time::SystemTime>,
    byte_len: u64,
}

impl From<&Attachment> for ThumbnailSource {
    fn from(attachment: &Attachment) -> Self {
        Self {
            path: attachment.path.clone(),
            modified: attachment.modified,
            byte_len: attachment.byte_len,
        }
    }
}

enum ThumbnailLoader {}

impl Asset for ThumbnailLoader {
    type Source = ThumbnailSource;
    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    #[allow(clippy::manual_async_fn)]
    fn load(
        source: Self::Source,
        _: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        async move {
            if let Some(image) = read_cached_thumbnail(&source) {
                return Ok(render_image(image));
            }
            let reader = ImageReader::open(&source.path)?.with_guessed_format()?;
            let mut decoder = reader.into_decoder()?;
            let orientation = decoder.orientation()?;
            let image = DynamicImage::from_decoder(decoder)?;
            let image = prepare_thumbnail(image, orientation);
            let _ = write_cached_thumbnail(&source, &image);
            Ok(render_image(image))
        }
    }
}

pub fn gallery_thumbnail(attachment: &Attachment, index: usize, theme: Theme) -> AnyElement {
    let source = ThumbnailSource::from(attachment);
    let loading_id = SharedString::from(format!("gallery-loading-{index}-{}", attachment.name));
    let loading_theme = theme;
    let fallback_theme = theme;
    img(move |window: &mut Window, cx: &mut App| window.use_asset::<ThumbnailLoader>(&source, cx))
        .size_full()
        .object_fit(ObjectFit::Cover)
        .with_loading(move || loading_view(loading_id.clone(), loading_theme))
        .with_fallback(move || image_fallback(fallback_theme))
        .into_any_element()
}

pub fn inline_thumbnail(attachment: &Attachment, theme: Theme) -> AnyElement {
    let source = ThumbnailSource::from(attachment);
    let loading_id = SharedString::from(format!("inline-image-loading-{}", attachment.name));
    let loading_theme = theme;
    let fallback_theme = theme;
    img(move |window: &mut Window, cx: &mut App| window.use_asset::<ThumbnailLoader>(&source, cx))
        .max_w_full()
        .max_h(px(420.0))
        .object_fit(ObjectFit::ScaleDown)
        .with_loading(move || loading_view(loading_id.clone(), loading_theme))
        .with_fallback(move || image_fallback(fallback_theme))
        .into_any_element()
}

fn prepare_thumbnail(mut image: DynamicImage, orientation: Orientation) -> DynamicImage {
    image.apply_orientation(orientation);
    let (width, height) = image.dimensions();
    if width > THUMBNAIL_MAX_SIZE || height > THUMBNAIL_MAX_SIZE {
        image.thumbnail(THUMBNAIL_MAX_SIZE, THUMBNAIL_MAX_SIZE)
    } else {
        image
    }
}

fn render_image(image: DynamicImage) -> Arc<RenderImage> {
    let mut pixels = image.into_rgba8();
    for pixel in pixels.as_mut().as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    Arc::new(RenderImage::new(vec![image::Frame::new(pixels)]))
}

fn read_cached_thumbnail(source: &ThumbnailSource) -> Option<DynamicImage> {
    let path = thumbnail_cache_path(source).ok()?.0;
    let image = ImageReader::open(&path)
        .ok()
        .and_then(|reader| reader.with_guessed_format().ok())
        .and_then(|reader| reader.into_decoder().ok())
        .and_then(|decoder| DynamicImage::from_decoder(decoder).ok());
    if image.is_none() {
        let _ = fs::remove_file(path);
    }
    image
}

fn write_cached_thumbnail(source: &ThumbnailSource, image: &DynamicImage) -> Result<(), String> {
    let (path, prefix) = thumbnail_cache_path(source)?;
    let directory = path
        .parent()
        .ok_or_else(|| "缩略图缓存路径无效".to_owned())?;
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let write_result = (|| {
        let file = fs::File::create(&temporary).map_err(|error| error.to_string())?;
        let mut writer = BufWriter::new(file);
        image
            .write_to(&mut writer, ImageFormat::Png)
            .map_err(|error| error.to_string())?;
        writer.flush().map_err(|error| error.to_string())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = fs::rename(&temporary, &path) {
        let _ = fs::remove_file(&temporary);
        if !path.is_file() {
            return Err(error.to_string());
        }
    }
    remove_stale_thumbnails(directory, &prefix, &path);
    Ok(())
}

fn thumbnail_cache_path(source: &ThumbnailSource) -> Result<(PathBuf, String), String> {
    let path_hash = stable_hash(source.path.to_string_lossy().as_bytes());
    let mut state = Vec::with_capacity(25);
    state.extend_from_slice(&source.byte_len.to_le_bytes());
    match source
        .modified
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
    {
        Some(modified) => {
            state.extend_from_slice(&modified.as_secs().to_le_bytes());
            state.extend_from_slice(&modified.subsec_nanos().to_le_bytes());
        }
        None => state.extend_from_slice(&[0; 12]),
    }
    state.extend_from_slice(&THUMBNAIL_MAX_SIZE.to_le_bytes());
    state.push(THUMBNAIL_CACHE_VERSION);
    let state_hash = stable_hash(&state);
    let prefix = format!("v{THUMBNAIL_CACHE_VERSION}-{path_hash:016x}-");
    let file_name = format!("{prefix}{state_hash:016x}.png");
    Ok((
        data_directory()?
            .join(THUMBNAIL_CACHE_DIRECTORY)
            .join(file_name),
        prefix,
    ))
}

fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

fn remove_stale_thumbnails(directory: &Path, prefix: &str, current: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path != current
            && entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".png"))
        {
            let _ = fs::remove_file(path);
        }
    }
}

fn loading_view(id: SharedString, theme: Theme) -> AnyElement {
    div()
        .size_full()
        .min_h(px(72.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(theme.raised)
        .child(
            icon(Icon::Loader, 18.0, theme.text_tertiary)
                .with_animation(
                    id,
                    Animation::new(Duration::from_millis(900))
                        .repeat_synced()
                        .with_max_fps(30.0),
                    |icon, phase| {
                        icon.with_transformation(Transformation::rotate(percentage(phase)))
                    },
                )
                .into_any_element(),
        )
        .into_any_element()
}

fn image_fallback(theme: Theme) -> AnyElement {
    div()
        .size_full()
        .min_h(px(72.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(theme.raised)
        .child(icon(Icon::ImageOff, 20.0, theme.text_tertiary))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_preserves_ratio_without_enlarging() {
        let thumbnail = render_image(prepare_thumbnail(
            DynamicImage::new_rgba8(1280, 640),
            Orientation::NoTransforms,
        ));
        let size = thumbnail.size(0);
        assert_eq!((size.width.0, size.height.0), (640, 320));

        let small = render_image(prepare_thumbnail(
            DynamicImage::new_rgba8(120, 80),
            Orientation::NoTransforms,
        ));
        let size = small.size(0);
        assert_eq!((size.width.0, size.height.0), (120, 80));
    }

    #[test]
    fn cache_name_changes_with_source_state() {
        let source = ThumbnailSource {
            path: PathBuf::from("/notes/example/image.jpg"),
            modified: Some(UNIX_EPOCH + Duration::from_secs(10)),
            byte_len: 100,
        };
        let changed = ThumbnailSource {
            byte_len: 101,
            ..source.clone()
        };

        let first = thumbnail_cache_path(&source).unwrap().0;
        let second = thumbnail_cache_path(&changed).unwrap().0;
        assert_ne!(first, second);
        assert_eq!(first.parent(), second.parent());
    }
}
