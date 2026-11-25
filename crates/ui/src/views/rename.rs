//! The in-place rename field (A13, T2, B5): a text field laid over the clip
//! header, the track name or the preset row being renamed. Enter keeps the
//! name, Esc cancels, a click elsewhere keeps it, and on a track Tab keeps
//! it and moves on to the next track (R9).

use vizia::prelude::*;

use crate::edit::RENAME_H;
use crate::{AppEvent, Signals};

/// Wraps the field to catch Tab (the field itself moves focus with it).
struct RenameBox;

impl View for RenameBox {
    fn element(&self) -> Option<&'static str> {
        Some("rename-box")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| {
            if let WindowEvent::KeyDown(Code::Tab, _) = we {
                cx.emit(AppEvent::RenameCommit(true));
                meta.consume();
            }
        });
    }
}

pub fn build(cx: &mut Context, s: Signals) {
    Binding::new(cx, s.rename, move |cx| {
        let Some(spec) = s.rename.get() else { return };
        let text = Signal::new(spec.text.clone());
        let target = spec.target;
        RenameBox
            .build(cx, |cx| {
                Textbox::new(cx, text)
                    .class("rename-field")
                    .width(Stretch(1.0))
                    .height(Stretch(1.0))
                    .on_edit(|cx, t| cx.emit(AppEvent::RenameText(t)))
                    // Enter (true) or a click elsewhere (false) keeps the name.
                    // (For this target only: a late blur of a field that was
                    // already replaced must not rename the next item.)
                    .on_submit(move |cx, t: String, _| cx.emit(AppEvent::RenameSubmit(target, t)))
                    .on_cancel(|cx| cx.emit(AppEvent::RenameCancel))
                    .on_build(|cx| cx.emit(TextEvent::StartEdit));
            })
            .position_type(PositionType::Absolute)
            .left(Pixels(spec.x))
            .top(Pixels(spec.y))
            .width(Pixels(spec.w))
            .height(Pixels(RENAME_H));
    });
}
