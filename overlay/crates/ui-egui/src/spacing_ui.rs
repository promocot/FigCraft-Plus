//! FigCraft Plus: the Layout section's Spacing field for several selected layers, as in Figma.
//!
//! When the selected layers sit in a column (one under another, not overlapping vertically) or
//! in a row (side by side), the field shows the gap between them ("Mixed" when the gaps differ).
//! Typing a value, or scrubbing, spaces them evenly by it from the first one
//! (`distribute.vertical` / `distribute.horizontal` with `spacing`).

use egui::{Id, Ui};
use figcraft_doc::{Document, NodeId};
use serde_json::json;

use crate::FigApp;
use crate::i18n::tr;
use crate::icons::Icon;
use crate::widgets::{self, Lead};

/// How the layers line up: in a column (true) or a row (false), and the gap between them when it
/// is the same everywhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Arrangement {
    pub vertical: bool,
    pub gap: Option<f64>,
}

/// The arrangement of boxes `[x0, y0, x1, y1]`: a column when, top to bottom, none overlaps the
/// next vertically; else a row when, left to right, none overlaps the next horizontally. When both
/// hold (a diagonal), the axis they spread along more.
pub(crate) fn arrangement(boxes: &[[f64; 4]]) -> Option<Arrangement> {
    if boxes.len() < 2 {
        return None;
    }
    let along = |vertical: bool| -> Option<Vec<f64>> {
        let (a0, a1) = if vertical { (1, 3) } else { (0, 2) };
        let mut b: Vec<&[f64; 4]> = boxes.iter().collect();
        b.sort_by(|p, q| p[a0].total_cmp(&q[a0]));
        let gaps: Vec<f64> = b.windows(2).filter_map(|w| Some(w.get(1)?[a0] - w.first()?[a1])).collect();
        // Touching counts as a gap of 0; overlapping by a hair (rounding) too.
        gaps.iter().all(|g| *g >= -0.01).then_some(gaps)
    };
    let span = |vertical: bool| {
        let (a0, a1) = if vertical { (1, 3) } else { (0, 2) };
        let lo = boxes.iter().map(|b| b[a0]).fold(f64::INFINITY, f64::min);
        let hi = boxes.iter().map(|b| b[a1]).fold(f64::NEG_INFINITY, f64::max);
        hi - lo
    };
    let (vertical, gaps) = match (along(true), along(false)) {
        (Some(v), Some(h)) => {
            if span(true) >= span(false) {
                (true, v)
            } else {
                (false, h)
            }
        }
        (Some(v), None) => (true, v),
        (None, Some(h)) => (false, h),
        (None, None) => return None,
    };
    let round = |g: f64| (g.max(0.0) * 100.0).round() / 100.0;
    let first = round(*gaps.first()?);
    let gap = gaps.iter().all(|g| (round(*g) - first).abs() < 0.011).then_some(first);
    Some(Arrangement { vertical, gap })
}

/// The selected layers that move on their own: not inside another selected one, not placed by an
/// auto layout frame.
fn movable(doc: &Document, sel: &[NodeId]) -> Option<Vec<NodeId>> {
    let ids: Vec<NodeId> = sel.iter().copied().filter(|i| !sel.iter().any(|o| o != i && doc.is_ancestor(*o, *i))).collect();
    let placed = ids.iter().any(|i| {
        doc.node(*i).is_some_and(|n| !n.child_layout.absolute && n.parent.and_then(|p| doc.node(p)).is_some_and(|p| p.layout.is_some()))
    });
    (!placed && ids.len() >= 2).then_some(ids)
}

/// The Spacing row under Dimensions (several layers in a row or column only).
pub(crate) fn show(app: &mut FigApp, ui: &mut Ui, doc: &Document, sel: &[NodeId]) {
    let Some(ids) = movable(doc, sel) else { return };
    let boxes: Vec<[f64; 4]> = ids.iter().filter_map(|i| doc.world_bounds(*i)).map(|b| [b.x0, b.y0, b.x1, b.y1]).collect();
    if boxes.len() != ids.len() {
        return;
    }
    let Some(a) = arrangement(&boxes) else { return };
    let col = widgets::col_width(ui.available_width());
    widgets::sublabels(ui, tr("Spacing"), None);
    crate::right::row(ui, 32.0, |ui| {
        let (icon, name, cmd) = if a.vertical {
            (Icon::DistributeV, tr("Vertical spacing"), "distribute.vertical")
        } else {
            (Icon::DistributeH, tr("Horizontal spacing"), "distribute.horizontal")
        };
        let edit = widgets::number_field_mixed(ui, Id::new(("plus_spacing", a.vertical)), Lead::Icon(icon, name), a.gap, "", col, 1.0);
        app.field_run(edit.and_then(widgets::Edit::settled), cmd, |v| json!({"ids": ids, "spacing": v}));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_column_a_row_or_neither() {
        // Two circles one under another, 96 apart (the Figma example).
        let col = [[0.0, 0.0, 268.0, 268.0], [0.0, 364.0, 250.0, 614.0]];
        assert_eq!(arrangement(&col), Some(Arrangement { vertical: true, gap: Some(96.0) }));
        let row = [[0.0, 0.0, 10.0, 10.0], [30.0, 0.0, 40.0, 10.0], [60.0, 0.0, 70.0, 10.0]];
        assert_eq!(arrangement(&row), Some(Arrangement { vertical: false, gap: Some(20.0) }));
        let uneven = [[0.0, 0.0, 10.0, 10.0], [20.0, 0.0, 30.0, 10.0], [50.0, 0.0, 60.0, 10.0]];
        assert_eq!(arrangement(&uneven), Some(Arrangement { vertical: false, gap: None }));
        let overlapping = [[0.0, 0.0, 100.0, 100.0], [50.0, 50.0, 150.0, 150.0]];
        assert_eq!(arrangement(&overlapping), None);
        assert_eq!(arrangement(&[[0.0, 0.0, 1.0, 1.0]]), None);
    }

    #[test]
    fn the_spacing_spaces_the_layers_evenly() {
        let mut s = figcraft_engine::Session::new();
        let mut ids = Vec::new();
        for y in [0, 150, 400] {
            let r = s.execute("shape.rectangle", &json!({"x": 0, "y": y, "width": 100, "height": 100})).unwrap();
            ids.push(figcraft_engine::params::id_of(&r["id"]).unwrap());
        }
        s.execute("distribute.vertical", &json!({"ids": ids, "spacing": 24})).unwrap();
        let doc = s.doc().unwrap();
        let boxes: Vec<[f64; 4]> = ids.iter().filter_map(|i| doc.world_bounds(*i)).map(|b| [b.x0, b.y0, b.x1, b.y1]).collect();
        assert_eq!(arrangement(&boxes), Some(Arrangement { vertical: true, gap: Some(24.0) }));
        assert_eq!(boxes[0][1], 0.0, "the first stays put");
    }
}
