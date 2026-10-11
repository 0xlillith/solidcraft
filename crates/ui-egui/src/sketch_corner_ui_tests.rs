use super::*;
use solvecraft_engine::sketch::CurveKind;

#[test]
fn clicked_extend_endpoint_reaches_exact_ellipse_and_preserves_support() {
    let mut h = crate::scenario::Harness::new();
    h.app.ui.show_origin = true;
    h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
    h.app.run("sketch.ellipse", json!({"center":[20,0],"major":[30,0],"minor_radius":5})).unwrap();
    h.app.run("sketch.constraint.fix", json!({"entity":"e1"})).unwrap();
    h.app.run("sketch.line", json!({"points":[[0,3],[5,3]],"connect":false,"infer":false})).unwrap();
    h.frames(2);
    h.step(&json!({"start":"sketch.extend"})).unwrap();
    h.step(&json!({"click":{"world":[5,3,0]}})).unwrap();
    let st = h.app.session.world_state();
    let ss = st.sketch(h.app.session.active_sketch.unwrap()).unwrap();
    let end = ss.sketch.point(ss.sketch.resolve_point("l1.end").unwrap()).unwrap();
    assert!(end.dist(Vec2::new(12.0, 3.0)) < 1e-7, "clicked endpoint must extend: {end:?}, {:?}", h.app.status);
    assert_eq!(ss.sketch.point(ss.sketch.resolve_point("l1.start").unwrap()), Some(Vec2::new(0.0, 3.0)));
}

#[test]
fn derived_offset_context_action_releases_geometry_and_undo_restores() {
    let mut h = crate::scenario::Harness::new();
    h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
    h.app.run("sketch.ellipse", json!({"center":[0,0],"major":[10,0],"minor_radius":5})).unwrap();
    let out = h.app.run("sketch.offset", json!({"curves":["e1"],"distance":-1,"chain":false})).unwrap();
    let id = out["curves"][0].as_str().unwrap().to_string();
    h.app.session.selection = vec![solvecraft_engine::Sel::SketchCurve { id: id.clone() }];
    let items = crate::context_menu::items(&h.app, &crate::context_menu::Target::Viewport);
    let item = items.iter().find(|item| item.id == "sketch.offset.break").unwrap();
    assert_eq!(item.params, Some(json!({"curve":id})));
    crate::context_menu::run_item(&mut h.app, item, egui::Pos2::ZERO);
    let sid = h.app.session.active_sketch.unwrap();
    assert!(h.app.session.model.state().sketch(sid).unwrap().sketch.freeform_offsets.is_empty());
    h.app.run("edit.undo", json!({})).unwrap();
    assert_eq!(h.app.session.model.state().sketch(sid).unwrap().sketch.freeform_offsets.len(), 1);
}

#[test]
fn clicked_origin_corner_fillets_and_chamfers_with_origin_visible() {
    for command in ["sketch.fillet", "sketch.chamfer.equal_distance", "sketch.chamfer.distance_angle", "sketch.chamfer.two_distance"] {
        let mut h = crate::scenario::Harness::new();
        h.app.ui.show_origin = true;
        h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
        h.app.run("sketch.rectangle.two_point", json!({"p0":[0,0],"p1":[20,30]})).unwrap();
        h.frames(2);
        let sid = h.app.session.active_sketch.unwrap();
        let proj = crate::viewport::projection(&h.app, h.app.viewport.rect.unwrap());
        let cursor = proj.to_screen(Vec3::ZERO).unwrap();
        // The ordinary snap still finds the fixed origin; the corner tool must instead
        // reach the rectangle's overlapping endpoint.
        assert_eq!(sketch_point_at(&h.app, &proj, cursor).unwrap().1.as_deref(), Some("origin"));
        h.step(&json!({"start":command})).unwrap();
        h.step(&json!({"move":{"world":[0,0,0]}})).unwrap();
        let point = h.app.tool.as_ref().unwrap().hover.as_ref().unwrap().1.as_deref().unwrap();
        assert_ne!(point, "origin", "the actual shared endpoint must be highlighted");
        h.step(&json!({"click":{"world":[0,0,0]}})).unwrap();
        let st = h.app.session.world_state();
        let ss = st.sketch(sid).unwrap();
        assert_eq!(ss.sketch.point(0), Some(Vec2::ZERO));
        assert!(ss.sketch.points[0].fixed);
        assert_eq!(ss.sketch.curves.len(), 5, "{command} must actually modify the geometry; status: {:?}", h.app.status);
        if command == "sketch.fillet" {
            assert!(ss.sketch.curves.iter().any(|c| matches!(c.kind, CurveKind::Arc { .. })));
        } else {
            assert!(ss.sketch.curves.iter().all(|c| matches!(c.kind, CurveKind::Line { .. })));
        }
        let expected = if command == "sketch.fillet" { 600.0 - 25.0 * (1.0 - std::f64::consts::PI / 4.0) } else { 587.5 };
        assert_eq!(ss.profiles.len(), 1);
        assert!((ss.profiles[0].area - expected).abs() < 1e-6, "{command}: {}", ss.profiles[0].area);
        h.app.run("edit.undo", json!({})).unwrap();
        let st = h.app.session.world_state();
        let ss = st.sketch(sid).unwrap();
        assert_eq!(ss.sketch.curves.len(), 4);
        assert!((ss.profiles[0].area - 600.0).abs() < 1e-6);
    }
}

#[test]
fn ordinary_drawing_keeps_origin_snap_and_real_origin_corners_work() {
    let mut h = crate::scenario::Harness::new();
    h.app.ui.show_origin = true;
    h.app.run("sketch.create", json!({"plane":"XY"})).unwrap();
    h.step(&json!({"start":"sketch.line"})).unwrap();
    h.step(&json!({"move":{"world":[0,0,0]}})).unwrap();
    assert_eq!(h.app.tool.as_ref().unwrap().hover.as_ref().unwrap().1.as_deref(), Some("origin"));
    h.step(&json!({"click":{"world":[0,0,0]}})).unwrap();
    assert_eq!(h.app.tool.as_ref().unwrap().pts[0].1.as_deref(), Some("origin"));
    h.app.tool = None;
    h.app.run("sketch.line", json!({"points":["origin",[20,0],[20,20],[0,20],"origin"]})).unwrap();
    h.step(&json!({"start":"sketch.fillet"})).unwrap();
    h.step(&json!({"click":{"world":[0,0,0]}})).unwrap();
    let st = h.app.session.world_state();
    let ss = st.sketch(h.app.session.active_sketch.unwrap()).unwrap();
    assert_eq!(ss.sketch.point(0), Some(Vec2::ZERO));
    assert!(ss.sketch.curves.iter().any(|c| matches!(c.kind, CurveKind::Arc { .. })));
    assert_eq!(ss.profiles.len(), 1);
}
