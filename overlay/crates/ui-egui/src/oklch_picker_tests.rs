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
    let n = nodes
        .iter()
        .map(|(_, n)| n)
        .find(|n| matches!(n.role(), Role::SpinButton | Role::TextInput) && n.label() == Some(name))
        .unwrap_or_else(|| panic!("no field {name:?}"));
    let b = n.bounds().expect("bounds");
    (pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32), n.value().unwrap_or_default().to_string())
}

fn type_into(u: &mut Ui, name: &str, text: &str) {
    let (p, _) = field(u, name);
    u.button(p, true);
    u.button(p, false);
    let key = |k, m, pressed| Event::Key { key: k, physical_key: None, pressed, repeat: false, modifiers: m };
    u.frame(vec![key(Key::A, Modifiers::COMMAND, true), key(Key::A, Modifiers::COMMAND, false)]);
    u.frame(vec![Event::Text(text.into())]);
    u.frame(vec![key(Key::Enter, Modifiers::NONE, true), key(Key::Enter, Modifiers::NONE, false)]);
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
    // The values oklch.com shows for #EF3D88: oklch(0.6456 0.2197 0.14).
    let (mut u, a) = setup("#EF3D88");
    assert_eq!(field(&mut u, "OKLCH lightness").1, "0.6456");
    assert_eq!(field(&mut u, "OKLCH chroma").1, "0.2197");
    assert_eq!(field(&mut u, "OKLCH hue").1, "0.14");
    // sRGB red is oklch(0.628 0.2577 29.23).
    let (mut u, a2) = setup("#FF0000");
    assert_eq!(field(&mut u, "OKLCH lightness").1, "0.628");
    assert_eq!(field(&mut u, "OKLCH chroma").1, "0.2577");
    assert_eq!(field(&mut u, "OKLCH hue").1, "29.23");
    // A new hue: the fields then show the colour as it is (chroma mapped into sRGB), not 0.2577.
    type_into(&mut u, "OKLCH hue", "260");
    let got = Oklch::from_color(fill(&u, a2), ColorProfile::Srgb);
    assert!((got.h - 260.0).abs() < 4.0 && (got.l - 0.628).abs() < 0.03, "{got:?}");
    assert_eq!(field(&mut u, "OKLCH chroma").1, format!("{}", (got.c * 10000.0).round() / 10000.0));
    // L and C take CSS forms too.
    type_into(&mut u, "OKLCH lightness", "50%");
    let now = Oklch::from_color(fill(&u, a2), ColorProfile::Srgb);
    // (out of sRGB at this chroma: CSS gamut mapping may move L by up to its 0.02 JND)
    assert!((now.l - 0.5).abs() < 0.01, "{now:?} shown {:?}", field(&mut u, "OKLCH lightness").1);
    type_into(&mut u, "OKLCH chroma", "0,1");
    assert!((Oklch::from_color(fill(&u, a2), ColorProfile::Srgb).c - 0.1).abs() < 0.002);
    // A grey keeps its hue while the chroma is 0.
    let h = field(&mut u, "OKLCH hue").1;
    type_into(&mut u, "OKLCH chroma", "0");
    assert_eq!(field(&mut u, "OKLCH hue").1, h);
    let grey = fill(&u, a2);
    assert!((grey.r - grey.g).abs() < 0.01 && (grey.g - grey.b).abs() < 0.01, "{grey:?}");
    let _ = a;
}

#[test]
fn a_display_p3_file_reads_and_writes_p3_values() {
    let (mut u, a) = setup("#FF0000");
    u.app.session.execute("file.colorProfile", &json!({"profile": "displayP3", "convert": false})).unwrap();
    u.frame(vec![]);
    // P3 red is wider: oklch(0.6486 0.2995 28.96).
    assert_eq!(field(&mut u, "OKLCH lightness").1, "0.6486");
    assert_eq!(field(&mut u, "OKLCH chroma").1, "0.2995");
    type_into(&mut u, "OKLCH lightness", "0.5");
    let got = Oklch::from_color(fill(&u, a), ColorProfile::DisplayP3);
    assert!((got.l - 0.5).abs() < 0.02 && (got.h - 28.96).abs() < 4.0, "{got:?}");
}

