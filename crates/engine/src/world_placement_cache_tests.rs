//! Placement-cache changes must retain physical picking, pending positions and named joints.
use crate::{Session, geom::Vec3};
use serde_json::{Value, json};
use std::sync::Arc;

fn run(session: &mut Session, command: &str, params: Value) -> Value {
    session.execute(command, &params).unwrap_or_else(|error| panic!("{command}: {error}"))
}

#[test]
fn world_placement_cache_preserves_pending_picks_materials_scratch_and_named_joint_frames() {
    let mut session = Session::default();
    let created = run(&mut session, "component.create", json!({"name":"Part"}));
    let component = created["component"].as_u64().unwrap();
    let first = created["occurrence"].as_u64().unwrap();
    run(&mut session, "solid.box", json!({"length":2,"width":3,"height":4,"body_name":"Block","operation":"new"}));
    run(&mut session, "material.assign", json!({"bodies":["Block"],"material":"Brass"}));
    let second = run(&mut session, "occurrence.copy", json!({"occurrence":first}))["occurrence"].as_u64().unwrap();
    run(&mut session, "occurrence.move", json!({"occurrence":second,"translate":[10,20,0],"axis":[0,0,1],"angle":"90 deg","capture":false}));
    let pending = session.pending_moves.clone();
    let path = vec![second];
    let placement = session.occurrence_placement(&path).unwrap();
    let name = placement.bodies[0].world.clone();
    let world = session.world_state();
    assert!(Arc::ptr_eq(&world, &session.world_state()));
    let source_state = session.model.state();
    let source = source_state.body("Block").unwrap();
    let placed = world.body(&name).unwrap();
    assert_eq!(placed.source_name.as_deref(), Some("Block"));
    assert_eq!(placed.component, component);
    let bounds = placed.mesh().bounds();
    assert!(bounds.min.dist(Vec3::new(7., 20., 0.)) < 1e-9);
    assert!(bounds.max.dist(Vec3::new(10., 22., 4.)) < 1e-9);
    assert_eq!(session.doc.model_body_look(placed), session.doc.model_body_look(source));
    let source_names = crate::doc::naming::face_names(source);
    assert_eq!(source_names, crate::doc::naming::face_names(placed));
    let pick = Vec3::new(8.5, 21., 4.);
    let top = placed.body.faces(0.001).unwrap().into_iter().find(|face| face.plane_normal.is_some_and(|n| n.z > 0.99)).unwrap().index;
    run(&mut session, "select.set", json!({"items":[{"type":"face","body":name,"index":top,"point":[pick.x,pick.y,pick.z]}]}));
    assert!(matches!(&session.selection[0], crate::Sel::Face { body, index, .. } if body == &name && *index == top));
    let scratch = session.scratch();
    assert_eq!(scratch.pending_moves, pending);
    let scratch_world = scratch.world_state();
    let scratch_body = scratch_world.body(&name).unwrap();
    assert!(scratch_body.mesh().bounds().min.dist(bounds.min) < 1e-9);
    assert_eq!(crate::doc::naming::face_names(scratch_body), source_names);
    run(&mut session, "component.capture_position", json!({}));
    run(&mut session, "joint.origin", json!({"name":"Top joint","occurrence":second,"face":[pick.x,pick.y,pick.z]}));
    let joint = session.doc.assembly.origins[0].origin.clone();
    let local_frame = crate::doc::joints::resolve_origin(&session.doc, &session.model.state(), &joint).unwrap();
    let before_joint = crate::doc::mat_mul(&crate::doc::joints::occurrence_world(&session.doc, second), &local_frame);
    assert!(crate::doc::apply_point(&before_joint, Vec3::ZERO).dist(pick) < 1e-9);
    run(&mut session, "occurrence.move", json!({"occurrence":second,"translate":[5,0,0],"capture":true}));
    let moved = session.world_state();
    assert!(!Arc::ptr_eq(&world, &moved), "a changed transform must invalidate placed meshes");
    assert!(moved.body(&name).unwrap().mesh().bounds().min.dist(bounds.min + Vec3::new(5., 0., 0.)) < 1e-9);
    assert!(world.body(&name).unwrap().mesh().bounds().min.dist(bounds.min) < 1e-9, "prior snapshots must remain immutable");
    assert_eq!(crate::doc::naming::face_names(moved.body(&name).unwrap()), source_names);
    let local_frame = crate::doc::joints::resolve_origin(&session.doc, &session.model.state(), &joint).unwrap();
    let after_joint = crate::doc::mat_mul(&crate::doc::joints::occurrence_world(&session.doc, second), &local_frame);
    assert!(crate::doc::apply_point(&after_joint, Vec3::ZERO).dist(pick + Vec3::new(5., 0., 0.)) < 1e-9);
    let reopened = Session::new(crate::doc::Document::from_json(&session.doc.to_json()).unwrap());
    assert!(reopened.world_state().body(&name).unwrap().mesh().bounds().min.dist(bounds.min + Vec3::new(5., 0., 0.)) < 1e-9);
    assert_eq!(reopened.model.results.iter().filter(|r| r.error.is_some()).count(), 0);
}
