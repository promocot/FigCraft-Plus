//! FigCraft Plus colour picker additions (kept in this one file so FigCraft updates touch it rarely):
//!
//! - the OKLCH model (L 0-1, C 0-0.4, H degrees, as oklch.com and CSS write them), read and written
//!   in the document's colour profile and gamut-mapped into it, with an OKLCH plane and hue strip
//!   in place of the HSB square and rainbow;
//! - the RGB, HSL, HSB and OKLCH rows drawn here, with captions, where every channel and alpha can
//!   be bound to a number variable (the variable button on hover, as in FigCraft's number fields);
//!   the binding is a formula (`figcraft_engine::oklch_links`) kept with whatever the colour
//!   belongs to: a layer's fill, stroke, gradient stop or effect, a page background, a colour
//!   style or a colour variable;
//! - a picker that widens to fit the chosen model and can be dragged wider by its left edge.

use egui::{Align2, Color32, Id, Mesh, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use figcraft_color::{Color, ColorProfile, Hsb, OKLCH_FULL_CHROMA, Oklch};
use figcraft_doc::{NodeId, VariableType};
use figcraft_engine::oklch_links::{self, Channel, Formula, Model};
use serde_json::json;

use crate::FigApp;
use crate::colorpicker::PickerTarget;
use crate::theme::{Tokens, doc_c32, regular};
use crate::widgets::{self, BindEvent};

/// The model dropdown's width (wide enough for "OKLCH").
pub(crate) const MODEL_DROPDOWN_W: f32 = 64.0;
/// A channel field's width (room for a bound variable's name).
const FIELD_W: f32 = 60.0;
/// The OKLCH L and C fields' width (fits "0.2197" and the variable button).
const OKLCH_FIELD_W: f32 = 64.0;
/// The picker's narrowest width (FigCraft's own).
const BASE_W: f32 = 240.0;
const MAX_W: f32 = 640.0;
const GAP: f32 = 4.0;
const ALPHA_W: f32 = 60.0;
/// In the picker's own rows: what draws RGB, HSL, HSB and OKLCH (FigCraft's own rows for them are skipped).
pub(crate) const OWN_ROWS: bool = true;

fn model_index(ctx: &egui::Context) -> usize {
    ctx.data(|d| d.get_temp::<usize>(Id::new("cp_model"))).unwrap_or(0)
}

/// The value fields a model shows: (caption, width) each, alpha last when it has one.
fn columns(model: &str) -> Vec<(&'static str, f32)> {
    match model {
        "RGB" => vec![("R", FIELD_W), ("G", FIELD_W), ("B", FIELD_W), ("A %", ALPHA_W)],
        "HSL" => vec![("H °", FIELD_W), ("S %", FIELD_W), ("L %", FIELD_W), ("A %", ALPHA_W)],
        "HSB" => vec![("H °", FIELD_W), ("S %", FIELD_W), ("B %", FIELD_W), ("A %", ALPHA_W)],
        "OKLCH" => vec![("L", OKLCH_FIELD_W), ("C", OKLCH_FIELD_W), ("H °", FIELD_W), ("A %", ALPHA_W)],
        "CSS" => vec![("CSS", 152.0)],
        _ => vec![("Hex", 101.0), ("A %", 44.0)],
    }
}

/// The width a model's row needs.
fn needed(model: &str) -> f32 {
    let cols = columns(model);
    let fields: f32 = cols.iter().map(|(_, w)| *w).sum::<f32>() + GAP * cols.len().saturating_sub(1) as f32;
    16.0 + MODEL_DROPDOWN_W + GAP + fields + 16.0
}

fn width_id() -> Id {
    Id::new("cp_width")
}

/// The picker's width now: what the user dragged it to, at least what the chosen model needs.
pub(crate) fn picker_width(ctx: &egui::Context, models: &[&str]) -> f32 {
    let model = models.get(model_index(ctx)).copied().unwrap_or("Hex");
    let chosen = ctx.data_mut(|d| d.get_persisted::<f32>(width_id())).unwrap_or(BASE_W);
    chosen.max(needed(model)).max(BASE_W).min(MAX_W)
}

/// A handle along the picker's left edge: dragging it left widens the picker (it stays anchored on
/// its right). Call last inside the picker window, so it wins over the content under it.
pub(crate) fn resize_handle(ui: &mut Ui, width: f32) {
    let r = ui.min_rect();
    let strip = Rect::from_min_max(pos2(r.min.x, r.min.y), pos2(r.min.x + 6.0, r.max.y));
    let resp = ui.interact(strip, Id::new("cp_resize"), Sense::drag());
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    if resp.dragged() {
        let w = (width - resp.drag_delta().x).clamp(BASE_W, MAX_W);
        ui.ctx().data_mut(|d| d.insert_persisted(width_id(), w));
    }
    if resp.double_clicked() {
        ui.ctx().data_mut(|d| d.insert_persisted(width_id(), BASE_W));
    }
}

/// Captions over the value fields of `model` (call just before its row).
pub(crate) fn captions(ui: &mut Ui, model: &str) {
    let t = Tokens::get(ui.ctx());
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 10.0), Sense::hover());
    let mut x = r.min.x + 16.0 + MODEL_DROPDOWN_W + GAP;
    for (label, w) in columns(model) {
        ui.painter().text(pos2(x + w / 2.0, r.center().y + 2.0), Align2::CENTER_CENTER, label, regular(9.5), t.text_secondary);
        x += w + GAP;
    }
}

