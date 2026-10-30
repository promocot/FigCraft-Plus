//! FigCraft Plus: more in the colour picker for print and accessibility.
//!
//! - **CMYK** in the model menu: C, M, Y, K in percent, converted the way FigCraft's CMYK PDF
//!   export converts (`K = 1 − max(R, G, B)`), so the numbers are the ones the PDF will hold.
//! - **Copy as**: the colour as Hex, RGB, HSL, HSB, LCH, OKLCH, CMYK or CSS, to the clipboard — a
//!   converter between colour spaces.
//! - **Contrast** under the fields: the WCAG 2 ratio (with AA / AAA) and the APCA lightness
//!   contrast (Lc) of the colour against what it sits on (the nearest frame with a solid fill,
//!   else the page).

use egui::{Id, Ui};
use figcraft_color::oklch_profile::Oklch;
use figcraft_color::{Color, ColorProfile, Hsb, Hsl, Lch, PaintKind};
use figcraft_doc::{Document, NodeId};

use crate::FigApp;
use crate::colorpicker::PickerTarget;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::widgets::{self, FieldEdit, Lead};

// ---- CMYK -------------------------------------------------------------------------------------------

/// sRGB → CMYK (0..1 each), the formula FigCraft's CMYK PDF export uses.
pub(crate) fn cmyk_of(c: Color) -> [f64; 4] {
    let unit = |v: f32| if v.is_finite() { f64::from(v.clamp(0.0, 1.0)) } else { 0.0 };
    let (r, g, b) = (unit(c.r), unit(c.g), unit(c.b));
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 - 1e-6 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let ink = |v: f64| ((1.0 - v - k) / (1.0 - k)).clamp(0.0, 1.0);
    [ink(r), ink(g), ink(b), k]
}

/// CMYK (0..1 each) → the colour, keeping alpha `a`.
pub(crate) fn from_cmyk([c, m, y, k]: [f64; 4], a: f32) -> Color {
    let ch = |v: f64| ((1.0 - v.clamp(0.0, 1.0)) * (1.0 - k.clamp(0.0, 1.0))) as f32;
    Color::new(ch(c), ch(m), ch(y), a)
}

/// The CMYK row: C, M, Y and K in percent. Returns the edited colour.
pub(crate) fn cmyk_row(ui: &mut Ui, c: Color) -> Option<Color> {
    let v = cmyk_of(c).map(|x| (x * 100.0).round());
    let mut out = None;
    for (k, name) in ["C", "M", "Y", "K"].into_iter().enumerate() {
        let id = Id::unique(("cp_cmyk", k));
        let label = format!("CMYK {name}");
        match widgets::number_field(ui, id, Lead::Named(&label), Some(v[k]), "", CMYK_W, 1.0) {
            Some(FieldEdit::Set(x) | FieldEdit::Scrub(x, _)) => {
                let mut nv = v;
                nv[k] = x.clamp(0.0, 100.0);
                out = Some(from_cmyk(nv.map(|p| p / 100.0), c.a));
            }
            None => {}
        }
    }
    out
}

/// A CMYK field's width.
pub(crate) const CMYK_W: f32 = 40.0;

// ---- Copy as -------------------------------------------------------------------------------------------

