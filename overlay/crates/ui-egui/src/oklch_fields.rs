//! The colour picker's OKLCH fields: lightness (%), chroma (% of 0.4, as CSS writes it) and hue
//! (degrees), read and written in the document's colour profile. A colour outside the profile's
//! gamut is mapped in (see `figcraft_color::Oklch::to_color`).

use egui::{Id, Ui};
use figcraft_color::{Color, ColorProfile, OKLCH_FULL_CHROMA, Oklch};

use crate::widgets;

/// The three fields for colour `c`: the committed colour when one was edited.
pub(crate) fn fields(ui: &mut Ui, c: Color, profile: ColorProfile) -> Option<Color> {
    // The last OKLCH typed for this colour: keeps the exact values (and the hue of a grey) that
    // the RGB round trip would blur.
    let cache = Id::new("cp_oklch");
    let rgba = c.to_rgba8();
    let lch = ui
        .data(|d| d.get_temp::<([u8; 4], f64, f64, f64)>(cache))
        .filter(|(k, ..)| *k == rgba)
        .map(|(_, l, ch, h)| Oklch::new(l, ch, h))
        .unwrap_or_else(|| Oklch::from_color(c, profile));
    let vals = [lch.l * 100.0, lch.c / OKLCH_FULL_CHROMA * 100.0, lch.h];
    let mut out = None;
    for (k, v) in vals.iter().enumerate() {
        let (id, name, step) = [("ok_l", "OKLCH lightness", 0.5), ("ok_c", "OKLCH chroma", 0.5), ("ok_h", "OKLCH hue", 1.0)][k.min(2)];
        let shown = Some((*v * 100.0).round() / 100.0);
        let edit = widgets::number_field(ui, Id::new(("cp_f", id)), widgets::Lead::Named(name), shown, "", 36.0, step);
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