// ---- binding channels to number variables ----------------------------------------------------

/// Where the edited colour's formula lives.
#[derive(Clone, Debug, PartialEq)]
enum Source {
    /// Layers (or a page): their `figcraftplus.colors` data, line `slot`.
    Nodes { ids: Vec<NodeId>, slot: String },
    /// A colour style's description, line `slot` ("paints/0", …).
    Style { id: String, slot: String },
    /// A colour variable's description (the line without a slot).
    Variable { id: String },
}

/// What the picker's rows need this frame (set by [`prepare`]).
#[derive(Clone, Debug, Default)]
struct BindCtx {
    source: Option<Source>,
    formula: Option<Formula>,
    /// The number variables: (name, collection name).
    numbers: Vec<(String, String)>,
}

/// A channel's variable button or detach button used.
#[derive(Clone, Copy, Debug)]
enum Ev {
    Open(usize, Pos2),
    Detach(usize),
}

fn ctx_id() -> Id {
    Id::new("cp_bind_ctx")
}
fn events_id() -> Id {
    Id::new("cp_bind_events")
}
fn popup_id() -> Id {
    Id::new("cp_bind_popup")
}

/// The source of `target`'s colour, if it can hold a formula.
fn source(app: &FigApp, ctx: &egui::Context, target: &PickerTarget) -> Option<Source> {
    let doc = app.session.doc().ok()?;
    let stop: usize = ctx.data(|d| d.get_temp(crate::colorpicker::stop_id())).unwrap_or(0);
    let with_stop = |base: String, gradient: bool| if gradient { format!("{base}/stops/{stop}") } else { base };
    let is_gradient = |p: Option<&figcraft_color::Paint>| p.is_some_and(|p| matches!(p.kind, figcraft_color::PaintKind::Gradient(_)));
    Some(match target {
        PickerTarget::Paint { fills, index, ids } => {
            let n = doc.node(*ids.first()?)?;
            let list = if *fills { &n.fills } else { &n.strokes };
            let base = format!("{}/{index}", if *fills { "fills" } else { "strokes" });
            Source::Nodes { ids: ids.clone(), slot: with_stop(base, is_gradient(list.get(*index))) }
        }
        PickerTarget::Effect { ids, index, secondary } => {
            Source::Nodes { ids: ids.clone(), slot: if *secondary { format!("effects/{index}/secondary") } else { format!("effects/{index}") } }
        }
        PickerTarget::PageBackground => Source::Nodes { ids: vec![app.session.page().ok()?], slot: "background".into() },
        PickerTarget::Style { id, index } => {
            let st = doc.style(id)?;
            let paint = match &st.kind {
                figcraft_doc::StyleKind::Paint { paints } => paints.get(*index),
                _ => None,
            };
            Source::Style { id: st.id.clone(), slot: with_stop(format!("paints/{index}"), is_gradient(paint)) }
        }
        PickerTarget::Variable { id, .. } => Source::Variable { id: doc.variables.get(id)?.id.clone() },
        PickerTarget::TextRange { .. } | PickerTarget::PluginField { .. } => return None,
    })
}

