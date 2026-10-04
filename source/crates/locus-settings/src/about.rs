use super::*;
use gpui_kit::base::Link;
use gpui_kit::component::{Disableable, Sizable, button::Button};
use locus_updater::UpdateStatus;

const PROJECT_URL: &str = "https://github.com/oneo-me/locus";
const RELEASES_URL: &str = "https://github.com/oneo-me/locus/releases/latest";
const LICENSE_URL: &str = "https://github.com/oneo-me/locus/blob/main/LICENSE";

impl Settings {
    pub fn show_about(&mut self, check_updates: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.page = SettingsPage::About;
        self.focus(window, cx);
        cx.notify();
        if check_updates {
            // Defer the native UI until the settings navigation has been applied.
            cx.defer_in(window, |this, _, cx| this.check_for_updates(cx));
        }
    }

    fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        self.updater.update(cx, |updater, cx| updater.check(cx));
    }

    pub(super) fn render_about(&self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let language = self.preferences.appearance.language;
        let locale = language.locale();
        let theme = Theme::global(cx).clone();
        let updater = self.updater.read(cx);
        let description = update_description(updater.status(), language);
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .child(SettingsItem::new(
                "Locus",
                t!("settings.about.description", locale = locale).into_owned(),
                div().text_sm().child(
                    t!(
                        "settings.about.version",
                        locale = locale,
                        version = env!("CARGO_PKG_VERSION")
                    )
                    .into_owned(),
                ),
            ))
            .child(div().mx_5().h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!("settings.about.license", locale = locale).into_owned(),
                "",
                external_link("about-license", "MIT".to_owned(), LICENSE_URL, &theme),
            ))
            .child(div().mx_5().h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!("settings.about.project", locale = locale).into_owned(),
                "",
                external_link("about-project", "GitHub".to_owned(), PROJECT_URL, &theme),
            ))
            .child(div().mx_5().h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!("settings.about.updates", locale = locale).into_owned(),
                description,
                Button::new("check-for-updates")
                    .outline()
                    .small()
                    .label(t!("settings.about.check_updates", locale = locale).into_owned())
                    .loading(updater.status() == &UpdateStatus::Checking)
                    .disabled(!updater.is_check_enabled())
                    .on_click(cx.listener(|this, _, _, cx| this.check_for_updates(cx))),
            ))
            .child(div().mx_5().h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!("settings.about.releases", locale = locale).into_owned(),
                t!("settings.about.releases_description", locale = locale).into_owned(),
                external_link(
                    "about-releases",
                    t!("settings.about.view_releases", locale = locale).into_owned(),
                    RELEASES_URL,
                    &theme,
                ),
            ))
    }
}

pub(super) fn update_description(status: &UpdateStatus, language: Language) -> String {
    let locale = language.locale();
    match status {
        UpdateStatus::Unavailable if cfg!(target_os = "macos") => {
            t!("settings.about.updates_unavailable", locale = locale).into_owned()
        }
        UpdateStatus::Unavailable => {
            t!("settings.about.updates_unsupported", locale = locale).into_owned()
        }
        UpdateStatus::Ready => t!("settings.about.updates_ready", locale = locale).into_owned(),
        UpdateStatus::Checking => {
            t!("settings.about.updates_checking", locale = locale).into_owned()
        }
        UpdateStatus::Available(version) => t!(
            "settings.about.updates_available",
            locale = locale,
            version = version
        )
        .into_owned(),
        UpdateStatus::NoUpdate(reason) => reason.clone(),
        UpdateStatus::Failed(error) => t!(
            "settings.about.updates_failed",
            locale = locale,
            error = error
        )
        .into_owned(),
    }
}

fn external_link(id: &'static str, label: String, url: &'static str, theme: &Theme) -> Link {
    let color = theme.link;
    let focus = theme.accent;
    let link = Link::new(id)
        .href(url)
        .accessibility_label(label.clone())
        .open_with(|url, _, _, cx| cx.open_url(url))
        .text_sm()
        .text_color(color)
        .px_2()
        .py_1()
        .underline()
        .hover(move |link| link.text_color(color.opacity(0.8)))
        .focus_visible(move |link| link.bg(focus))
        .child(label);
    #[cfg(test)]
    let link = link.debug_selector(move || id.into());
    link
}
