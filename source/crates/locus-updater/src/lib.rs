//! Shared product update service; Sparkle owns downloading and installation.
use gpui_kit::{App, AppContext, Context, Entity, Global, Task};

#[cfg(target_os = "macos")]
mod macos;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateStatus {
    Unavailable,
    Ready,
    Checking,
    Available(String),
    NoUpdate(String),
    Failed(String),
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug)]
enum UpdateEvent {
    Started,
    Found(String),
    NoUpdate(String),
    Finished(Option<String>),
}

struct SharedUpdater(Entity<Updater>);
impl Global for SharedUpdater {}

pub struct Updater {
    status: UpdateStatus,
    #[cfg(target_os = "macos")]
    backend: Option<macos::Backend>,
    _events: Option<Task<()>>,
}

impl Updater {
    /// One app-wide entity keeps native state and results alive across settings visits.
    pub fn shared(cx: &mut App) -> Entity<Self> {
        if let Some(shared) = cx.try_global::<SharedUpdater>() {
            return shared.0.clone();
        }
        let updater = cx.new(Self::new);
        cx.set_global(SharedUpdater(updater.clone()));
        updater
    }

    fn new(cx: &mut Context<Self>) -> Self {
        #[cfg(target_os = "macos")]
        {
            let (sender, receiver) = async_channel::unbounded();
            let (backend, status) = match macos::Backend::new(sender) {
                Ok(Some(backend)) => (Some(backend), UpdateStatus::Ready),
                Ok(None) => (None, UpdateStatus::Unavailable),
                Err(error) => (None, UpdateStatus::Failed(error)),
            };
            let events = backend.as_ref().map(|_| {
                cx.spawn(async move |this, cx| {
                    while let Ok(event) = receiver.recv().await {
                        if this
                            .update(cx, |this, cx| {
                                this.apply_event(event);
                                // Finishing a native session can re-enable the button even
                                // when its result text is unchanged.
                                cx.notify();
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
            });
            Self {
                status,
                backend,
                _events: events,
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = cx;
            Self {
                status: UpdateStatus::Unavailable,
                _events: None,
            }
        }
    }

    pub fn status(&self) -> &UpdateStatus {
        &self.status
    }

    pub fn is_check_enabled(&self) -> bool {
        if self.status == UpdateStatus::Checking {
            return false;
        }
        #[cfg(target_os = "macos")]
        {
            self.backend
                .as_ref()
                .is_some_and(macos::Backend::is_check_enabled)
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    pub fn check(&mut self, cx: &mut Context<Self>) {
        if !self.is_check_enabled() {
            return;
        }
        self.status = UpdateStatus::Checking;
        cx.notify();
        #[cfg(target_os = "macos")]
        if let Some(backend) = &self.backend {
            backend.check();
        }
    }

    #[cfg(any(target_os = "macos", test))]
    fn apply_event(&mut self, event: UpdateEvent) {
        match event {
            UpdateEvent::Started => self.status = UpdateStatus::Checking,
            UpdateEvent::Found(version) => self.status = UpdateStatus::Available(version),
            UpdateEvent::NoUpdate(reason) => self.status = UpdateStatus::NoUpdate(reason),
            UpdateEvent::Finished(Some(error)) => self.status = UpdateStatus::Failed(error),
            UpdateEvent::Finished(None) if self.status == UpdateStatus::Checking => {
                // Cancellation is not proof that the current version is latest.
                self.status = UpdateStatus::Ready;
            }
            UpdateEvent::Finished(None) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui_kit::test]
    fn unavailable_builds_do_not_claim_to_be_current(cx: &mut gpui_kit::TestAppContext) {
        cx.update(|cx| {
            let updater = Updater::shared(cx);
            assert_eq!(updater, Updater::shared(cx));
            updater.update(cx, |updater, cx| {
                assert_eq!(updater.status(), &UpdateStatus::Unavailable);
                assert!(!updater.is_check_enabled());
                updater.check(cx);
                assert_eq!(updater.status(), &UpdateStatus::Unavailable);
            });
        });
    }

    #[gpui_kit::test]
    fn native_results_and_cancellation_preserve_truthful_status(cx: &mut gpui_kit::TestAppContext) {
        cx.update(|cx| {
            Updater::shared(cx).update(cx, |updater, _| {
                updater.apply_event(UpdateEvent::Started);
                assert_eq!(updater.status, UpdateStatus::Checking);
                assert!(!updater.is_check_enabled());
                updater.apply_event(UpdateEvent::Finished(None));
                assert_eq!(updater.status, UpdateStatus::Ready);
                updater.apply_event(UpdateEvent::NoUpdate("Requires a newer macOS".into()));
                updater.apply_event(UpdateEvent::Finished(None));
                assert_eq!(
                    updater.status,
                    UpdateStatus::NoUpdate("Requires a newer macOS".into())
                );
                updater.apply_event(UpdateEvent::Found("0.2026.2".into()));
                updater.apply_event(UpdateEvent::Finished(None));
                assert_eq!(updater.status, UpdateStatus::Available("0.2026.2".into()));
                updater.apply_event(UpdateEvent::Finished(Some("Network unavailable".into())));
                assert_eq!(
                    updater.status,
                    UpdateStatus::Failed("Network unavailable".into())
                );
            });
        });
    }
}