/// The text holding `src`'s formulas now.
fn source_text(app: &FigApp, src: &Source) -> String {
    let Ok(doc) = app.session.doc() else { return String::new() };
    match src {
        Source::Nodes { ids, .. } => ids.first().and_then(|i| doc.node(*i)).and_then(|n| n.plugin_data.get(oklch_links::NODE_KEY)).cloned().unwrap_or_default(),
        Source::Style { id, .. } => doc.style(id).map(|s| s.description.clone()).unwrap_or_default(),
        Source::Variable { id } => doc.variables.get(id).map(|v| v.description.clone()).unwrap_or_default(),
    }
}

fn slot_of(src: &Source) -> Option<&str> {
    match src {
        Source::Nodes { slot, .. } | Source::Style { slot, .. } => Some(slot),
        Source::Variable { .. } => None,
    }
}

/// `src`'s formula. A style's first paint also takes a line without a slot.
fn formula_of(app: &FigApp, src: &Source) -> Option<Formula> {
    let text = source_text(app, src);
    oklch_links::formula_in(&text, slot_of(src)).or_else(|| match src {
        Source::Style { slot, .. } if slot == "paints/0" => oklch_links::formula_in(&text, None),
        _ => None,
    })
}

/// Store `f` (None: remove it) as `src`'s formula, as one undo step.
fn write_formula(app: &mut FigApp, src: &Source, f: Option<&Formula>) {
    let Ok(doc) = app.session.doc() else { return };
    match src {
        Source::Nodes { ids, slot } => {
            let commands: Vec<serde_json::Value> = ids
                .iter()
                .filter_map(|i| doc.node(*i).map(|n| (i, n.plugin_data.get(oklch_links::NODE_KEY).cloned().unwrap_or_default())))
                .map(|(i, text)| {
                    let new = oklch_links::with_formula(&text, Some(slot), f);
                    let value = if new.trim().is_empty() { serde_json::Value::Null } else { json!(new) };
                    json!({"command": "node.set", "params": {"ids": [i], "props": {"pluginData": {oklch_links::NODE_KEY: value}}}})
                })
                .collect();
            app.run("command.batch", json!({"label": "Link colour to variables", "commands": commands}));
        }
        Source::Style { id, slot } => {
            let text = doc.style(id).map(|s| s.description.clone()).unwrap_or_default();
            // A legacy line without a slot (the first paint) moves to its slot.
            let text = if slot == "paints/0" { oklch_links::with_formula(&text, None, None) } else { text };
            let new = oklch_links::with_formula(&text, Some(slot), f);
            let id = id.clone();
            app.run("style.update", json!({"style": id, "description": new}));
        }
        Source::Variable { id } => {
            let text = doc.variables.get(id).map(|v| v.description.clone()).unwrap_or_default();
            let new = oklch_links::with_formula(&text, None, f);
            let id = id.clone();
            app.run("variable.update", json!({"variable": id, "description": new}));
        }
    }
}

/// Before the picker's rows: work out what the colour's formula is.
pub(crate) fn prepare(app: &FigApp, ctx: &egui::Context, target: &PickerTarget) {
    let source = source(app, ctx, target);
    let formula = source.as_ref().and_then(|s| formula_of(app, s));
    let numbers = app
        .session
        .doc()
        .map(|d| {
            d.variables
                .variables
                .iter()
                .filter(|v| v.kind == VariableType::Number)
                .map(|v| (v.name.clone(), d.variables.collection(&v.collection).map(|c| c.name.clone()).unwrap_or_default()))
                .collect()
        })
        .unwrap_or_default();
    ctx.data_mut(|d| d.insert_temp(ctx_id(), BindCtx { source, formula, numbers }));
}

fn bind_ctx(ctx: &egui::Context) -> BindCtx {
    ctx.data(|d| d.get_temp::<BindCtx>(ctx_id())).unwrap_or_default()
}

fn push(ctx: &egui::Context, e: Ev) {
    ctx.data_mut(|d| {
        let mut v: Vec<Ev> = d.get_temp(events_id()).unwrap_or_default();
        v.push(e);
        d.insert_temp(events_id(), v);
    });
}

