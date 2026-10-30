//! FigCraft Plus: one look for every popover of the Design panel.
//!
//! A popover is a rounded card (12 points) on the panel's colour with a 1-point border and a soft
//! shadow; a 40-point header holds its title on the left and a close button on the right, with a
//! hairline under it; the body is inset 16 points on the sides. The colour picker, the variable
//! list, the style menu (Fill, Stroke, Effects, Text styles: light in the light theme), the Units
//! settings and the style dialogs all use it. Dropdown menus stay FigCraft's dark menus.

use std::cell::Cell;

use egui::{Align2, CornerRadius, Id, Rect, Sense, Stroke, Ui, pos2, vec2};

use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::theme::{Tokens, semibold};

/// Corner radius of a popover.
pub(crate) const RADIUS: u8 = 12;
/// Height of a popover's header.
pub(crate) const HEADER_H: f32 = 40.0;
/// Inset of a popover's body from its sides.
pub(crate) const INSET: i8 = 16;

/// The popover card: fill, border, radius and shadow (no margin: the header spans it).
pub(crate) fn frame(t: &Tokens) -> egui::Frame {
    egui::Frame::NONE
        .fill(t.panel)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(CornerRadius::same(RADIUS))
        .shadow(egui::Shadow { offset: [0, 6], blur: 20, spread: 0, color: t.shadow })
}

/// [`frame`] with the body's margin, for popovers without a header row of their own.
pub(crate) fn padded(t: &Tokens) -> egui::Frame {
    frame(t).inner_margin(egui::Margin::same(INSET))
}

/// The header across `w`: `title` and a close button; true when the close button was clicked.
pub(crate) fn header(ui: &mut Ui, id: Id, title: &str, w: f32) -> bool {
    let t = Tokens::get(ui.ctx());
    let (hr, _) = ui.allocate_exact_size(vec2(w, HEADER_H), Sense::hover());
    ui.painter().text(pos2(hr.min.x + f32::from(INSET), hr.center().y), Align2::LEFT_CENTER, title, semibold(12.0), t.text);
    let close = close_button(ui, id, hr);
    ui.painter().hline(hr.x_range(), hr.max.y - 0.5, Stroke::new(1.0, t.border));
    close
}

/// The close button at the right of a header `hr`; true when clicked.
pub(crate) fn close_button(ui: &mut Ui, id: Id, hr: Rect) -> bool {
    let t = Tokens::get(ui.ctx());
    let xr = Rect::from_center_size(pos2(hr.max.x - 20.0, hr.center().y), vec2(24.0, 24.0));
    let resp = ui.interact(xr, id.with("close"), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tr("Close")));
    if resp.hovered() {
        ui.painter().rect_filled(xr, CornerRadius::same(5), t.hover);
    }
    icons::paint(ui.painter(), xr.shrink(6.0), Icon::Close, t.icon);
    resp.clicked()
}

/// The hairline under a header that FigCraft draws itself (`hr`: the header's rect).
pub(crate) fn divider(ui: &Ui, hr: Rect) {
    let t = Tokens::get(ui.ctx());
    ui.painter().hline(hr.x_range(), hr.max.y - 0.5, Stroke::new(1.0, t.border));
}

/// A body section inset like the header's title.
pub(crate) fn body<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::NONE.inner_margin(egui::Margin { left: INSET, right: INSET, top: 12, bottom: 14 }).show(ui, add).inner
}

// ---- closing -----------------------------------------------------------------------------------
//
// Every popover closes the same ways: its close button, Escape, a click outside it (not one in a
// menu or popover it opened), and a change of selection. (The colour picker moves to the same slot
// of the new selection instead, when it has one: see `picker_place_ui`.)

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

thread_local! {
    /// FigCraft's style menu is being drawn as a popover (see [`PanelMenus`]).
    static PANEL_MENUS: Cell<bool> = const { Cell::new(false) };
}

/// Whether menu colours are the popover's now (FigCraft's dark field scope stays off then).
pub(crate) fn panel_menus() -> bool {
    PANEL_MENUS.with(Cell::get)
}

/// While alive, FigCraft's menu colours (the style menu's dark card) are the popover's: the
/// panel, its text, hover and secondary text. Dropping it puts the theme back.
pub(crate) struct PanelMenus {
    ctx: egui::Context,
    saved: Tokens,
}

impl PanelMenus {
    pub(crate) fn begin(ctx: &egui::Context) -> Self {
        let t = Tokens::get(ctx);
        let mut m = t;
        m.menu = t.panel;
        m.menu_text = t.text;
        m.menu_hover = t.hover;
        m.menu_dim = t.text_secondary;
        let saved = crate::theme::swap_tokens(ctx, m);
        PANEL_MENUS.with(|p| p.set(true));
        PanelMenus { ctx: ctx.clone(), saved }
    }
}

impl Drop for PanelMenus {
    fn drop(&mut self) {
        crate::theme::swap_tokens(&self.ctx, self.saved);
        PANEL_MENUS.with(|p| p.set(false));
    }
}
