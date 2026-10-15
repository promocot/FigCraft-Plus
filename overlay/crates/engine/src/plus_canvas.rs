//! FigCraft Plus: picking frames on the canvas.
//!
//! - A press anywhere on a selected layer (a frame, a button made with auto layout, a group) keeps
//!   it and drags it as a whole, its text and other layers included; a double-click enters it, and
//!   Cmd/Ctrl-click still picks the layer under the pointer.
//! - A click (no drag) on the empty area of a top-level frame selects the frame; dragging from
//!   there still draws a marquee over its layers.

use figcraft_doc::geometry::select_target;
use figcraft_doc::{Document, NodeId};
use figcraft_geom::Point;

use crate::Session;
use crate::tools::PointerEvent;

/// Screen pixels the pointer may move and still make a click (as the Move tool's own drag slop).
const CLICK_SLOP_PX: f64 = 3.0;

/// What a press on `leaf` picks: the selected layer holding it (so it moves as a whole), else the
/// usual click target. Cmd/Ctrl (`deep`) and Shift keep the usual rules.
pub(crate) fn press_target(doc: &Document, leaf: NodeId, sel: &[NodeId], deep: bool, shift: bool) -> NodeId {
    if !deep && !shift {
        // The outermost selected layer holding the leaf.
        let mut chain = doc.ancestors(leaf);
        chain.reverse();
        chain.push(leaf);
        if let Some(held) = chain.into_iter().find(|c| sel.contains(c)) {
            return held;
        }
    }
    select_target(doc, leaf, sel, deep)
}

/// The end of a marquee gesture: when the pointer hardly moved and it started on the empty area of
/// top-level frame `within` (outside the focus view), that frame is selected (Shift: toggled).
pub(crate) fn marquee_click(s: &mut Session, ev: &PointerEvent, start: Point, additive: bool, base: Vec<NodeId>, within: Option<NodeId>) {
    let Some(board) = within else { return };
    if (ev.p() - start).hypot() * ev.zoom.max(1e-6) >= CLICK_SLOP_PX {
        return;
    }
    let Ok(ds) = s.ds() else { return };
    if ds.focused().is_some() || ds.doc.node(board).is_none_or(|n| n.locked) {
        return;
    }
    let sel = if additive {
        let had = base.contains(&board);
        let mut sel: Vec<NodeId> = base.into_iter().filter(|b| *b != board).collect();
        if !had {
            sel.push(board);
        }
        sel
    } else {
        vec![board]
    };
    s.set_selection(sel);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::PointerKind;
    use serde_json::json;

    fn ex(s: &mut Session, cmd: &str, p: serde_json::Value) -> NodeId {
        let r = s.execute(cmd, &p).unwrap_or_else(|e| panic!("{cmd}: {e}"));
        crate::params::id_of(&r["id"]).unwrap()
    }

    /// A frame (0,0 400×300) with a rectangle in it, and a button: an auto layout frame at (500, 0)
    /// holding a rectangle (its label).
    fn session() -> (Session, NodeId, NodeId, NodeId) {
        let mut s = Session::new();
        let board = ex(&mut s, "frame.create", json!({"x": 0, "y": 0, "width": 400, "height": 300}));
        ex(&mut s, "shape.rectangle", json!({"x": 20, "y": 20, "width": 50, "height": 50, "parent": board}));
        let button = ex(&mut s, "frame.create", json!({"x": 500, "y": 0, "width": 120, "height": 40}));
        let label = ex(&mut s, "shape.rectangle", json!({"x": 510, "y": 10, "width": 60, "height": 20, "parent": button}));
        s.execute("layout.add", &json!({"ids": [button]})).unwrap();
        (s, board, button, label)
    }

    fn click(s: &mut Session, x: f64, y: f64) {
        s.pointer(&PointerEvent::at(PointerKind::Down, x, y, 1.0)).unwrap();
        s.pointer(&PointerEvent::at(PointerKind::Up, x, y, 1.0)).unwrap();
    }

    #[test]
    fn a_click_on_a_frames_empty_area_selects_it_and_a_drag_draws_a_marquee() {
        let (mut s, board, _, _) = session();
        s.set_selection(vec![]);
        click(&mut s, 300.0, 200.0);
        assert_eq!(s.selection(), vec![board]);
        // From nothing selected, a drag over its layers selects them, not the frame.
        s.set_selection(vec![]);
        s.pointer(&PointerEvent::at(PointerKind::Down, 300.0, 200.0, 1.0)).unwrap();
        s.pointer(&PointerEvent::at(PointerKind::Move, 10.0, 10.0, 1.0)).unwrap();
        s.pointer(&PointerEvent::at(PointerKind::Up, 10.0, 10.0, 1.0)).unwrap();
        assert!(!s.selection().is_empty());
        assert!(!s.selection().contains(&board));
    }

    #[test]
    fn a_selected_button_moves_as_a_whole_from_its_label() {
        let (mut s, _, button, label) = session();
        let at = s.doc().unwrap().world_bounds(label).unwrap();
        let (x, y) = ((at.x0 + at.x1) / 2.0, (at.y0 + at.y1) / 2.0);
        s.set_selection(vec![button]);
        s.pointer(&PointerEvent::at(PointerKind::Down, x, y, 1.0)).unwrap();
        s.pointer(&PointerEvent::at(PointerKind::Move, x + 40.0, y + 30.0, 1.0)).unwrap();
        s.pointer(&PointerEvent::at(PointerKind::Up, x + 40.0, y + 30.0, 1.0)).unwrap();
        assert_eq!(s.selection(), vec![button]);
        let doc = s.doc().unwrap();
        assert_eq!(doc.node(label).and_then(|n| n.parent), Some(button), "the label stays in the button");
        let b = doc.world_bounds(button).unwrap();
        assert_eq!((b.x0, b.y0), (540.0, 30.0), "the button moved");
    }
}