/// `v` with up to `places` decimals, trailing zeros dropped ("0.2197", "0.14", "0").
fn num_text(v: f64, places: usize) -> String {
    let s = format!("{v:.places$}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    if s == "-0" { "0".into() } else { s }
}

/// A typed number; `pct_of` is what 100% means (`64.56%` gives 0.6456 for L, `55%` 0.22 for C).
fn parse_num(text: &str, pct_of: f64) -> Option<f64> {
    let t = text.trim().replace(',', ".");
    let (num, pct) = match t.strip_suffix('%') {
        Some(n) => (n.trim().to_string(), true),
        None => (t.clone(), false),
    };
    let v = num.parse::<f64>().ok().filter(|v| v.is_finite())?;
    Some(if pct { v / 100.0 * pct_of } else { v })
}

/// The channel values the row shows for `c` (OKLCH keeps a grey's hue).
fn values(ctx: &egui::Context, model: Model, c: Color, profile: ColorProfile) -> [f64; 4] {
    let mut v = model.values(c, profile);
    if model == Model::Oklch {
        let o = current(ctx, c, profile);
        v[0] = o.l;
        v[1] = o.c;
        v[2] = o.h;
    }
    v
}

/// The row of an RGB, HSL, HSB or OKLCH colour `c`: its three channels and alpha, each with a
/// variable button when the colour can hold a formula, or the bound variable's name. Returns the
/// edited colour.
pub(crate) fn model_row(ui: &mut Ui, c: Color, profile: ColorProfile, label: &str) -> Option<Color> {
    let model = Model::from_name(label)?;
    let ctx = ui.ctx().clone();
    let b = bind_ctx(&ctx);
    let bindable = b.source.is_some();
    let bound = b.formula.as_ref().filter(|f| f.model == model);
    let vals = values(&ctx, model, c, profile);
    let names = model.letters();
    let cols = columns(label);
    let mut new: Option<(usize, f64)> = None;
    for k in 0..4 {
        let w = cols.get(k).map_or(FIELD_W, |(_, w)| *w);
        let id = Id::new(("cp_ch", label, k));
        let name = format!("{} {}", model.label(), if k == 3 { "alpha" } else { names[k] });
        if let Some(var) = bound.and_then(|f| f.var(k)) {
            let (rect, _) = ui.allocate_exact_size(vec2(w, widgets::FIELD_H), Sense::hover());
            match widgets::bound_field(ui, id, widgets::Lead::Named(&name), rect, var) {
                Some(BindEvent::Open(at)) => push(&ctx, Ev::Open(k, at)),
                Some(BindEvent::Detach) => push(&ctx, Ev::Detach(k)),
                None => {}
            }
            continue;
        }
        let v = vals[k];
        if model == Model::Oklch && k < 2 {
            // L and C to 4 decimals, as oklch.com shows them (number fields show 2).
            let (rect, _) = ui.allocate_exact_size(vec2(w, widgets::FIELD_H), Sense::hover());
            let typed = ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| widgets::text_field_named(ui, id, &num_text(v, 4), w, &name)).inner;
            if let Some(x) = typed.as_deref().and_then(|t| parse_num(t, if k == 0 { 1.0 } else { OKLCH_FULL_CHROMA })) {
                // "64.56" without % means percent too: lightness can't exceed 1.
                new = Some((k, if k == 0 && x > 1.0 { x / 100.0 } else { x }));
            }
            if bindable && crate::variable_ui::variable_button(ui, rect, id.with("bind")) {
                push(&ctx, Ev::Open(k, rect.left_bottom()));
            }
            continue;
        }
        let shown = match (model, k) {
            (Model::Oklch, 2) => (v * 100.0).round() / 100.0,
            _ => v.round(),
        };
        let (edit, ev) = widgets::number_field_bindable(ui, id, widgets::Lead::Named(&name), Some(shown), "", w, 1.0, bindable.then_some(None), "");
        if let Some(widgets::FieldEdit::Set(x) | widgets::FieldEdit::Scrub(x, _)) = edit.and_then(widgets::Edit::settled) {
            new = Some((k, x));
        }
        match ev {
            Some(BindEvent::Open(at)) => push(&ctx, Ev::Open(k, at)),
            Some(BindEvent::Detach) => push(&ctx, Ev::Detach(k)),
            None => {}
        }
    }
    let (k, x) = new?;
    let mut v = vals;
    let x = match (model, k) {
        (_, 3) => x.clamp(0.0, 100.0),
        (Model::Oklch, 0) => x.clamp(0.0, 1.0),
        (Model::Oklch, 1) => x.clamp(0.0, 0.5),
        (Model::Rgb, _) => x.clamp(0.0, 255.0),
        (_, 0) | (Model::Oklch, 2) => x.rem_euclid(360.0),
        _ => x.clamp(0.0, 100.0),
    };
    if let Some(slot) = v.get_mut(k) {
        *slot = x;
    }
    let col = model.color(v, profile);
    if model == Model::Oklch {
        remember_hue(&ctx, col, v[2]);
    }
    Some(col)
}

