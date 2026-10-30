//! FigCraft Plus: the units fields show values in.
//!
//! Each file has two display units (Units: the gear in Typography or Layout, or the Page section):
//!
//! - **Layout** — position, size, min / max, corner radius, gaps and padding, stroke weights and
//!   dashes, shadow and blur sizes, layout grid sizes, vector points, the spacing between
//!   selected layers, the size label on the canvas and the measurements: px, rem, mm, cm, in or pt.
//! - **Text** — font size, line height, letter spacing, paragraph spacing and indent, list
//!   spacing: px, rem or pt (see `rem_ui`).
//!
//! Values are always stored in px, as FigCraft stores them: a field divides by the unit's size to
//! show, and multiplies back what is typed or scrubbed. A number typed with a unit (`12px`,
//! `5mm`, `1.5rem`) means that unit whatever the display unit. A px is a point (1/72 in), as in
//! FigCraft's print export; rem is the file's root size.
//!
//! Every length field shows its unit at its right end (in px only while the pointer is over it):
//! a click opens the list of units and switches the **document's** unit — never one property's.
//! How many decimals fields show is a preference (0–4); values themselves are never rounded.
//!
//! The choices are kept in the first page's layer data (`figcraftplus.layoutUnit`,
//! `figcraftplus.remUnit`), so the official app opens the file as before; a file without them
//! uses the defaults from Preferences ▸ FigCraft Plus.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use egui::{Align2, CornerRadius, Id, Rect, Sense, Ui, pos2, vec2};
use figcraft_doc::Document;

use crate::FigApp;
use crate::i18n::tr;
use crate::theme::{Tokens, regular};
use crate::widgets::{Edit, FieldEdit, Rel};

/// The layer data key (on a page) holding the file's layout unit.
pub(crate) const LAYOUT_KEY: &str = "figcraftplus.layoutUnit";
/// The units layout fields can show.
pub(crate) const LAYOUT_UNITS: [&str; 6] = ["px", "rem", "mm", "cm", "in", "pt"];
/// The units text fields can show.
pub(crate) const TEXT_UNITS: [&str; 3] = ["px", "rem", "pt"];

static LAYOUT: AtomicUsize = AtomicUsize::new(0);
static TEXT: AtomicUsize = AtomicUsize::new(0);

/// What a number field measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Layout,
    Text,
}

thread_local! {
    /// The kind of the number field about to be drawn (set by the variable-bindable wrappers).
    static NEXT: Cell<Option<Kind>> = const { Cell::new(None) };
    /// Number fields known by their id.
    static BY_ID: HashMap<Id, Kind> = by_id();
    /// Where the number field being drawn sits.
    static RECT: Cell<Rect> = const { Cell::new(Rect::NOTHING) };
}

/// px in one `unit` (`root`: the file's root size).
pub(crate) fn per(unit: &str, root: f64) -> f64 {
    match unit {
        "rem" => root,
        "mm" => 72.0 / 25.4,
        "cm" => 72.0 / 2.54,
        "in" => 72.0,
        _ => 1.0,
    }
}

fn pick(list: &[&'static str], want: &str) -> Option<usize> {
    list.iter().position(|u| u.eq_ignore_ascii_case(want.trim()))
}

/// The file's layout unit (its own choice, else the preference).
pub(crate) fn doc_layout(doc: &Document) -> &'static str {
    let own = doc.pages.iter().find_map(|p| doc.node(*p)?.plugin_data.get(LAYOUT_KEY).and_then(|u| pick(&LAYOUT_UNITS, u)));
    LAYOUT_UNITS[own.or_else(|| pick(&LAYOUT_UNITS, &crate::prefs_plus_ui::get().layout_unit)).unwrap_or(0)]
}

/// The file's text unit (its own choice, else the preference).
pub(crate) fn doc_text(doc: &Document) -> &'static str {
    let own = doc.pages.iter().find_map(|p| doc.node(*p)?.plugin_data.get(crate::rem_ui::UNIT_KEY).and_then(|u| pick(&TEXT_UNITS, u)));
    TEXT_UNITS[own.or_else(|| pick(&TEXT_UNITS, &crate::prefs_plus_ui::get().text_unit())).unwrap_or(0)]
}

