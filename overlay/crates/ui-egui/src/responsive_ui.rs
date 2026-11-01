//! FigCraft Plus: responsive layout in the Design panel (`figcraft_engine::responsive`) and the key
//! object on the canvas (`figcraft_engine::key_object`).
//!
//! - **% / vw / vh sizes.** The W / H sizing menu (Fixed, Hug, Fill) adds *Percent of parent* and
//!   *Viewport (vw / vh)*; the menu then reads `50%` or `100vw`. In W / H, `100vw` or `50vh` typed
//!   with the number makes the side relative to the frame the layer is in; on a side that is a
//!   percentage already, `60%` changes the share (elsewhere `%` keeps FigCraft's meaning: a share
//!   of the value there now). A plain size, or Fixed, ends it.
//! - **Auto layout from variables.** The variable button at the end of the direction row opens
//!   *Layout from variables*: direction, wrap, alignment and the W / H sizing, each following a
//!   string (or boolean) variable — a mode switches them.
//! - **Key object.** The layer clicked last in a multi-selection is outlined thicker: the align
//!   buttons move the others to it.

use std::cell::Cell;

use egui::{Id, Painter, Rect, Stroke, Ui, pos2, vec2};
use figcraft_doc::{Document, NodeId, VariableType};
use figcraft_engine::responsive::{self, Unit};
use serde_json::json;

use crate::FigApp;
use crate::i18n::{tr, tr_noop};
use crate::icons::{self, Icon};
use crate::theme::{Tokens, regular, semibold};
use crate::widgets;

/// The W / H field being drawn: its axis, what its shares are of, and whether `%` is its unit.
#[derive(Clone, Copy, Debug)]
struct Field {
    horizontal: bool,
    /// The parent's inner box and the viewport frame (px), for the first selected layer.
    parent: Option<(f64, f64)>,
    viewport: Option<(f64, f64)>,
    percent: bool,
}

thread_local! {
    static FIELD: Cell<Option<Field>> = const { Cell::new(None) };
    /// A share typed this frame: (horizontal, value, unit).
    static PENDING: Cell<Option<(bool, f64, Unit)>> = const { Cell::new(None) };
}

/// Before the W (`horizontal`) or H field of `sel` draws.
pub(crate) fn field_begin(doc: &Document, sel: &[NodeId], horizontal: bool) {
    let first = sel.first().copied();
    let field = first.map(|id| Field {
        horizontal,
        parent: responsive::reference(doc, id, Unit::Percent),
        viewport: responsive::reference(doc, id, Unit::Vw),
        percent: sel.iter().all(|i| responsive::size_rule(doc, *i, horizontal).is_some_and(|(_, u)| u == Unit::Percent)),
    });
    FIELD.with(|f| f.set(field));
}

/// After the field drew.
pub(crate) fn field_end() {
    FIELD.with(|f| f.set(None));
}

/// Whether `%` is a unit in the field being typed into (its side is a percentage already).
pub(crate) fn percent_typing() -> bool {
    FIELD.with(Cell::get).is_some_and(|f| f.percent)
}

/// A number typed with `unit` in a W / H field: its px, and the share noted for the field (None:
/// not a responsive unit, or not a W / H field).
pub(crate) fn unit(v: f64, unit: &str) -> Option<f64> {
    let f = FIELD.with(Cell::get)?;
    let u = Unit::parse(unit)?;
    let (w, h) = match u {
        Unit::Percent => f.parent?,
        Unit::Vw | Unit::Vh => f.viewport?,
    };
    let base = match u {
        Unit::Percent => {
            if f.horizontal {
                w
            } else {
                h
            }
        }
        Unit::Vw => w,
        Unit::Vh => h,
    };
    PENDING.with(|p| p.set(Some((f.horizontal, v, u))));
    Some(base * v / 100.0)
}

/// The W (`horizontal`) or H field committed (`edited`): when a share was typed, make the side
/// relative (true: the field's own px edit is not needed).
pub(crate) fn after_field(app: &mut FigApp, ids: &[NodeId], horizontal: bool, edited: bool) -> bool {
    let pending = PENDING.with(|p| p.take());
    let Some((h, value, unit)) = pending.filter(|(h, _, _)| *h == horizontal && edited) else { return false };
    let _ = h;
    app.run("layout.relative", json!({"ids": ids, "axis": if horizontal { "width" } else { "height" }, "value": value, "unit": unit.suffix()}))
        .is_some()
}

/// What the sizing menu reads for side `horizontal` of `sel`: the share, when every layer has the
/// same one; else FigCraft's `label`.
pub(crate) fn sizing_label<'a>(doc: &Document, sel: &[NodeId], horizontal: bool, label: &'a str) -> std::borrow::Cow<'a, str> {
    let rules: Vec<Option<(f64, Unit)>> = sel.iter().map(|i| responsive::size_rule(doc, *i, horizontal)).collect();
    match rules.first() {
        Some(Some((v, u))) if rules.iter().all(|r| *r == rules[0]) => format!("{}{}", widgets::fmt_num(*v), u.suffix()).into(),
        _ => label.into(),
    }
}