fn n(v: f64, places: usize) -> String {
    let s = format!("{v:.places$}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    if s == "-0" { "0".into() } else { s }
}

/// `c` written in every format "Copy as" offers: (label, text).
pub(crate) fn formats(c: Color, profile: ColorProfile) -> Vec<(&'static str, String)> {
    let a = f64::from(c.a);
    let alpha = |s: String| if a < 0.999 { format!("{s} / {}%", n(a * 100.0, 1)) } else { s };
    let [r, g, b, _] = c.to_rgba8();
    let hsl = Hsl::from_color(c);
    let hsb = Hsb::from_color(c);
    let lch = Lch::from_color(c);
    let ok = Oklch::from_color(c, profile);
    let [cc, m, y, k] = cmyk_of(c).map(|x| x * 100.0);
    vec![
        ("Hex", c.to_css_hex()),
        ("RGB", format!("{})", alpha(format!("rgb({r} {g} {b}")))),
        ("HSL", format!("{})", alpha(format!("hsl({} {}% {}%", n(f64::from(hsl.h), 1), n(f64::from(hsl.s) * 100.0, 1), n(f64::from(hsl.l) * 100.0, 1))))),
        ("HSB", format!("hsb({} {}% {}%)", n(f64::from(hsb.h), 1), n(f64::from(hsb.s) * 100.0, 1), n(f64::from(hsb.b) * 100.0, 1))),
        ("LCH", format!("{})", alpha(format!("lch({}% {} {}", n(f64::from(lch.l), 2), n(f64::from(lch.c), 2), n(f64::from(lch.h), 2))))),
        ("OKLCH", format!("{})", alpha(format!("oklch({} {} {}", n(ok.l, 4), n(ok.c, 4), n(ok.h, 2))))),
        ("CMYK", format!("device-cmyk({}% {}% {}% {}%)", n(cc, 1), n(m, 1), n(y, 1), n(k, 1))),
        ("CSS", c.to_css()),
    ]
}

/// "Copy as": a button that lists the colour in every format; a click copies one.
fn copy_as(app: &mut FigApp, ui: &mut Ui, c: Color, profile: ColorProfile) {
    let resp = widgets::dropdown(ui, tr("Copy as"), 96.0, None);
    crate::right::menu_popup(&resp).width(260.0).show(|ui| {
        for (label, text) in formats(c, profile) {
            if crate::right::menu_item(ui, &text, label) {
                crate::egui_copy_text(ui.ctx(), &text);
                app.toast(format!("{} {text}", tr("Copied")));
                ui.close();
            }
        }
    });
}

// ---- contrast ------------------------------------------------------------------------------------------

/// APCA lightness contrast (Lc, APCA-W3 0.0.98G) of `text` on `bg`, both opaque sRGB: positive for
/// dark text on a light background, negative for light on dark.
pub(crate) fn apca(text: Color, bg: Color) -> f64 {
    let y = |c: Color| {
        let ch = |v: f32| f64::from(v.clamp(0.0, 1.0)).powf(2.4);
        let y = 0.2126729 * ch(c.r) + 0.7151522 * ch(c.g) + 0.0721750 * ch(c.b);
        if y < 0.022 { y + (0.022 - y).powf(1.414) } else { y }
    };
    let (yt, yb) = (y(text), y(bg));
    if (yb - yt).abs() < 0.0005 {
        return 0.0;
    }
    let out = if yb > yt {
        let s = (yb.powf(0.56) - yt.powf(0.57)) * 1.14;
        if s < 0.1 { 0.0 } else { s - 0.027 }
    } else {
        let s = (yb.powf(0.65) - yt.powf(0.62)) * 1.14;
        if s > -0.1 { 0.0 } else { s + 0.027 }
    };
    out * 100.0
}

/// What a layer's colour sits on: the nearest frame (or other ancestor) with a visible, opaque
/// solid fill, else the page's background.
fn backdrop(doc: &Document, id: Option<NodeId>) -> Color {
    let page_bg = doc.pages.iter().find_map(|p| match &doc.node(*p)?.kind {
        figcraft_doc::NodeKind::Page { background, .. } if Some(*p) == doc.current_page => Some(*background),
        _ => None,
    });
    let page_bg = page_bg.unwrap_or(Color::WHITE);
    let Some(id) = id else { return page_bg };
    for a in doc.ancestors(id) {
        let Some(n) = doc.node(a) else { continue };
        if n.is_page() {
            if let figcraft_doc::NodeKind::Page { background, .. } = &n.kind {
                return *background;
            }
        }
        let fill = n.fills.iter().rev().find(|p| p.visible && p.opacity >= 0.999 && matches!(p.kind, PaintKind::Solid { .. }));
        if let Some(PaintKind::Solid { color }) = fill.map(|p| &p.kind) {
            return color.with_alpha(1.0);
        }
    }
    page_bg
}

/// `c` seen over `bg` (alpha composited).
fn over(c: Color, bg: Color) -> Color {
    let a = c.a.clamp(0.0, 1.0);
    Color::new(c.r * a + bg.r * (1.0 - a), c.g * a + bg.g * (1.0 - a), c.b * a + bg.b * (1.0 - a), 1.0)
}

/// The WCAG level a ratio reaches for normal text.
fn level(ratio: f32) -> &'static str {
    if ratio >= 7.0 {
        "AAA"
    } else if ratio >= 4.5 {
        "AA"
    } else if ratio >= 3.0 {
        "AA Large"
    } else {
        "Fail"
    }
}