/// Each frame: the open file's units.
pub(crate) fn tick(doc: Option<&Document>) {
    let (l, t) = doc.map(|d| (doc_layout(d), doc_text(d))).unwrap_or(("px", "px"));
    LAYOUT.store(pick(&LAYOUT_UNITS, l).unwrap_or(0), Ordering::Relaxed);
    TEXT.store(pick(&TEXT_UNITS, t).unwrap_or(0), Ordering::Relaxed);
}

/// The layout unit now.
pub(crate) fn layout() -> &'static str {
    LAYOUT_UNITS[LAYOUT.load(Ordering::Relaxed).min(LAYOUT_UNITS.len() - 1)]
}

/// The text unit now.
pub(crate) fn text() -> &'static str {
    TEXT_UNITS[TEXT.load(Ordering::Relaxed).min(TEXT_UNITS.len() - 1)]
}

/// px in one layout unit now.
pub(crate) fn layout_k() -> f64 {
    per(layout(), crate::rem_ui::root())
}

/// px in one text unit now.
pub(crate) fn text_k() -> f64 {
    per(text(), crate::rem_ui::root())
}

/// For tests: set the units.
#[cfg(test)]
pub(crate) fn set_for_test(layout: &str, text: &str) {
    LAYOUT.store(pick(&LAYOUT_UNITS, layout).unwrap_or(0), Ordering::Relaxed);
    TEXT.store(pick(&TEXT_UNITS, text).unwrap_or(0), Ordering::Relaxed);
}

// ---- which fields are lengths -------------------------------------------------------------------

/// What a variable-bindable property measures (its key, as `variable.bind` names it).
pub(crate) fn classify(field: &str) -> Option<Kind> {
    const LAYOUT_FIELDS: [&str; 26] = [
        "width",
        "height",
        "minWidth",
        "maxWidth",
        "minHeight",
        "maxHeight",
        "gap",
        "columnGap",
        "rowGap",
        "crossGap",
        "counterAxisSpacing",
        "padding",
        "paddingHorizontal",
        "paddingVertical",
        "paddingTop",
        "paddingRight",
        "paddingBottom",
        "paddingLeft",
        "cornerRadius",
        "topLeftRadius",
        "topRightRadius",
        "bottomRightRadius",
        "bottomLeftRadius",
        "strokeWeight",
        "strokeTopWeight",
        "strokeRightWeight",
    ];
    if LAYOUT_FIELDS.contains(&field) || matches!(field, "strokeBottomWeight" | "strokeLeftWeight") {
        return Some(Kind::Layout);
    }
    if matches!(field, "fontSize" | "paragraphIndent" | "paragraphSpacing" | "listSpacing") {
        return Some(Kind::Text);
    }
    // Effects: offsets, blur and spread; layout grids: size, gutter, offset (not the count).
    let mut parts = field.split('/');
    match (parts.next(), parts.nth(1)) {
        (Some("effects"), Some("offsetX" | "offsetY" | "radius" | "spread" | "startRadius")) => Some(Kind::Layout),
        (Some("layoutGrids"), Some("size" | "gutter" | "offset" | "sectionSize" | "gutterSize")) => Some(Kind::Layout),
        _ => None,
    }
}

/// The variable-bindable field about to be drawn is `field`.
pub(crate) fn field_key(field: &str) {
    NEXT.with(|n| n.set(classify(field)));
}

/// Fields FigCraft draws without a variable button, known by their id.
fn by_id() -> HashMap<Id, Kind> {
    let mut m = HashMap::new();
    let l = Kind::Layout;
    for k in ["pos_x", "pos_y", "scale_w", "scale_h", "vpt_x", "vpt_y", "vpt_w", "vpt_h", "vpt_radius", "wpt_w", "stroke_dash", "stroke_gap"] {
        m.insert(Id::unique(k), l);
    }
    for k in ["h", "v", "horizontal", "vertical", "x", "y", "gap", "spacing"] {
        m.insert(Id::unique(("selection_spacing", k)), l);
    }
    for i in 0..32usize {
        m.insert(Id::unique(("grid_field", "size", i)), l);
    }
    m.insert(Id::unique("ty_list_spacing"), Kind::Text);
    m.insert(Id::unique("ty_size"), Kind::Text);
    m
}

