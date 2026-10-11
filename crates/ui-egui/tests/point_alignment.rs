//! Horizontal/Vertical accepts either one line or two distinct sketch points.

use serde_json::json;
use solvecraft_engine::sketch::ConstraintKind;
use solvecraft_ui_egui::scenario::Harness;

#[test]
fn horizontal_vertical_aligns_origin_and_circle_center() {
    for (center, horizontal) in [([20.0, 3.0], true), ([3.0, 20.0], false)] {
        for origin_first in [true, false] {
            let mut h = Harness::new();
            h.run(&json!([
                {"call":"engine.execute","params":{"command":"sketch.create","params":{"plane":"XY"}}},
                {"call":"engine.execute","params":{"command":"sketch.circle.center","params":{"center":center,"radius":5}}},
                {"call":"ui.view","params":{"view":"top"}},
                {"start":"sketch.constraint.horizontal_vertical"}
            ]))
            .unwrap();
            let (first, second) = if origin_first { ([0.0, 0.0], center) } else { (center, [0.0, 0.0]) };
            let undo = h.app.session.undo.len();
            h.run(&json!([{"click":{"sketch":first}}, {"expect":{"errors":0}}])).unwrap();
            assert_eq!(h.app.tool.as_ref().unwrap().picks.len(), 1);
            assert!(!h.app.status.as_ref().is_some_and(|(_, _, error)| *error));
            assert_eq!(h.app.session.undo.len(), undo);
            // Repeating the first point must not complete a self-constraint.
            h.run(&json!([{"click":{"sketch":first}}, {"click":{"sketch":second}}, {"expect":{"errors":0}}])).unwrap();
            let aligned = |h: &Harness| {
                let sk = &h.app.session.model.state().sketches[0].sketch;
                let p = sk.point(sk.resolve_point("c1.center").unwrap()).unwrap();
                if horizontal { p.y } else { p.x }
            };
            assert!(aligned(&h).abs() < 1e-6);
            let sk = &h.app.session.model.state().sketches[0].sketch;
            assert!(sk.constraints.iter().any(|c| if horizontal {
                matches!(c.kind, ConstraintKind::HorizontalPoints { .. })
            } else {
                matches!(c.kind, ConstraintKind::VerticalPoints { .. })
            }));
            assert_eq!(h.app.session.undo.len(), undo + 1);
            assert!(h.app.tool.as_ref().unwrap().picks.is_empty());
            h.run(&json!([{"call":"engine.execute","params":{"command":"edit.undo","params":{}}}])).unwrap();
            assert!((aligned(&h) - 3.0).abs() < 1e-6);
            h.run(&json!([{"call":"engine.execute","params":{"command":"edit.redo","params":{}}}])).unwrap();
            assert!(aligned(&h).abs() < 1e-6);
        }
    }
}

#[test]
fn horizontal_vertical_still_accepts_one_line() {
    let mut h = Harness::new();
    h.run(&json!([
        {"call":"engine.execute","params":{"command":"sketch.create","params":{"plane":"XY"}}},
        {"call":"engine.execute","params":{"command":"sketch.line","params":{"points":[[10,10],[40,13]],"infer":false}}},
        {"call":"ui.view","params":{"view":"top"}},
        {"start":"sketch.constraint.horizontal_vertical"},
        {"click":{"sketch":[25,11.5]}},
        {"expect":{"errors":0}}
    ]))
    .unwrap();
    let sk = &h.app.session.model.state().sketches[0].sketch;
    assert!(sk.constraints.iter().any(|c| matches!(c.kind, ConstraintKind::Horizontal { .. })));
    let a = sk.point(sk.resolve_point("l1.start").unwrap()).unwrap();
    let b = sk.point(sk.resolve_point("l1.end").unwrap()).unwrap();
    assert!((a.y - b.y).abs() < 1e-6);
}

#[test]
fn horizontal_vertical_ignores_curves_and_cancels_a_pending_point() {
    let mut h = Harness::new();
    h.run(&json!([
        {"call":"engine.execute","params":{"command":"sketch.create","params":{"plane":"XY"}}},
        {"call":"engine.execute","params":{"command":"sketch.circle.center","params":{"center":[20,3],"radius":5}}},
        {"call":"ui.view","params":{"view":"top"}},
        {"start":"sketch.constraint.horizontal_vertical"},
        {"click":{"sketch":[25,3]}}
    ]))
    .unwrap();
    assert!(h.app.tool.as_ref().unwrap().picks.is_empty());
    assert!(!h.app.status.as_ref().is_some_and(|(_, _, error)| *error));
    let undo = h.app.session.undo.len();
    h.run(&json!([{"click":{"sketch":[0,0]}}, {"click":{"sketch":[25,3]}}])).unwrap();
    assert_eq!(h.app.tool.as_ref().unwrap().picks.len(), 1);
    h.run(&json!([{"key":"Escape"}])).unwrap();
    assert!(h.app.tool.is_none());
    assert_eq!(h.app.session.undo.len(), undo);
    let sk = &h.app.session.model.state().sketches[0].sketch;
    let p = sk.point(sk.resolve_point("c1.center").unwrap()).unwrap();
    assert!((p.y - 3.0).abs() < 1e-6);
}
