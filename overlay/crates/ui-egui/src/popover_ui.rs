//! FigCraft Plus: our popovers on FigCraft's own popover ([`crate::popover::Popover`]).
//!
//! FigCraft draws its detached panels — the colour picker, the style picker — as one card with a
//! header (title, close button, a hairline under it) and the theme's popup shadow. The variable
//! list, the Units settings and the style dialogs use that same card: [`show`] keeps one
//! `Popover` per id from frame to frame (it remembers where it was dragged) and [`forget`] drops it
//! when it closes. The small settings card beside a style dialog, which has no header, gets the
//! same surface from [`frame`].
//!
//! Closing is the same for all of them: the close button, Escape, a press outside and a change of
//! selection ([`selection_changed`]). (The colour picker moves to the same slot of the new selection
//! instead, when it has one: see `picker_place_ui`.)

use std::cell::RefCell;
use std::collections::HashMap;

use egui::{CornerRadius, Id, Rect, Ui};

use crate::popover::{Hold, Open, Popover};
use crate::theme::Tokens;

/// FigCraft's popover corner radius.
pub(crate) const RADIUS: u8 = 13;

thread_local! {
    static OPEN: RefCell<HashMap<Id, Popover>> = RefCell::new(HashMap::new());
}

/// Draws popover `id` (its egui area) titled `title`, `width` wide, its slot unpadded; true when
/// its close button was clicked. Our callers close it on a press outside and on Escape themselves
/// (they know which presses belong to them), so the popover leaves those to them.
pub(crate) fn show(ctx: &egui::Context, id: Id, title: &str, width: f32, open: Open, body: impl FnOnce(&mut Ui)) -> (bool, Rect) {
    let mut pop = OPEN.with(|o| o.borrow_mut().remove(&id)).unwrap_or_else(|| Popover::with_id(id, title).pad(0.0).close_on_outside(false));
    pop = pop.width(width);
    title.clone_into(&mut pop.title);
    let closed = pop.show(ctx, open, Hold { rects: &[], escape: true }, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        body(ui);
    });
    // Above what opened it (the colour picker, a style dialog), as a popover opened from it is.
    ctx.move_to_top(egui::LayerId::new(egui::Order::Foreground, id));
    let rect = pop.rect;
    OPEN.with(|o| o.borrow_mut().insert(id, pop));
    (closed, rect)
}

/// Popover `id` closed: the next [`show`] opens it afresh.
pub(crate) fn forget(id: Id) {
    OPEN.with(|o| o.borrow_mut().remove(&id));
}

/// Where popover `id` was drawn last (None while closed).
pub(crate) fn rect(id: Id) -> Option<Rect> {
    OPEN.with(|o| o.borrow().get(&id).map(|p| p.rect)).filter(|r| r.is_positive())
}

/// Where every open Plus popover is: a press in one of them is not a press outside the colour
/// picker (the variable list for a channel, a style dialog the picker was opened from).
pub(crate) fn open_rects() -> Vec<Rect> {
    OPEN.with(|o| o.borrow().values().map(|p| p.rect).filter(|r| r.is_positive()).collect())
}

/// FigCraft's popover surface without a header: the panel's fill, its radius, the theme's popup
/// shadow.
pub(crate) fn frame(ui: &Ui) -> egui::Frame {
    let t = Tokens::get(ui.ctx());
    egui::Frame::NONE.fill(t.panel).corner_radius(CornerRadius::same(RADIUS)).shadow(ui.ctx().global_style().visuals.popup_shadow)
}

/// Whether the selection changed since popover `id` opened (call it every frame the popover is
/// open; a popover not seen the frame before counts as just opened).
pub(crate) fn selection_changed(ctx: &egui::Context, id: Id, selection: &[figcraft_doc::NodeId]) -> bool {
    let key = id.with("selection");
    let now = ctx.cumulative_frame_nr();
    let seen: Option<(Vec<figcraft_doc::NodeId>, u64)> = ctx.data(|d| d.get_temp(key));
    let changed = matches!(&seen, Some((s, at)) if at + 1 >= now && s != selection);
    let keep = match seen {
        Some((s, at)) if at + 1 >= now => s,
        _ => selection.to_vec(),
    };
    ctx.data_mut(|d| d.insert_temp(key, (keep, now)));
    changed
}

/// Popover `id` closed: forget the selection it opened on.
pub(crate) fn forget_selection(ctx: &egui::Context, id: Id) {
    ctx.data_mut(|d| d.remove::<(Vec<figcraft_doc::NodeId>, u64)>(id.with("selection")));
}
