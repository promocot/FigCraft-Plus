//! Custom colour picker additions (kept in this one file so FigCraft updates touch it rarely):
//!
//! - the OKLCH model's fields: lightness (%), chroma (% of 0.4, as CSS writes it) and hue
//!   (degrees), read and written in the document's colour profile, gamut-mapped into it;
//! - captions over every model's fields (R G B, H S L, H S B, L C H, A);
//! - in the OKLCH model, an OKLCH plane (chroma across, lightness down, at the current hue, with
//!   the file's gamut edge) and an OKLCH hue strip in place of the HSB square and rainbow;
//! - a picker that widens to fit the chosen model and can be dragged wider by its left edge;
//! - for a colour variable, links from its L, C and H to number variables (an `oklch(…)` formula in
//!   its description, kept up to date by `figcraft_engine::oklch_links`).

use egui::{Align2, Color32, Id, Mesh, Rect, Sense, Stroke, Ui, pos2, vec2};
use figcraft_color::{Color, ColorProfile, Hsb, OKLCH_FULL_CHROMA, Oklch};
use figcraft_doc::VariableType;
use figcraft_engine::oklch_links::{self, Channel, Formula};
use serde_json::json;

use crate::FigApp;
use crate::colorpicker::PickerTarget;
use crate::theme::{Tokens, doc_c32, regular};
use crate::widgets;

/// The model dropdown's width (wide enough for "OKLCH").
pub(crate) const MODEL_DROPDOWN_W: f32 = 64.0;
/// One OKLCH field's width (fits "64.42").
const OKLCH_FIELD_W: f32 = 40.0;
/// The picker's narrowest width (FigCraft's own).
const BASE_W: f32 = 240.0;
const MAX_W: f32 = 640.0;
const GAP: f32 = 4.0;
const ALPHA_W: f32 = 44.0;

fn model_index(ctx: &egui::Context) -> usize {
    ctx.data(|d| d.get_temp::<usize>(Id::new("cp_model"))).unwrap_or(0)
}