/// After the picker's rows: handle the variable and detach buttons, draw the variable list, and
/// note links made in another model.
pub(crate) fn bindings_ui(app: &mut FigApp, ui: &mut Ui, color: Color, models: &[&str]) {
    let ctx = ui.ctx().clone();
    let b = bind_ctx(&ctx);
    let events: Vec<Ev> = ctx.data_mut(|d| d.remove_temp::<Vec<Ev>>(events_id())).unwrap_or_default();
    let Some(src) = b.source.clone() else { return };
    let label = models.get(model_index(&ctx)).copied().unwrap_or("Hex");
    let profile = app.session.doc().map(|d| d.color_profile).unwrap_or_default();
    let model = Model::from_name(label);
    for e in events {
        match (e, model) {
            (Ev::Open(k, at), Some(m)) => {
                // Under the field clicked (the button reports its row's corner).
                let x = ctx.input(|i| i.pointer.interact_pos()).map_or(at.x, |p| p.x - 60.0);
                let at = pos2(x, at.y);
                ctx.data_mut(|d| d.insert_temp(popup_id(), (k, m.name().to_string(), at)));
            }
            (Ev::Detach(k), Some(m)) => {
                if let Some(mut f) = b.formula.clone().filter(|f| f.model == m) {
                    let v = values(&ctx, m, color, profile);
                    if let Some(slot) = f.ch.get_mut(k) {
                        *slot = Channel::Fixed(v[k.min(3)]);
                    }
                    let keep = f.linked();
                    write_formula(app, &src, keep.then_some(&f));
                }
            }
            _ => {}
        }
    }
    // Links made in another model: say so (they stay until a link is made here).
    if let (Some(f), Some(m)) = (&b.formula, model)
        && f.model != m
        && f.linked()
    {
        let t = Tokens::get(&ctx);
        let linked: Vec<&str> = (0..4).filter(|k| f.var(*k).is_some()).map(|k| f.model.letters()[k]).collect();
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            widgets::label(ui, &format!("Linked in {}: {} (a link here replaces them)", f.model.label(), linked.join(", ")), t.text_secondary);
        });
    } else if let (Some(f), None) = (&b.formula, model)
        && f.linked()
    {
        let t = Tokens::get(&ctx);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            widgets::label(ui, &format!("Linked to variables in {}", f.model.label()), t.text_secondary);
        });
    }
    // The variable list.
    let Some((k, mname, at)) = ctx.data(|d| d.get_temp::<(usize, String, Pos2)>(popup_id())) else { return };
    let Some(m) = Model::from_name(&mname).filter(|m| Some(*m) == model) else {
        ctx.data_mut(|d| d.remove_temp::<(usize, String, Pos2)>(popup_id()));
        return;
    };
    let current = b.formula.as_ref().filter(|f| f.model == m).and_then(|f| f.var(k)).map(str::to_string);
    let mut picked: Option<String> = None;
    let mut close = false;
    let area = egui::Area::new(popup_id()).order(egui::Order::Foreground).fixed_pos(at + vec2(0.0, 4.0)).show(&ctx, |ui| {
        egui::Frame::popup(&ctx.global_style()).show(ui, |ui| {
            ui.set_min_width(180.0);
            ui.set_max_width(240.0);
            ui.set_max_height(320.0);
            let t = Tokens::get(&ctx);
            widgets::label(ui, &format!("{} — number variable", m.letters()[k.min(3)]), t.text_secondary);
            if b.numbers.is_empty() {
                widgets::label(ui, "No number variables yet", t.text_secondary);
            }
            egui::ScrollArea::vertical().max_height(280.0).auto_shrink([true, true]).show(ui, |ui| {
                let mut last = String::new();
                for (name, coll) in &b.numbers {
                    if *coll != last {
                        widgets::label(ui, coll, t.text_tertiary);
                        last = coll.clone();
                    }
                    if crate::right::menu_check(ui, current.as_deref() == Some(name.as_str()), name, "") {
                        picked = Some(name.clone());
                    }
                }
            });
        });
    });
    let list = area.response.rect;
    let pressed_outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.press_origin().is_some_and(|p| !list.contains(p)));
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) || (pressed_outside && picked.is_none()) {
        close = true;
    }
    if let Some(name) = picked {
        let mut f = match b.formula.clone() {
            Some(f) if f.model == m => f,
            other => {
                if other.as_ref().is_some_and(Formula::linked) {
                    app.toast(format!("Links in {} replaced by {}", other.map(|f| f.model.label()).unwrap_or(""), m.label()));
                }
                Formula::fixed(m, color, profile)
            }
        };
        if let Some(slot) = f.ch.get_mut(k) {
            *slot = Channel::Var(name);
        }
        write_formula(app, &src, Some(&f));
        close = true;
    }
    if close {
        ctx.data_mut(|d| d.remove_temp::<(usize, String, Pos2)>(popup_id()));
    }
}