/// The sizing menu's responsive items (after FigCraft's Fixed / Hug / Fill).
pub(crate) fn sizing_menu(app: &mut FigApp, ui: &mut Ui, doc: &Document, ids: &[NodeId], horizontal: bool) {
    let has_parent = ids.iter().all(|i| responsive::reference(doc, *i, Unit::Percent).is_some());
    let has_view = ids.iter().all(|i| responsive::reference(doc, *i, Unit::Vw).is_some());
    let now = ids.first().and_then(|i| responsive::size_rule(doc, *i, horizontal)).map(|(_, u)| u);
    if !(has_parent || has_view) {
        return;
    }
    ui.separator();
    let axis = if horizontal { "width" } else { "height" };
    let view = if horizontal { Unit::Vw } else { Unit::Vh };
    let items = [
        (has_parent, Unit::Percent, tr("Percent of parent (%)")),
        (has_view, view, if horizontal { tr("Viewport width (vw)") } else { tr("Viewport height (vh)") }),
    ];
    for (ok, unit, label) in items {
        if ok && crate::right::menu_check(ui, now == Some(unit), label, "") {
            app.run("layout.relative", json!({"ids": ids, "axis": axis, "unit": unit.suffix()}));
            ui.close();
        }
    }
}

/// FigCraft's Fixed / Hug / Fill was picked for side `horizontal`: the side stops being relative.
pub(crate) fn clear(app: &mut FigApp, doc: &Document, ids: &[NodeId], horizontal: bool) {
    if ids.iter().any(|i| responsive::size_rule(doc, *i, horizontal).is_some()) {
        app.run("layout.relative", json!({"ids": ids, "axis": if horizontal { "width" } else { "height" }}));
    }
}

// ---- auto layout from variables ------------------------------------------------------------------

fn popover_id() -> Id {
    Id::unique("figcraftplus_layout_vars")
}

/// Labels of the fields a variable can drive.
const FIELDS: [(&str, &str); 6] = [
    ("direction", tr_noop("Direction")),
    ("wrap", tr_noop("Wrap")),
    ("mainAlign", tr_noop("Main alignment")),
    ("crossAlign", tr_noop("Cross alignment")),
    ("sizingH", tr_noop("Width sizing")),
    ("sizingV", tr_noop("Height sizing")),
];

/// The variable button at the end of the direction row (`ids`: the auto layout frames).
pub(crate) fn vars_button(ui: &mut Ui, doc: &Document, ids: &[NodeId]) {
    let bound = ids.iter().any(|i| FIELDS.iter().any(|(f, _)| responsive::var_rule(doc, *i, f).is_some()));
    let resp = icons::button(ui, Icon::Variable, 24.0, bound, tr("Layout from variables"));
    if resp.clicked() {
        let open = ui.data(|d| d.get_temp::<egui::Pos2>(popover_id())).is_some();
        // Beside the Design panel, level with the button (as FigCraft's popovers open).
        let at = pos2(ui.max_rect().left() - 300.0 - 24.0, resp.rect.top() - 8.0);
        ui.data_mut(|d| {
            if open {
                d.remove_temp::<egui::Pos2>(popover_id());
            } else {
                d.insert_temp(popover_id(), at);
            }
        });
        if open {
            crate::popover_ui::forget(popover_id().with("area"));
        }
    }
    ui.data_mut(|d| d.insert_temp(popover_id().with("button"), resp.rect));
}

