use gpui::{AnyElement, App, Context, Entity, Render, WeakEntity, Window, div, prelude::*};

use super::Locus;

type PaneContent = fn(&Locus, &mut Window, &mut Context<Locus>) -> AnyElement;

pub(super) struct LocusPane {
    locus: WeakEntity<Locus>,
    content: PaneContent,
}

impl LocusPane {
    pub(super) fn new(locus: &Entity<Locus>, content: PaneContent, cx: &mut App) -> Entity<Self> {
        let locus_weak = locus.downgrade();
        cx.new(|cx| {
            cx.observe(locus, |_, _, cx| cx.notify()).detach();
            Self {
                locus: locus_weak,
                content,
            }
        })
    }
}

impl Render for LocusPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(locus) = self.locus.upgrade() else {
            return div().into_any_element();
        };
        let content = self.content;
        locus.update(cx, |locus, cx| content(locus, window, cx))
    }
}
