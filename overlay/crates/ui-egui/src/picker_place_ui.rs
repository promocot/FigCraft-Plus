//! FigCraft Plus: where the colour picker opens and when it closes.
//!
//! - It opens beside the control that opened it (a fill's swatch, a colour variable's value, a
//!   style's paint…): left of the Design panel at the swatch's height, else beside the swatch, on
//!   the other side when there's no room, always inside the window. It follows that control while
//!   panels scroll, resize or move (egui reports the clicked widget and where it is drawn).
//! - It sits above every panel and window (the Variables editor included).
//! - Selecting other layers keeps it open on the same slot of the new selection (Fill 1 stays
//!   Fill 1); when they have no such slot, or nothing is selected, it closes.
//! - A click on the canvas that doesn't select other layers (empty space, the same layer) closes
//!   it, and the click does what it always does (an empty spot deselects). Drags (gradient
//!   handles, moving layers) leave it open.

use egui::{Id, Pos2, Rect, pos2, vec2};
use figcraft_doc::NodeId;

use crate::FigApp;
use crate::colorpicker::PickerTarget;

/// The window id of the colour picker: egui names a window's area by its title text
/// (`Id::unique(title.text())`, an `Option<Cow<str>>`).
fn window_id() -> Id {
    Id::unique(Some(std::borrow::Cow::<str>::Borrowed("color_picker")))
}

#[derive(Clone, Debug, Default)]
struct Anchor {
    /// What the picker edits, without the layers (kept across a change of selection).
    slot: String,
    /// The widget clicked to open it, and where it was last drawn.
    widget: Option<Id>,
    rect: Option<Rect>,
    /// The selection the picker follows.
    selection: Vec<NodeId>,
    /// A press on the canvas that may close the picker: where, and the selection before it.
    press: Option<(Pos2, Vec<NodeId>)>,
}

fn anchor_id() -> Id {
    Id::unique("figcraftplus_picker_anchor")
}

/// The last widget clicked and the pass it was clicked in.
fn last_click_id() -> Id {
    Id::unique("figcraftplus_picker_last_click")
}

/// What `t` edits, ignoring which layers: "Paint { fills: true, index: 0 }".
fn slot_key(t: &PickerTarget) -> String {
    match t {
        PickerTarget::Paint { fills, index, .. } => format!("paint {fills} {index}"),
        PickerTarget::Effect { index, secondary, .. } => format!("effect {index} {secondary}"),
        other => format!("{other:?}"),
    }
}

/// `t` on the layers `ids` (paints and effects; other targets don't follow the selection).
fn retarget(t: &PickerTarget, ids: &[NodeId]) -> Option<PickerTarget> {
    match t {
        PickerTarget::Paint { fills, index, .. } => Some(PickerTarget::Paint { fills: *fills, index: *index, ids: ids.to_vec() }),
        PickerTarget::Effect { index, secondary, .. } => Some(PickerTarget::Effect { ids: ids.to_vec(), index: *index, secondary: *secondary }),
        _ => None,
    }
}

/// Before the picker draws: note what opened it, follow the selection and close on a click on the
/// canvas. `exists` tells whether a target has a colour to edit.
pub(crate) fn follow(app: &mut FigApp, ctx: &egui::Context, exists: impl Fn(&FigApp, &PickerTarget) -> bool) {
    // The widget clicked this frame, or the frame before (panels drawn after the picker open it a
    // frame late).
    let pass = ctx.cumulative_pass_nr();
    let now = ctx.viewport(|v| v.interact_widgets.clicked);
    let last: Option<(Id, u64)> = ctx.data(|d| d.get_temp(last_click_id()));
    let clicked = now.or(last.filter(|(_, at)| at + 1 >= pass).map(|(w, _)| w));
    if let Some(w) = now {
        ctx.data_mut(|d| d.insert_temp(last_click_id(), (w, pass)));
    }
    let Some(target) = app.ui.picker.clone() else {
        ctx.data_mut(|d| d.remove::<Anchor>(anchor_id()));
        return;
    };
    let mut a: Anchor = ctx.data(|d| d.get_temp(anchor_id())).unwrap_or_default();
    let selection = app.session.selection();
    let slot = slot_key(&target);
    if a.slot != slot {
        // Opened (or moved to another slot): anchor to the widget clicked this frame.
        let widget = clicked;
        let rect = widget
            .and_then(|w| ctx.read_response(w))
            .map(|r| r.rect)
            .or_else(|| ctx.input(|i| i.pointer.interact_pos()).map(|p| Rect::from_center_size(p, vec2(1.0, 1.0))));
        a = Anchor { slot, widget, rect, selection: selection.clone(), press: None };
    } else if let Some(r) = a.widget.and_then(|w| ctx.read_response(w)) {
        a.rect = Some(r.rect);
    }
    // A press on the canvas, outside the picker and anything it opened, may close it.
    let (pressed, released, pos) = ctx.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_released(), i.pointer.interact_pos()));
    if pressed && let Some(p) = pos {
        // The picker, its menus and the variable list sit in the foreground, above panels, the
        // canvas and the Variables editor.
        let on_overlay = ctx.layer_id_at(p).is_some_and(|l| l.order >= egui::Order::Foreground);
        let on_picker = ctx.memory(|m| m.area_rect(window_id())).is_some_and(|r| r.contains(p));
        let on_list = crate::variable_ui::picker_rect(ctx).is_some_and(|r| r.contains(p));
        let on_anchor = a.rect.is_some_and(|r| r.expand(2.0).contains(p));
        // The eyedropper's click on the canvas samples a colour for the picker: it keeps it open.
        // (The canvas may already have taken the sample this frame and gone back to Move.)
        let sampling = app.session.tool.tool == figcraft_engine::ToolId::Eyedropper || app.session.tool.picked.is_some();
        if app.canvas.rect.contains(p) && !sampling && !on_overlay && !on_picker && !on_list && !on_anchor && !egui::Popup::is_any_open(ctx) {
            a.press = Some((p, a.selection.clone()));
        }
    }
    // Follow the selection: the same slot on other layers, or close.
    let mut close = false;
    if a.selection != selection && retarget(&target, &[]).is_some() {
        match retarget(&target, &selection).filter(|t| !selection.is_empty() && exists(app, t)) {
            Some(t) => app.ui.picker = Some(t),
            None => close = true,
        }
        a.selection = selection.clone();
    }
    if released && let Some((origin, before)) = a.press.take() {
        let click = pos.is_none_or(|p| p.distance(origin) < 6.0);
        // A click that selected other layers keeps the picker on them; any other click closes it.
        let reselected = retarget(&target, &[]).is_some() && !selection.is_empty() && selection != before;
        if click && !reselected {
            close = true;
        }
    }
    if close {
        app.ui.picker = None;
        ctx.data_mut(|d| d.remove::<Anchor>(anchor_id()));
    } else {
        ctx.data_mut(|d| d.insert_temp(anchor_id(), a));
    }
}