/// When the number field `id` (with `suffix`) about to be drawn is a length: px per unit of the
/// unit it shows (1 in px), the unit's name and what it measures.
pub(crate) fn field(id: Id, suffix: &str) -> Option<(f64, &'static str, Kind)> {
    let kind = NEXT.with(Cell::take).or_else(|| BY_ID.with(|m| m.get(&id).copied()))?;
    // What this field's own text names is what counts (see `back`).
    crate::rem_ui::take_explicit();
    if !suffix.is_empty() {
        return None;
    }
    let (k, unit) = match kind {
        Kind::Layout => (layout_k(), layout()),
        Kind::Text => (text_k(), text()),
    };
    (k.is_finite() && k > 0.0).then_some((k, unit, kind))
}

/// The number field being drawn sits in `rect` (FigCraft's field, before its unit chip).
pub(crate) fn note_rect(rect: Rect) {
    RECT.with(|r| r.set(rect));
}

fn pending_id() -> Id {
    Id::unique("figcraftplus_unit_pending")
}

/// A length field's unit, right after the value it shows (`shown`, which starts `lead_w` into the
/// field), shown in px only while the pointer is over the field: a click lists the units and
/// switches the document's.
pub(crate) fn chip(ui: &mut Ui, id: Id, kind: Kind, lead_w: f32, shown: &str) {
    let rect = RECT.with(Cell::get);
    if !rect.is_positive() || ui.data(|d| d.get_temp::<String>(id.with("editing"))).is_some() {
        return;
    }
    let t = Tokens::get(ui.ctx());
    let (unit, list): (&str, &[&'static str]) = match kind {
        Kind::Layout => (layout(), &LAYOUT_UNITS),
        Kind::Text => (text(), &TEXT_UNITS),
    };
    let value_w = ui.painter().layout_no_wrap(shown.to_string(), regular(11.0), t.text).size().x;
    let unit_w = ui.painter().layout_no_wrap(unit.to_string(), regular(11.0), t.text_secondary).size().x;
    let x = rect.min.x + lead_w + value_w + 1.0;
    let r = Rect::from_min_max(pos2(x - 1.0, rect.min.y + 4.0), pos2((x + unit_w + 3.0).min(rect.max.x - 2.0), rect.max.y - 4.0));
    if r.width() < unit_w {
        return;
    }
    let resp = ui.interact(r, id.with("unit_chip"), Sense::click());
    let over = ui.rect_contains_pointer(rect);
    if unit != "px" || over || resp.hovered() {
        if resp.hovered() {
            ui.painter().rect_filled(r, CornerRadius::same(3), t.hover);
        }
        let col = if resp.hovered() { t.text } else { t.text_secondary };
        ui.painter().text(pos2(x, rect.center().y), Align2::LEFT_CENTER, unit, regular(11.0), col);
    }
    let resp = resp.on_hover_text(tr("Units of this document"));
    crate::right::menu_popup(&resp).show(|ui| {
        for u in list {
            if crate::right::menu_check(ui, *u == unit, u, "") {
                ui.data_mut(|d| d.insert_temp(pending_id(), (kind == Kind::Text, (*u).to_string())));
                ui.close();
            }
        }
        ui.separator();
        if crate::right::menu_item(ui, tr("Units settings…"), "") {
            crate::prefs_plus_ui::open_units(ui.ctx(), pos2(resp.rect.min.x - 300.0, resp.rect.min.y - 60.0));
            ui.close();
        }
    });
}

/// Each frame: a unit picked from a field's chip becomes the document's.
pub(crate) fn apply_pending(app: &mut FigApp, ctx: &egui::Context) {
    let Some((text, unit)) = ctx.data_mut(|d| d.remove_temp::<(bool, String)>(pending_id())) else { return };
    let Some(page) = app.session.doc().ok().and_then(|d| d.pages.first().copied()) else { return };
    let key = if text { crate::rem_ui::UNIT_KEY } else { LAYOUT_KEY };
    app.run("node.set", serde_json::json!({"ids": [page], "props": {"pluginData": {key: unit}}}));
}

/// A field's scrub and arrow step in `unit` (`step`: FigCraft's, for px): a tenth for the large
/// units (rem, cm, in).
pub(crate) fn step(unit: &str, step: f64) -> f64 {
    if matches!(unit, "rem" | "cm" | "in") { step / 10.0 } else { step }
}

/// A field's edit (in its display unit) in px: a number typed with its own unit is px already.
pub(crate) fn back(e: Edit, k: f64) -> Edit {
    let explicit = crate::rem_ui::take_explicit();
    match e {
        Edit::All(FieldEdit::Set(v)) if explicit => Edit::All(FieldEdit::Set(v)),
        Edit::All(FieldEdit::Set(v)) => Edit::All(FieldEdit::Set(v * k)),
        Edit::All(FieldEdit::Scrub(v, done)) => Edit::All(FieldEdit::Scrub(v * k, done)),
        Edit::Each(Rel::Add(v)) if !explicit => Edit::Each(Rel::Add(v * k)),
        other => other,
    }
}

// ---- labels on the canvas -------------------------------------------------------------------------

pub(crate) fn num(v: f64) -> String {
    let s = format!("{v:.*}", crate::prefs_plus_ui::precision());
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_string() }
}

