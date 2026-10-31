//! FigCraft Plus: the colour picker follows the selection.
//!
//! FigCraft places the picker beside the Design panel at the height of the control that opened it,
//! and closes it on Escape, a press outside it and a change of selection. Plus keeps it open
//! instead when other layers are selected and they have the same slot (Fill 1 stays Fill 1, the
//! second shadow stays the second shadow): it moves to them. When they have no such slot, or
//! nothing is selected, it closes as FigCraft's does. Opened from a style dialog, it opens beside
//! the dialog rather than over it.

use egui::{Id, pos2};
use figcraft_doc::NodeId;

use crate::FigApp;
use crate::colorpicker::PickerTarget;
use crate::popover::Open;

/// The selection the open picker follows.
fn followed_id() -> Id {
    Id::unique("figcraftplus_picker_selection")
}

/// `t` on the layers `ids` (paints and effects; other targets don't follow the selection).
fn retarget(t: &PickerTarget, ids: &[NodeId]) -> Option<PickerTarget> {
    match t {
        PickerTarget::Paint { fills, index, .. } => Some(PickerTarget::Paint { fills: *fills, index: *index, ids: ids.to_vec() }),
        PickerTarget::Effect { index, secondary, .. } => Some(PickerTarget::Effect { ids: ids.to_vec(), index: *index, secondary: *secondary }),
        _ => None,
    }
}

/// Before the picker draws: move it to the same slot of a new selection (FigCraft then closes it
/// only when that slot has no colour). `exists` tells whether a target has a colour to edit.
pub(crate) fn follow(app: &mut FigApp, ctx: &egui::Context, exists: impl Fn(&FigApp, &PickerTarget) -> bool) {
    let Some(target) = app.ui.picker.clone() else {
        ctx.data_mut(|d| d.remove::<Vec<NodeId>>(followed_id()));
        return;
    };
    let selection = app.session.selection();
    let seen: Vec<NodeId> = ctx.data(|d| d.get_temp(followed_id())).unwrap_or_else(|| selection.clone());
    if seen != selection
        && !selection.is_empty()
        && let Some(t) = retarget(&target, &selection).filter(|t| exists(app, t))
    {
        app.ui.picker = Some(t);
    }
    ctx.data_mut(|d| d.insert_temp(followed_id(), selection));
}

/// Where the picker (`w` wide) opens: beside the style dialog when its button is in one, else
/// where FigCraft opens it (`open`).
pub(crate) fn open(app: &FigApp, ctx: &egui::Context, open: Open, w: f32) -> Open {
    let Some(button) = app.ui.picker_button else { return open };
    let dialog = ctx.memory(|m| m.area_rect(Id::unique("figcraftplus_style_dialog_area"))).filter(|d| d.contains(button.center()));
    match dialog {
        Some(d) => Open::At(pos2(d.min.x - w - 8.0, button.min.y - 8.0)),
        None => open,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_follow_the_selection() {
        let a = PickerTarget::Paint { fills: true, index: 1, ids: vec![NodeId(1)] };
        let b = PickerTarget::Paint { fills: true, index: 1, ids: vec![NodeId(2), NodeId(3)] };
        assert_eq!(retarget(&a, &[NodeId(2), NodeId(3)]), Some(b));
        let fx = PickerTarget::Effect { ids: vec![NodeId(1)], index: 0, secondary: true };
        assert_eq!(retarget(&fx, &[NodeId(4)]), Some(PickerTarget::Effect { ids: vec![NodeId(4)], index: 0, secondary: true }));
        assert_eq!(retarget(&PickerTarget::PageBackground, &[NodeId(4)]), None);
    }
}