/// Where the picker (`w` wide) goes: beside `beside` when given (a plugin's window), else by its
/// anchor — left of the Design panel at the anchor's height when the anchor is in it, else left of
/// the anchor or right of it when there's no room — kept inside the window.
pub(crate) fn position(app: &FigApp, ctx: &egui::Context, w: f32, beside: Option<Pos2>) -> Pos2 {
    let screen = ctx.content_rect();
    let h = ctx.memory(|m| m.area_rect(window_id())).map_or(460.0, |r| r.height());
    let right_panel = screen.max.x - app.ui.right_width;
    let a: Option<Rect> = ctx.data(|d| d.get_temp::<Anchor>(anchor_id())).and_then(|a| a.rect);
    let want = match (beside, a) {
        (Some(p), _) => p,
        (None, Some(r)) => {
            // Opened from the style dialog: beside the dialog, not over it.
            let dialog = ctx.memory(|m| m.area_rect(Id::unique("figcraftplus_style_dialog_area"))).filter(|d| d.contains(r.center()));
            let x = if let Some(d) = dialog {
                d.min.x - w - 8.0
            } else if r.min.x >= right_panel - 1.0 {
                right_panel - w - 12.0
            } else if r.min.x - w - 8.0 >= screen.min.x + 8.0 {
                r.min.x - w - 8.0
            } else {
                r.max.x + 8.0
            };
            pos2(x, r.min.y - 8.0)
        }
        (None, None) => pos2(right_panel - w - 20.0, 140.0),
    };
    pos2(clamp(want.x, screen.min.x + 8.0, screen.max.x - w - 8.0), clamp(want.y, screen.min.y + 8.0, screen.max.y - h - 8.0))
}

/// `v` within `lo..=hi` (`lo` when the range is empty).
fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    v.min(hi).max(lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_ignore_the_layers_and_follow_the_selection() {
        let a = PickerTarget::Paint { fills: true, index: 1, ids: vec![NodeId(1)] };
        let b = PickerTarget::Paint { fills: true, index: 1, ids: vec![NodeId(2), NodeId(3)] };
        assert_eq!(slot_key(&a), slot_key(&b));
        assert_ne!(slot_key(&a), slot_key(&PickerTarget::Paint { fills: false, index: 1, ids: vec![NodeId(1)] }));
        assert_eq!(retarget(&a, &[NodeId(2), NodeId(3)]), Some(b));
        let fx = PickerTarget::Effect { ids: vec![NodeId(1)], index: 0, secondary: true };
        assert_eq!(retarget(&fx, &[NodeId(4)]), Some(PickerTarget::Effect { ids: vec![NodeId(4)], index: 0, secondary: true }));
        assert_eq!(retarget(&PickerTarget::PageBackground, &[NodeId(4)]), None);
        assert_eq!(clamp(5.0, 8.0, 100.0), 8.0);
        assert_eq!(clamp(500.0, 8.0, 100.0), 100.0);
        assert_eq!(clamp(50.0, 8.0, 4.0), 8.0);
    }

    #[test]
    fn the_window_id_is_the_one_egui_gives_the_picker() {
        let ctx = egui::Context::default();
        let mut id = None;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            id = egui::Window::new("color_picker").show(ui.ctx(), |_| {}).map(|r| r.response.layer_id.id);
        })
        .textures_delta
        .clear();
        assert_eq!(id, Some(window_id()));
    }
}