/// A length in px as the layout unit shows it ("210 mm"); None in px (FigCraft's own text).
pub(crate) fn length(px: f64) -> Option<String> {
    let unit = layout();
    (unit != "px").then(|| format!("{}{unit}", num(px / layout_k())))
}

/// The selection's size label: `w × h` in the layout unit (`fmt`: FigCraft's own, for px).
pub(crate) fn size_label(w: f64, h: f64, line: bool, fmt: impl Fn(f64) -> String) -> String {
    match (length(w), length(h)) {
        (Some(a), _) if line => a,
        (Some(a), Some(b)) => format!("{a} × {b}"),
        _ if line => fmt(w),
        _ => format!("{} × {}", fmt(w), fmt(h)),
    }
}

/// A measurement label (a px number as text) in the layout unit.
pub(crate) fn measure(label: String) -> String {
    match label.trim().parse::<f64>() {
        Ok(px) if px.is_finite() => length(px).unwrap_or(label),
        _ => label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_fields_show_and_take_the_layout_unit() {
        crate::rem_ui::set_root_for_test(16.0);
        set_for_test("mm", "pt");
        assert_eq!(classify("width"), Some(Kind::Layout));
        assert_eq!(classify("effects/0/radius"), Some(Kind::Layout));
        assert_eq!(classify("layoutGrids/2/count"), None);
        assert_eq!(classify("opacity"), None);
        assert_eq!(classify("paragraphIndent"), Some(Kind::Text));
        field_key("width");
        let (k, unit, kind) = field(Id::unique("lay_w"), "").unwrap();
        assert_eq!(kind, Kind::Layout);
        assert_eq!(unit, "mm");
        assert!(((595.28 / k) - 210.0).abs() < 0.01, "A4 width shows as 210 mm");
        assert!(matches!(back(Edit::All(FieldEdit::Set(210.0)), k), Edit::All(FieldEdit::Set(v)) if (v - 595.28).abs() < 0.01));
        // A percentage field is never converted, even with a length key.
        field_key("width");
        assert!(field(Id::unique("lay_w"), "%").is_none());
        // Position, by id.
        assert!(field(Id::unique("pos_x"), "").is_some());
        assert!(field(Id::unique("pos_rot"), "°").is_none());
        assert_eq!(size_label(595.28, 841.89, false, |v| v.to_string()), "210mm × 297mm");
        assert_eq!(measure("28.35".into()), "10mm");
        set_for_test("rem", "px");
        assert_eq!(size_label(24.0, 8.0, false, |v| v.to_string()), "1.5rem × 0.5rem");
        set_for_test("px", "px");
        assert_eq!(size_label(24.0, 8.0, false, |v| format!("{v}")), "24 × 8");
        assert!(matches!(field(Id::unique("pos_x"), ""), Some((k, "px", Kind::Layout)) if k == 1.0), "px: shown as it is");
        field_key("fontSize");
        assert!(matches!(field(Id::unique("ty_size"), ""), Some((_, "px", Kind::Text))));
    }
}
