use std::sync::Arc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, App, Asset, ImageCacheError, ObjectFit, RenderImage,
    SharedString, StyledImage, Transformation, Window, div, img, percentage, prelude::*, px,
};
use image::{DynamicImage, GenericImageView, ImageDecoder, ImageReader, metadata::Orientation};

use crate::document_library::Attachment;
use crate::theme::Theme;
use crate::ui::{Icon, icon};

const THUMBNAIL_MAX_SIZE: u32 = 640;

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
            let reader = ImageReader::open(&source.path)?.with_guessed_format()?;
            let mut decoder = reader.into_decoder()?;
            let orientation = decoder.orientation()?;
            let image = DynamicImage::from_decoder(decoder)?;
            Ok(render_thumbnail(image, orientation))
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

fn render_thumbnail(mut image: DynamicImage, orientation: Orientation) -> Arc<RenderImage> {
    image.apply_orientation(orientation);
    let (width, height) = image.dimensions();
    let mut pixels = if width > THUMBNAIL_MAX_SIZE || height > THUMBNAIL_MAX_SIZE {
        image.thumbnail(THUMBNAIL_MAX_SIZE, THUMBNAIL_MAX_SIZE)
    } else {
        image
    }
    .into_rgba8();
    for pixel in pixels.as_mut().as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    Arc::new(RenderImage::new(vec![image::Frame::new(pixels)]))
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
        let thumbnail = render_thumbnail(
            DynamicImage::new_rgba8(1280, 640),
            Orientation::NoTransforms,
        );
        let size = thumbnail.size(0);
        assert_eq!((size.width.0, size.height.0), (640, 320));

        let small = render_thumbnail(DynamicImage::new_rgba8(120, 80), Orientation::NoTransforms);
        let size = small.size(0);
        assert_eq!((size.width.0, size.height.0), (120, 80));
    }
}
