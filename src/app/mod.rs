mod settings;
mod workspace;

pub use workspace::Locus;

pub(crate) fn bind_keys(cx: &mut gpui::App) {
    settings::text_input::bind_keys(cx);
}
