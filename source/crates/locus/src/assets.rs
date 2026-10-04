use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString};

gpui_kit::assets::icon_assets!(ExtraIcons, [Download, Pencil]);

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match ExtraIcons.load(path)? {
            Some(bytes) => Ok(Some(bytes)),
            None => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(ExtraIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::assets::IconName;

    use super::*;

    #[test]
    fn application_assets_include_toolbar_and_component_icons() {
        let mut paths = gpui_kit::assets::Assets.list("icons/").unwrap();
        paths.extend([IconName::Download.path(), IconName::Pencil.path()]);
        let listed = Assets.list("icons/").unwrap();
        for path in paths {
            assert!(listed.contains(&path), "missing asset listing: {path}");
            let bytes = Assets.load(&path).unwrap().unwrap();
            assert!(
                std::str::from_utf8(&bytes).unwrap().contains("<svg"),
                "invalid icon: {path}"
            );
        }
    }
}
