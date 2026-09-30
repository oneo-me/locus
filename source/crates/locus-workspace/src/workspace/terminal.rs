use super::*;
use gpui_kit::component::{WindowExt as _, notification::Notification};

impl Locus {
    pub(super) fn new_terminal_action(
        &mut self,
        _: &NewTerminal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.terminal_launch_pending {
            return;
        }
        let preferences = self.settings.read(cx).preferences();
        let Some(root) = preferences.library_path.clone() else {
            return;
        };
        let terminal = preferences.terminal;
        let agent = preferences.agent;
        self.terminal_launch_pending = true;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_spawn(
                    async move { locus_ui::platform::open_agent(&root, terminal, agent) },
                )
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.terminal_launch_pending = false;
                if let Err(error) = result {
                    let language = this.language(cx);
                    let message = t!(
                        "workspace.terminal.launch_error",
                        locale = language.locale()
                    )
                    .into_owned();
                    window
                        .push_notification(Notification::error(format!("{message}\n{error}")), cx);
                }
                cx.notify();
            });
        })
        .detach();
    }
}
