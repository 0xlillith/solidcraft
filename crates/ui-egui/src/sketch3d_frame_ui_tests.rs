//! Real 3D drawing and triad input in a rotated second component occurrence.

use serde_json::json;
use solvecraft_engine::geom::Vec3;

#[test]
fn second_occurrence_drawn_wire_fit_picks_and_world_triad_drag_stay_aligned() {
    let mut h = crate::scenario::Harness::new();
    let component = h.app.run("component.create", json!({"name":"Part"})).unwrap()["component"].as_u64().unwrap();
    let first = h.app.session.doc.occurrence_of(component).unwrap().id;
    let second = h.app.run("occurrence.copy", json!({"component":component})).unwrap()["occurrence"].as_u64().unwrap();
    h.app.run("occurrence.move", json!({"occurrence":second,"translate":[20,30,40],"axis":[0,0,1],"angle":"90 deg","capture":true})).unwrap();
    h.app.run("component.activate", json!({"component":component,"occurrence":second})).unwrap();
    h.app.run("solid.box", json!({"corner":[17,32,45],"length":2,"width":2,"height":10,"body_name":"SnapVertices"})).unwrap();
    h.app.run("browser.visibility", json!({"paths":[[first]],"visible":false})).unwrap();
    let sketch = h.app.run("sketch.create", json!({"plane":"XY","name":"Wire"})).unwrap()["sketch"].as_u64().unwrap();
    h.app.ui.show_origin = false;
    h.settle();
    h.step(&json!({"widget":"3D Sketch"})).unwrap();
    assert!(
        crate::sketch3d::active(&h.app),
        "palette click must enable 3D: raw={:?}, world={:?}, status={:?}, active={:?}",
        h.app.session.model.state().sketch(sketch).unwrap().sketch.view,
        h.app.session.world_state().sketch(sketch).unwrap().sketch.view,
        h.app.status,
        h.app.session.active_sketch
    );
    h.call("ui.view", json!({"view":"home"}));
    h.call("ui.view", json!({"view":"fit"}));
    h.app.start("sketch.line");
    h.frames(2);
    for world in [Vec3::new(17.0, 32.0, 45.0), Vec3::new(17.0, 32.0, 55.0)] {
        let [x, y] = h.at(&json!({"world":[world.x,world.y,world.z]})).unwrap();
        let proj = crate::viewport::projection(&h.app, h.app.viewport.rect.unwrap());
        let picked = crate::sketch3d::point_at(&h.app, &proj, egui::pos2(x as f32, y as f32));
        assert!(picked.is_some_and(|(p, _)| p.dist(world) < 1e-5), "world={world:?}, picked={picked:?}, status={:?}", h.app.status);
    }
    h.step(&json!({"click":{"world":[17,32,45]}})).unwrap();
    h.step(&json!({"click":{"world":[17,32,55]}})).unwrap();
    h.step(&json!({"key":"Escape"})).unwrap();
    h.step(&json!({"key":"Escape"})).unwrap();
    assert!(h.app.tool.is_none());
    let raw = h.app.session.model.state();
    let wire = raw.sketch(sketch).unwrap().sketch.wires.first().expect("actual off-plane clicks create a wire");
    assert!(wire.fit[0].dist(Vec3::new(2.0, 3.0, 5.0)) < 1e-6, "{:?}", wire.fit);
    assert!(wire.fit[1].dist(Vec3::new(2.0, 3.0, 15.0)) < 1e-6, "{:?}", wire.fit);
    let world = h.app.session.world_state();
    let shown = &world.sketch(sketch).unwrap().sketch.wires[0];
    assert!(shown.fit[0].dist(Vec3::new(17.0, 32.0, 45.0)) < 1e-6, "{:?}", shown.fit);
    assert!(shown.fit[1].dist(Vec3::new(17.0, 32.0, 55.0)) < 1e-6, "{:?}", shown.fit);
    h.step(&json!({"click":{"world":[17,32,45]}})).unwrap();
    let [x0, y0] = h.at(&json!({"handle":"triad_x"})).unwrap();
    let before = h.at(&json!({"world":[17,32,45]})).unwrap();
    let after = h.at(&json!({"world":[21,32,45]})).unwrap();
    let history = h.app.session.undo.len();
    h.call("ui.drag", json!({"x0":x0,"y0":y0,"x1":x0+after[0]-before[0],"y1":y0+after[1]-before[1],"steps":8}));
    let moved = h.app.session.model.state();
    let fit = &moved.sketch(sketch).unwrap().sketch.wires[0].fit;
    assert!(fit[0].dist(Vec3::new(2.0, -1.0, 5.0)) < 1e-5, "{fit:?}");
    assert!(fit[1].dist(Vec3::new(2.0, 3.0, 15.0)) < 1e-6);
    let placed = h.app.session.world_state();
    assert!(placed.sketch(sketch).unwrap().sketch.wires[0].fit[0].dist(Vec3::new(21.0, 32.0, 45.0)) < 1e-5);
    assert_eq!(h.app.session.undo.len(), history + 1);
    h.app.run("edit.undo", json!({})).unwrap();
    let restored = h.app.session.model.state();
    assert!(restored.sketch(sketch).unwrap().sketch.wires[0].fit[0].dist(Vec3::new(2.0, 3.0, 5.0)) < 1e-6);
}