/// The *Layout from variables* popover, while open (once a frame).
pub(crate) fn vars_popover(app: &mut FigApp, ctx: &egui::Context) {
    let Some(at) = ctx.data(|d| d.get_temp::<egui::Pos2>(popover_id())) else { return };
    // Once a frame, however many times the panel is laid out.
    let frame = ctx.cumulative_frame_nr();
    if ctx.data(|d| d.get_temp::<u64>(popover_id().with("drawn"))) == Some(frame) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(popover_id().with("drawn"), frame));
    let Ok(ds) = app.session.ds() else { return };
    let doc = ds.doc.clone();
    let ids: Vec<NodeId> = ds.selection.iter().copied().filter(|i| doc.node(*i).is_some_and(|n| n.layout.is_some())).collect();
    if ids.is_empty() {
        ctx.data_mut(|d| d.remove_temp::<egui::Pos2>(popover_id()));
        crate::popover_ui::forget(popover_id().with("area"));
        return;
    }
    let t = Tokens::get(ctx);
    let vars: Vec<(String, VariableType)> = doc
        .variables
        .variables
        .iter()
        .filter(|v| matches!(v.kind, VariableType::String | VariableType::Boolean))
        .map(|v| (v.name.clone(), v.kind))
        .collect();
    let mut set: Option<(&str, Option<String>)> = None;
    let (closed, rect) = crate::popover_ui::show(ctx, popover_id().with("area"), tr("Layout from variables"), 300.0, crate::popover::Open::At(at), |ui| {
        egui::Frame::NONE.inner_margin(egui::Margin { left: 16, right: 16, top: 12, bottom: 14 }).show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
            ui.label(
                egui::RichText::new(tr("Each follows a string or boolean variable in the frame's modes: row / column, wrap, start / center / end / space-between, hug / fill / fixed."))
                    .font(regular(10.5))
                    .color(t.text_secondary),
            );
            for (field, label) in FIELDS {
                // Each row's own ids: the dropdowns may read the same ("None").
                ui.push_id(field, |ui| ui.horizontal(|ui| {
                    ui.add_sized([110.0, 24.0], egui::Label::new(egui::RichText::new(tr(label)).font(regular(11.0)).color(t.text)));
                    let now = ids.first().and_then(|i| responsive::var_rule(&doc, *i, field));
                    let shown = now.clone().unwrap_or_else(|| tr("None").to_string());
                    let resp = widgets::dropdown(ui, &shown, ui.available_width(), if now.is_some() { Some(Icon::Variable) } else { None });
                    crate::right::menu_popup(&resp).show(|ui| {
                        if crate::right::menu_check(ui, now.is_none(), tr("None"), "") {
                            set = Some((field, None));
                            ui.close();
                        }
                        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                            for (name, _) in &vars {
                                if crate::right::menu_check(ui, now.as_deref() == Some(name.as_str()), name, "") {
                                    set = Some((field, Some(name.clone())));
                                    ui.close();
                                }
                            }
                        });
                    });
                }));
            }
            if vars.is_empty() {
                ui.label(egui::RichText::new(tr("No string or boolean variables in this file yet.")).font(semibold(10.5)).color(t.text_tertiary));
            }
        });
    });
    if let Some((field, var)) = set {
        app.run("layout.bindVariable", json!({"ids": ids, "field": field, "variable": var}));
    }
    let button = ctx.data(|d| d.get_temp::<Rect>(popover_id().with("button")));
    let outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !rect.contains(p) && !button.is_some_and(|b| b.contains(p))))
        && !egui::Popup::is_any_open(ctx)
        && rect.is_positive();
    if closed || outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        ctx.data_mut(|d| d.remove_temp::<egui::Pos2>(popover_id()));
        crate::popover_ui::forget(popover_id().with("area"));
    }
}

// ---- key object -----------------------------------------------------------------------------------

/// The key object's outline (over the selection's), shifted like the selection's by `shift`.
pub(crate) fn key_outline(app: &FigApp, painter: &Painter, doc: &Document, sel: &[NodeId], shift: egui::Vec2) {
    let Some(k) = figcraft_engine::key_object::key_in(sel) else { return };
    let Some(b) = doc.world_bounds(k) else { return };
    let r = Rect::from_two_pos(app.to_screen(b.x0, b.y0), app.to_screen(b.x1, b.y1)).translate(shift);
    let col = Tokens::get(painter.ctx()).accent;
    painter.rect_stroke(r.expand(1.0), egui::CornerRadius::ZERO, Stroke::new(3.0, col), egui::StrokeKind::Outside);
}

/// The key object preference, for the engine (each frame).
pub(crate) fn tick() {
    figcraft_engine::key_object::set_enabled(crate::prefs_plus_ui::get().key_object);
}

/// W / H responsive sizing for layers that have no FigCraft sizing menu (outside auto layout).
pub(crate) fn relative_row(app: &mut FigApp, ui: &mut Ui, doc: &Document, sel: &[NodeId], col: f32) {
    if !sel.iter().all(|i| responsive::reference(doc, *i, Unit::Percent).is_some() || responsive::reference(doc, *i, Unit::Vw).is_some()) {
        return;
    }
    let ids = sel.to_vec();
    crate::right::row(ui, 32.0, |ui| {
        for horizontal in [true, false] {
            let label = sizing_label(doc, sel, horizontal, tr("Fixed"));
            let resp = ui.push_id(("figcraftplus_relative", horizontal), |ui| widgets::dropdown(ui, &label, col, Some(Icon::Fixed))).inner;
            crate::right::menu_popup(&resp).width(200.0).show(|ui| {
                let relative = ids.iter().any(|i| responsive::size_rule(doc, *i, horizontal).is_some());
                if crate::right::menu_check(ui, !relative, tr("Fixed"), "") {
                    clear(app, doc, &ids, horizontal);
                    ui.close();
                }
                sizing_menu(app, ui, doc, &ids, horizontal);
            });
        }
    });
}