/// The value fields a model shows: (caption, width) each, alpha last when it has one.
fn columns(model: &str) -> Vec<(&'static str, f32)> {
    let three = |a, b, c, w| vec![(a, w), (b, w), (c, w), ("A %", ALPHA_W)];
    match model {
        "RGB" => three("R", "G", "B", 31.0),
        "HSL" => three("H °", "S %", "L %", 31.0),
        "HSB" => three("H °", "S %", "B %", 31.0),
        "OKLCH" => three("L %", "C %", "H °", OKLCH_FIELD_W),
        "CSS" => vec![("CSS", 152.0)],
        _ => vec![("Hex", 101.0), ("A %", ALPHA_W)],
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

/// The three OKLCH fields for colour `c`: the committed colour when one was edited.
pub(crate) fn fields(ui: &mut Ui, c: Color, profile: ColorProfile) -> Option<Color> {
    // The last OKLCH typed for this colour: keeps the exact values (and the hue of a grey) that
    // the RGB round trip would blur.
    let cache = cache_id();
    let lch = current(ui.ctx(), c, profile);
    let vals = [lch.l * 100.0, lch.c / OKLCH_FULL_CHROMA * 100.0, lch.h];
    let mut out = None;
    for (k, v) in vals.iter().enumerate() {
        let (id, name, step) = [("ok_l", "OKLCH lightness", 0.5), ("ok_c", "OKLCH chroma", 0.5), ("ok_h", "OKLCH hue", 1.0)][k.min(2)];
        let shown = Some((*v * 100.0).round() / 100.0);
        let edit = widgets::number_field(ui, Id::new(("cp_f", id)), widgets::Lead::Named(name), shown, "", OKLCH_FIELD_W, step);
        if let Some(widgets::FieldEdit::Set(x) | widgets::FieldEdit::Scrub(x, _)) = edit {
            let mut v3 = vals;
            if let Some(slot) = v3.get_mut(k) {
                *slot = if k == 2 { x.rem_euclid(360.0) } else { x.clamp(0.0, 100.0) };
            }
            let want = Oklch::new(v3[0] / 100.0, v3[1] / 100.0 * OKLCH_FULL_CHROMA, v3[2]);
            let col = want.to_color(profile, c.a);
            ui.data_mut(|d| d.insert_temp(cache, (col.to_rgba8(), want.l, want.c, want.h)));
            out = Some(col);
        }
    }
    out
}

/// For a colour variable shown in the OKLCH model: a row linking its L, C and H to number
/// variables. `color` is the colour the picker shows (the variable's value in the edited mode).
pub(crate) fn variable_links(app: &mut FigApp, ui: &mut Ui, target: &PickerTarget, color: Color, models: &[&str], width: f32) {
    let PickerTarget::Variable { id, .. } = target else { return };
    if models.get(model_index(ui.ctx())).copied() != Some("OKLCH") {
        return;
    }
    let Ok(doc) = app.session.doc() else { return };
    let Some(var) = doc.variables.get(id).filter(|v| v.kind == VariableType::Color) else { return };
    let profile = doc.color_profile;
    let formula = oklch_links::parse(&var.description);
    let numbers: Vec<String> = doc.variables.variables.iter().filter(|v| v.kind == VariableType::Number).map(|v| v.name.clone()).collect();
    let (var_id, description) = (var.id.clone(), var.description.clone());
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        widgets::label(ui, "Linked to number variables", t.text_secondary);
    });
    let mut pick: Option<(usize, Option<String>)> = None;
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.spacing_mut().item_spacing.x = GAP;
        let w = ((width - 32.0 - 2.0 * GAP) / 3.0).max(48.0);
        for (k, letter) in ["L", "C", "H"].iter().enumerate() {
            let linked = formula.as_ref().and_then(|f| match [&f.l, &f.c, &f.h][k] {
                Channel::Var(n) => Some(n.clone()),
                Channel::Fixed(_) => None,
            });
            let text = format!("{letter}: {}", linked.as_deref().unwrap_or("—"));
            let resp = widgets::dropdown(ui, &text, w, None).on_hover_text(match k {
                0 => "Lightness from a number variable (0–100)",
                1 => "Chroma from a number variable (0–0.4)",
                _ => "Hue from a number variable (degrees)",
            });
            crate::right::menu_popup(&resp).show(|ui| {
                if crate::right::menu_check(ui, linked.is_none(), "— (fixed value)", "") {
                    pick = Some((k, None));
                    ui.close();
                }
                for n in &numbers {
                    if crate::right::menu_check(ui, linked.as_deref() == Some(n.as_str()), n, "") {
                        pick = Some((k, Some(n.clone())));
                        ui.close();
                    }
                }
                if numbers.is_empty() {
                    widgets::label(ui, "No number variables yet", t.text_secondary);
                }
            });
        }
    });
    let Some((k, choice)) = pick else { return };
    // Start from the formula, or from the colour as it is now.
    let mut f: Formula = formula.unwrap_or_else(|| oklch_links::fixed_from(Oklch::from_color(color, profile)));
    let now = Oklch::from_color(color, profile);
    let slot = match k {
        0 => &mut f.l,
        1 => &mut f.c,
        _ => &mut f.h,
    };
    *slot = match choice {
        Some(n) => Channel::Var(n),
        None => Channel::Fixed([now.l, now.c, now.h][k.min(2)]),
    };
    // All channels fixed again: the colour no longer needs a formula.
    let keep = [&f.l, &f.c, &f.h].iter().any(|c| matches!(c, Channel::Var(_)));
    let new = oklch_links::with_formula(&description, keep.then_some(&f));
    app.run("variable.update", json!({"variable": var_id, "description": new}));
}

fn cache_id() -> Id {
    Id::new("cp_oklch")
}

/// The OKLCH the picker shows for `c`: the last one set for this exact colour (keeps the values
/// typed or dragged, and the hue of a grey), else `c`'s own.
fn current(ctx: &egui::Context, c: Color, profile: ColorProfile) -> Oklch {
    let rgba = c.to_rgba8();
    ctx.data(|d| d.get_temp::<([u8; 4], f64, f64, f64)>(cache_id()))
        .filter(|(k, ..)| *k == rgba)
        .map(|(_, l, ch, h)| Oklch::new(l, ch, h))
        .unwrap_or_else(|| Oklch::from_color(c, profile))
}

/// Whether the picker shows the OKLCH model.
pub(crate) fn active(ctx: &egui::Context, models: &[&str]) -> bool {
    models.get(model_index(ctx)).copied() == Some("OKLCH")
}

/// The colour for `want` (gamut-mapped into `profile`, alpha `a`) as the picker will hold it (it
/// goes through HSB), remembered so the fields and the plane show `want` itself.
fn set(ctx: &egui::Context, want: Oklch, profile: ColorProfile, a: f32, hsb: &mut Hsb) {
    *hsb = Hsb::from_color(want.to_color(profile, a));
    let held = hsb.to_color().with_alpha(a);
    ctx.data_mut(|d| d.insert_temp(cache_id(), (held.to_rgba8(), want.l, want.c, want.h)));
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
