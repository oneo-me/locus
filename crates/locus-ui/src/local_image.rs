use std::sync::Arc;

use gpui::{
    App, Asset, ImageAssetLoader, ImageCacheError, Img, RenderImage, Resource, Window, img,
};

use locus_library::DocumentIcon;

enum DocumentIconLoader {}

impl Asset for DocumentIconLoader {
    type Source = DocumentIcon;
    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    fn load(
        source: Self::Source,
        cx: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        <ImageAssetLoader as Asset>::load(Resource::Path(source.path.into()), cx)
    }
}

pub fn document_icon_image(source: &DocumentIcon) -> Img {
    let source = source.clone();
    img(move |window: &mut Window, cx: &mut App| {
        window.use_asset::<DocumentIconLoader>(&source, cx)
    })
}