#[test]
fn the_picker_widens_for_oklch_and_remembers_a_dragged_width() {
    let (u, _) = setup("#FF0000");
    let models = ["Hex", "RGB", "HSL", "HSB", "OKLCH", "CSS"];
    let w = crate::oklch_fields::picker_width(&u.ctx, &models);
    assert!(w >= 276.0, "OKLCH needs room for its fields: {w}");
    u.ctx.data_mut(|d| d.insert_temp(egui::Id::new("cp_model"), 0usize));
    assert!(crate::oklch_fields::picker_width(&u.ctx, &models) < w, "Hex needs less");
    u.ctx.data_mut(|d| d.insert_persisted(egui::Id::new("cp_width"), 400.0f32));
    assert_eq!(crate::oklch_fields::picker_width(&u.ctx, &models), 400.0);
}

#[test]
fn a_colour_variable_shows_its_links_and_follows_a_linked_hue() {
    let mut s = Session::new();
    let c = s.execute("variable.createCollection", &json!({"name": "Theme"})).unwrap()["id"].as_str().unwrap().to_string();
    s.execute("variable.create", &json!({"collection": c, "name": "Hue", "type": "number", "value": 280})).unwrap();
    let p = s.execute("variable.create", &json!({"collection": c, "name": "Primary", "type": "color", "value": "#3366FF"})).unwrap()["id"].as_str().unwrap().to_string();
    s.execute("variable.update", &json!({"variable": p, "description": "oklch(62% 0.15 {Hue})"})).unwrap();
    let mode = s.doc().unwrap().variables.collection(&c).unwrap().modes[0].id.clone();
    let mut u = Ui::new(s);
    u.ctx.enable_accesskit();
    u.app.ui.picker = Some(PickerTarget::Variable { id: p.clone(), mode });
    u.ctx.data_mut(|d| d.insert_temp(egui::Id::new("cp_model"), 4usize));
    u.frame(vec![]);
    let nodes = u.frame_output(vec![]).platform_output.accesskit_update.unwrap().nodes;
    let texts: Vec<String> = nodes.iter().filter_map(|(_, n)| n.label().or(n.value()).map(str::to_string)).collect();
    assert!(texts.iter().any(|t| t.contains("H: Hue")), "{texts:?}");
    assert!(texts.iter().any(|t| t.contains("L: —")), "{texts:?}");
    // The hue field shows the linked value.
    assert!((field(&mut u, "OKLCH hue").1.parse::<f64>().unwrap() - 280.0).abs() < 0.5);
    u.app.run("variable.set", json!({"variable": "Hue", "value": 130}));
    assert!((field(&mut u, "OKLCH hue").1.parse::<f64>().unwrap() - 130.0).abs() < 0.5);
}

/// Two frames of `draw`: the second with a click at `at`; what the colour became.
fn click_in(draw: impl Fn(&mut egui::Ui, &mut figcraft_color::Hsb) -> (bool, bool), start: figcraft_color::Color, at: Pos2) -> figcraft_color::Color {
    let ctx = egui::Context::default();
    let mut hsb = figcraft_color::Hsb::from_color(start);
    let mut t = 0.0;
    for events in [
        vec![],
        vec![Event::PointerMoved(at), Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }],
        vec![Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }],
    ] {
        t += 0.1;
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0))), time: Some(t), events, ..Default::default() };
        let mut out = ctx.run_ui(input, |ui| {
            draw(ui, &mut hsb);
        });
        out.textures_delta.clear();
    }
    hsb.to_color()
}

#[test]
fn the_oklch_plane_sets_lightness_and_chroma_and_the_hue_strip_keeps_lightness() {
    let start = figcraft_color::Color::from_hex("#3366FF").unwrap();
    let before = Oklch::from_color(start, ColorProfile::Srgb);
    // Plane: 400 wide = chroma 0–0.4, 200 high = lightness 100%–0. A quarter across, halfway down.
    let plane = egui::Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(400.0, 200.0));
    let got = click_in(|ui, hsb| crate::oklch_fields::plane(ui, plane, hsb, start, ColorProfile::Srgb), start, pos2(100.0, 100.0));
    let o = Oklch::from_color(got, ColorProfile::Srgb);
    assert!((o.l - 0.5).abs() < 0.01 && (o.c - 0.1).abs() < 0.005 && (o.h - before.h).abs() < 1.0, "{o:?} from {before:?}");
    // Hue strip: 360 wide. Hue 130 at the same lightness (chroma mapped into sRGB if needed).
    let strip = egui::Rect::from_min_size(pos2(0.0, 300.0), egui::vec2(360.0, 20.0));
    let got = click_in(|ui, hsb| crate::oklch_fields::hue_strip(ui, strip, hsb, start, ColorProfile::Srgb), start, pos2(130.0, 310.0));
    let o = Oklch::from_color(got, ColorProfile::Srgb);
    assert!((o.l - before.l).abs() < 0.02 && (o.h - 130.0).abs() < 4.0, "{o:?} from {before:?}");
}
