//! The colour picker's OKLCH model, driven through egui headless: it shows a fill's OKLCH, an
//! edited field sets the fill (gamut-mapped into the file's profile) and keeps the typed values.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, Pos2, pos2};
use figcraft_color::{ColorProfile, Oklch, PaintKind};
use figcraft_doc::NodeId;
use figcraft_engine::Session;
use serde_json::json;

use crate::colorpicker::PickerTarget;
use crate::layers_drag_tests::Ui;

fn setup(hex: &str) -> (Ui, NodeId) {
    let mut s = Session::new();
    let a = NodeId(s.execute("shape.rectangle", &json!({"x": 10, "y": 20, "width": 40, "height": 30, "props": {"fill": hex}})).unwrap()["id"].as_u64().unwrap());
    s.execute("select.set", &json!({"ids": [a]})).unwrap();
    let mut u = Ui::new(s);
    u.ctx.enable_accesskit();
    u.app.ui.picker = Some(PickerTarget::Paint { fills: true, index: 0, ids: vec![a] });
    // The OKLCH model (index 4 of the model dropdown).
    u.ctx.data_mut(|d| d.insert_temp(egui::Id::new("cp_model"), 4usize));
    u.frame(vec![]);
    (u, a)
}

fn field(u: &mut Ui, name: &str) -> (Pos2, String) {
    u.frame(vec![]);
    let nodes = u.frame_output(vec![]).platform_output.accesskit_update.expect("an AccessKit update").nodes;
    let n = nodes.iter().map(|(_, n)| n).find(|n| n.role() == Role::SpinButton && n.label() == Some(name)).unwrap_or_else(|| panic!("no field {name:?}"));
    let b = n.bounds().expect("bounds");
    (pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32), n.value().unwrap_or_default().to_string())
}

fn type_into(u: &mut Ui, name: &str, text: &str) {
    let (p, _) = field(u, name);
    u.button(p, true);
    u.button(p, false);
    u.frame(vec![Event::Text(text.into())]);
    let key = |pressed| Event::Key { key: Key::Enter, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE };
    u.frame(vec![key(true), key(false)]);
    u.frame(vec![]);
}

fn fill(u: &Ui, a: NodeId) -> figcraft_color::Color {
    match u.app.session.doc().unwrap().node(a).unwrap().fills.first().map(|p| &p.kind) {
        Some(PaintKind::Solid { color }) => *color,
        other => panic!("not a solid fill: {other:?}"),
    }
}

#[test]
fn the_oklch_model_shows_and_sets_a_fill() {
    let (mut u, a) = setup("#FF0000");
    // sRGB red is oklch(62.8% 0.2577 29.23): C shows as % of 0.4.
    assert_eq!(field(&mut u, "OKLCH lightness").1, "62.8");
    assert_eq!(field(&mut u, "OKLCH chroma").1, "64.42");
    assert_eq!(field(&mut u, "OKLCH hue").1, "29.23");
    // A new hue keeps L and C where they fit.
    type_into(&mut u, "OKLCH hue", "260");
    let got = Oklch::from_color(fill(&u, a), ColorProfile::Srgb);
    assert!((got.h - 260.0).abs() < 4.0, "{got:?}");
    assert!((got.l - 0.628).abs() < 0.03, "{got:?}");
    // The fields keep what was typed, not the RGB round trip (C was gamut-mapped down).
    assert_eq!(field(&mut u, "OKLCH hue").1, "260");
    assert_eq!(field(&mut u, "OKLCH chroma").1, "64.42");
    // A grey keeps its hue while the chroma is 0.
    type_into(&mut u, "OKLCH chroma", "0");
    assert_eq!(field(&mut u, "OKLCH hue").1, "260");
    let grey = fill(&u, a);
    assert!((grey.r - grey.g).abs() < 0.01 && (grey.g - grey.b).abs() < 0.01, "{grey:?}");
}

#[test]
fn a_display_p3_file_reads_and_writes_p3_values() {
    let (mut u, a) = setup("#FF0000");
    u.app.session.execute("file.colorProfile", &json!({"profile": "displayP3", "convert": false})).unwrap();
    u.frame(vec![]);
    // P3 red is wider: oklch(64.86% 0.2995 28.96).
    assert_eq!(field(&mut u, "OKLCH lightness").1, "64.86");
    type_into(&mut u, "OKLCH lightness", "50");
    let got = Oklch::from_color(fill(&u, a), ColorProfile::DisplayP3);
    assert!((got.l - 0.5).abs() < 0.02 && (got.h - 28.96).abs() < 4.0, "{got:?}");
}