fn cache_id() -> Id {
    Id::new("cp_oklch")
}

/// The OKLCH of `c` as it is. A grey has no hue of its own: it keeps the hue last set for it (so
/// dragging chroma to 0 and back doesn't lose the hue).
fn current(ctx: &egui::Context, c: Color, profile: ColorProfile) -> Oklch {
    let o = Oklch::from_color(c, profile);
    if o.c >= 0.0005 {
        return o;
    }
    let hue = ctx.data(|d| d.get_temp::<([u8; 4], f64)>(cache_id())).filter(|(k, _)| *k == c.to_rgba8()).map(|(_, h)| h);
    hue.map_or(o, |h| Oklch { h, ..o })
}

/// `c` rounded to what its hex shows (8 bits a channel), so the OKLCH shown for it is exactly the
/// OKLCH of its hex (what oklch.com and CSS tools compute from a copied hex).
fn snap(c: Color) -> Color {
    let [r, g, b, a] = c.to_rgba8();
    Color::new(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, f32::from(a) / 255.0)
}

fn remember_hue(ctx: &egui::Context, c: Color, h: f64) {
    ctx.data_mut(|d| d.insert_temp(cache_id(), (c.to_rgba8(), h)));
}

/// Whether the picker shows the OKLCH model.
pub(crate) fn active(ctx: &egui::Context, models: &[&str]) -> bool {
    models.get(model_index(ctx)).copied() == Some("OKLCH")
}

/// The colour for `want` (gamut-mapped into `profile`, alpha `a`) as the picker will hold it (it
/// goes through HSB), remembered so the fields and the plane show `want` itself.
fn set(ctx: &egui::Context, want: Oklch, profile: ColorProfile, a: f32, hsb: &mut Hsb) {
    *hsb = Hsb::from_color(snap(want.to_color(profile, a)));
    remember_hue(ctx, hsb.to_color().with_alpha(a), want.h);
}