/// Under the picker's fields: "Copy as" and the contrast against the colour's backdrop.
pub(crate) fn below_fields(app: &mut FigApp, ui: &mut Ui, c: Color, target: &PickerTarget) {
    let Ok(doc) = app.session.doc() else { return };
    let profile = doc.color_profile;
    let layer = match target {
        PickerTarget::Paint { ids, .. } | PickerTarget::Effect { ids, .. } => ids.first().copied(),
        PickerTarget::TextRange { id, .. } => Some(*id),
        _ => None,
    };
    let bg = backdrop(doc, layer);
    let fg = over(c, bg);
    let ratio = fg.contrast(bg);
    let lc = apca(fg, bg);
    let t = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        copy_as(app, ui, c, profile);
        ui.add_space(4.0);
        let (sw, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
        ui.painter().rect_filled(sw, egui::CornerRadius::same(3), crate::theme::doc_c32(ui.ctx(), bg));
        ui.painter().rect_stroke(sw, egui::CornerRadius::same(3), egui::Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
        let text = format!("{}:1 {} · Lc {}", n(f64::from(ratio), 2), level(ratio), n(lc, 0));
        let tip = format!("{} {}", tr("Contrast with"), bg.to_css_hex());
        let col = if ratio >= 4.5 { t.text_secondary } else { t.warning };
        ui.label(egui::RichText::new(text).font(crate::theme::regular(10.5)).color(col)).on_hover_text(tip);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmyk_round_trips_and_matches_the_pdf_export() {
        let c = Color::from_hex("#EF3D88").unwrap();
        let k = cmyk_of(c);
        assert_eq!(k.map(|x| (x * 100.0).round()), [0.0, 74.0, 43.0, 6.0]);
        assert_eq!(from_cmyk(k, 1.0).to_hex(), "EF3D88");
        assert_eq!(cmyk_of(Color::BLACK), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(cmyk_of(Color::WHITE), [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn contrast_wcag_and_apca() {
        let (b, w) = (Color::BLACK, Color::WHITE);
        assert!((b.contrast(w) - 21.0).abs() < 0.01);
        // APCA's published reference values: black on white 106.04, white on black -107.88.
        assert!((apca(b, w) - 106.04).abs() < 0.1, "{}", apca(b, w));
        assert!((apca(w, b) + 107.88).abs() < 0.1, "{}", apca(w, b));
        assert_eq!(level(4.6), "AA");
        let half = over(Color::new(0.0, 0.0, 0.0, 0.5), w);
        assert_eq!(half.to_hex(), "808080");
    }

    #[test]
    fn copy_as_lists_every_format() {
        let f = formats(Color::from_hex("#3A81F6").unwrap(), ColorProfile::Srgb);
        let get = |l: &str| f.iter().find(|(k, _)| *k == l).map(|(_, v)| v.clone()).unwrap();
        assert_eq!(get("Hex"), "#3A81F6");
        assert_eq!(get("RGB"), "rgb(58 129 246)");
        assert!(get("OKLCH").starts_with("oklch(0.6"), "{}", get("OKLCH"));
        assert!(get("CMYK").starts_with("device-cmyk(76.4% 47.6% 0% 3.5%"), "{}", get("CMYK"));
        assert_eq!(f.len(), 8);
    }
}