/// The largest chroma at lightness `l` and hue `h` inside `profile`'s gamut.
fn max_chroma(l: f64, h: f64, profile: ColorProfile) -> f64 {
    let (mut lo, mut hi) = (0.0, OKLCH_FULL_CHROMA);
    for _ in 0..18 {
        let mid = (lo + hi) / 2.0;
        if Oklch::new(l, mid, h).in_gamut(profile) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

fn knob(ui: &Ui, at: egui::Pos2, r: f32) {
    ui.painter().circle_stroke(at, r, Stroke::new(2.0, Color32::WHITE));
    ui.painter().circle_stroke(at, r + 1.0, Stroke::new(1.0, Color32::from_black_alpha(80)));
}

/// The OKLCH plane in place of the HSB square: chroma 0–0.4 across, lightness 100%–0 down, at the
/// current hue. Colours outside the file's gamut are left out and its edge is drawn (in a Display
/// P3 file, the sRGB edge too, dashed). Returns (changed, done) like the square it replaces.
pub(crate) fn plane(ui: &mut Ui, rect: Rect, hsb: &mut Hsb, color: Color, profile: ColorProfile) -> (bool, bool) {
    let ctx = ui.ctx().clone();
    let lch = current(&ctx, color, profile);
    const NX: usize = 32;
    const NY: usize = 60;
    // Each row runs from grey to the gamut's edge at its lightness, so the edge is exact and smooth.
    let mut m = Mesh::default();
    for j in 0..=NY {
        let y = rect.min.y + j as f32 / NY as f32 * rect.height();
        let l = 1.0 - j as f64 / NY as f64;
        let cmax = max_chroma(l, lch.h, profile);
        for i in 0..=NX {
            let c = i as f64 / NX as f64 * cmax;
            let col = doc_c32(&ctx, Oklch::new(l, c, lch.h).to_color(profile, 1.0));
            m.colored_vertex(pos2(rect.min.x + (c / OKLCH_FULL_CHROMA) as f32 * rect.width(), y), col);
        }
    }
    let row = (NX + 1) as u32;
    for j in 0..NY as u32 {
        for i in 0..NX as u32 {
            let k = j * row + i;
            m.add_triangle(k, k + 1, k + row);
            m.add_triangle(k + 1, k + row + 1, k + row);
        }
    }
    ui.painter().add(m);
    // The gamut edge (and sRGB's inside a P3 file).
    let edge = |p: ColorProfile| -> Vec<egui::Pos2> {
        (0..=NY * 2)
            .map(|j| {
                let t = j as f32 / (NY * 2) as f32;
                let c = max_chroma(1.0 - f64::from(t), lch.h, p);
                pos2(rect.min.x + (c / OKLCH_FULL_CHROMA) as f32 * rect.width(), rect.min.y + t * rect.height())
            })
            .collect()
    };
    let t = Tokens::get(&ctx);
    ui.painter().add(egui::Shape::line(edge(profile), Stroke::new(1.0, t.text_secondary)));
    if profile == ColorProfile::DisplayP3 {
        ui.painter().extend(egui::Shape::dashed_line(&edge(ColorProfile::Srgb), Stroke::new(1.0, t.text_secondary), 3.0, 3.0));
    }
    let resp = ui.interact(rect, Id::new("cp_sv"), Sense::click_and_drag());
    let mut changed = false;
    if (resp.dragged() || resp.clicked() || resp.drag_started())
        && let Some(p) = resp.interact_pointer_pos()
    {
        let c = f64::from(((p.x - rect.min.x) / rect.width()).clamp(0.0, 1.0)) * OKLCH_FULL_CHROMA;
        let l = 1.0 - f64::from(((p.y - rect.min.y) / rect.height()).clamp(0.0, 1.0));
        set(&ctx, Oklch::new(l, c, lch.h), profile, color.a, hsb);
        changed = true;
    }
    let shown = if changed { current(&ctx, hsb.to_color().with_alpha(color.a), profile) } else { lch };
    knob(ui, pos2(rect.min.x + (shown.c / OKLCH_FULL_CHROMA).min(1.0) as f32 * rect.width(), rect.min.y + (1.0 - shown.l) as f32 * rect.height()), 6.0);
    (changed, resp.drag_stopped() || resp.clicked())
}

/// The OKLCH hue strip in place of the rainbow: every hue at the current lightness and chroma
/// (gamut-mapped where it doesn't fit). Returns (changed, done) like the slider it replaces.
pub(crate) fn hue_strip(ui: &mut Ui, rect: Rect, hsb: &mut Hsb, color: Color, profile: ColorProfile) -> (bool, bool) {
    let ctx = ui.ctx().clone();
    let lch = current(&ctx, color, profile);
    const N: usize = 72;
    let mut m = Mesh::default();
    for i in 0..=N {
        let t = i as f32 / N as f32;
        let c = doc_c32(&ctx, Oklch::new(lch.l, lch.c, f64::from(t) * 360.0).to_color(profile, 1.0));
        let x = rect.min.x + t * rect.width();
        m.colored_vertex(pos2(x, rect.min.y), c);
        m.colored_vertex(pos2(x, rect.max.y), c);
        if i > 0 {
            let k = (i * 2) as u32;
            m.add_triangle(k - 2, k - 1, k);
            m.add_triangle(k - 1, k, k + 1);
        }
    }
    ui.painter().add(m);
    let resp = ui.interact(rect.expand(4.0), Id::new("cp_hue"), Sense::click_and_drag());
    let mut changed = false;
    if (resp.dragged() || resp.clicked() || resp.drag_started())
        && let Some(p) = resp.interact_pointer_pos()
    {
        let h = f64::from(((p.x - rect.min.x) / rect.width()).clamp(0.0, 1.0)) * 360.0;
        set(&ctx, Oklch::new(lch.l, lch.c, h.min(359.99)), profile, color.a, hsb);
        changed = true;
    }
    let shown = if changed { current(&ctx, hsb.to_color().with_alpha(color.a), profile) } else { lch };
    knob(ui, pos2(rect.min.x + (shown.h / 360.0) as f32 * rect.width(), rect.center().y), rect.height() / 2.0 + 1.0);
    (changed, resp.drag_stopped() || resp.clicked())
}
