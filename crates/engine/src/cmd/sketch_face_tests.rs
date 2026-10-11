use crate::Session;
use serde_json::{Value, json};
use solvecraft_geom::Vec3;

fn run(s: &mut Session, command: &str, params: Value) -> Value {
    s.execute(command, &params).unwrap_or_else(|error| panic!("{command} {params}: {error}"))
}
fn top_pick(s: &Session, body: &str) -> Value {
    let world = s.world_state();
    let body = world.body(body).unwrap();
    let face = body
        .body
        .faces(0.01)
        .unwrap()
        .into_iter()
        .filter(|face| face.plane_normal.is_some_and(|normal| normal.dot(Vec3::Z) > 0.999))
        .max_by(|a, b| a.centroid.z.total_cmp(&b.centroid.z))
        .unwrap();
    json!({"body":body.name,"index":face.index,"face":[face.centroid.x,face.centroid.y,face.centroid.z]})
}
fn draw(s: &mut Session) {
    run(s, "sketch.rectangle.two_point", json!({"p0":[1,1],"p1":[3,3]}));
    run(s, "sketch.dimension", json!({"entities":["l1"],"value":2}));
    run(s, "sketch.finish", json!({}));
}
#[test]
fn exact_selected_base_follows_height_and_never_retargets_control() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Base","name":"BaseFeature"}));
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":30,"body_name":"Control","name":"ControlFeature"}));
    let pick = top_pick(&s, "Base");
    let sketch = run(&mut s, "sketch.create", json!({"name":"Attached","plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "solid.extrude", json!({"sketch":sketch,"distance":2,"operation":"new","body_name":"Badge"}));
    let height = s.doc.find_feature("BaseFeature").unwrap().param_names[2].clone();
    run(&mut s, "parameters.change", json!({"name":height,"expression":"20 mm"}));
    assert!((s.model.state().sketch(sketch).unwrap().plane.origin.z - 20.).abs() < 1e-6);
    assert!((s.world_state().body("Badge").unwrap().mesh().bounds().max.z - 22.).abs() < 1e-6);
    run(&mut s, "timeline.delete", json!({"features":["BaseFeature"]}));
    assert!(s.model.result(sketch).unwrap().error.is_some());
    assert!(s.model.state().body("Badge").is_none());
    assert!((s.world_state().body("Control").unwrap().mesh().bounds().max.z - 30.).abs() < 1e-6);
    let before = s.doc.to_json();
    let reopened = Session::new(solvecraft_doc::Document::from_json(&before).unwrap());
    assert!(reopened.model.result(sketch).unwrap().error.is_some());
}
#[test]
fn explicit_rebind_preserves_geometry_dimensions_context_and_one_undo() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Base","name":"BaseFeature"}));
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":30,"body_name":"Control"}));
    let pick = top_pick(&s, "Base");
    let id = run(&mut s, "sketch.create", json!({"plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    draw(&mut s);
    run(&mut s, "timeline.delete", json!({"features":["BaseFeature"]}));
    let geometry = s.doc.sketch(id).unwrap().clone();
    let before = s.doc.to_json();
    let context = (s.active_component, s.active_occurrence_path.clone(), s.active_sketch);
    let pick = top_pick(&s, "Control");
    let commands = vec![("sketch.redefine".into(), json!({"sketch":id,"plane":pick}))];
    let preview = s.preview_visible_world(&commands).unwrap();
    assert_eq!(s.doc.to_json(), before);
    assert!(preview.after.sketch(id).is_some());
    run(&mut s, "sketch.redefine", commands[0].1.clone());
    assert_eq!(s.doc.sketch(id).unwrap(), &geometry);
    assert_eq!((s.active_component, s.active_occurrence_path.clone(), s.active_sketch), context);
    assert!((s.model.state().sketch(id).unwrap().plane.origin.z - 30.).abs() < 1e-6);
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc.to_json(), before);
    assert!(s.model.result(id).unwrap().error.is_some());
    run(&mut s, "edit.redo", json!({}));
    let reopened = Session::new(solvecraft_doc::Document::from_json(&s.doc.to_json()).unwrap());
    assert!((reopened.model.state().sketch(id).unwrap().plane.origin.z - 30.).abs() < 1e-6);
}
#[test]
fn unsupported_face_has_explicit_fixed_alternative_and_no_live_projection() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Base"}));
    run(&mut s, "solid.box", json!({"length":2,"width":2,"height":2,"corner":[4,4,10],"operation":"join"}));
    let mut pick = top_pick(&s, "Base");
    let before = s.doc.to_json();
    let error = s.execute("sketch.create", &json!({"plane":pick})).unwrap_err();
    assert!(error.to_string().contains("fixed"), "{error}");
    assert_eq!(s.doc.to_json(), before);
    pick["fixed"] = json!(true);
    let id = run(&mut s, "sketch.create", json!({"plane":pick}))["sketch"].as_u64().unwrap();
    assert!(matches!(s.doc.feature(id).unwrap().kind, solvecraft_doc::FeatureKind::Sketch { plane: solvecraft_doc::PlaneRef::Custom { .. }, .. }));
    assert!(s.doc.sketch(id).unwrap().links.is_empty());
}
#[test]
fn malformed_exact_face_and_path_leave_every_session_field_unchanged() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Base"}));
    let pick = top_pick(&s, "Base");
    for invalid in [
        json!({"body":"Missing","index":pick["index"],"face":pick["face"]}),
        json!({"body":"Base","index":-1,"face":pick["face"]}),
        json!({"body":"Base","index":pick["index"],"face":pick["face"],"path":[999]}),
        json!({"body":"Base","face":pick["face"]}),
    ] {
        let before = s.doc.to_json();
        let pending = s.pending_moves.clone();
        let context = (s.active_component, s.active_occurrence_path.clone(), s.active_sketch);
        assert!(s.execute("sketch.create", &json!({"plane":invalid})).is_err());
        assert_eq!(s.doc.to_json(), before);
        assert_eq!(s.pending_moves, pending);
        assert_eq!((s.active_component, s.active_occurrence_path.clone(), s.active_sketch), context);
    }
}

#[test]
fn default_certified_borders_follow_width_and_height_with_stable_entities() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":10,"width":8,"height":10,"body_name":"Base","name":"BaseFeature"}));
    let pick = top_pick(&s, "Base");
    let id = run(&mut s, "sketch.create", json!({"plane":pick}))["sketch"].as_u64().unwrap();
    assert!(s.doc.sketch(id).unwrap().links.iter().any(|link| link.auto_face));
    let old_ids: Vec<_> = s.doc.sketch(id).unwrap().curves.iter().map(|curve| curve.id.clone()).collect();
    run(&mut s, "sketch.finish", json!({}));
    let dimensions = s.doc.find_feature("BaseFeature").unwrap().param_names.clone();
    run(&mut s, "parameters.change", json!({"name":dimensions[0],"expression":"20 mm"}));
    run(&mut s, "parameters.change", json!({"name":dimensions[2],"expression":"15 mm"}));
    let resolved = s.model.state();
    let sketch = resolved.sketch(id).unwrap();
    assert!((sketch.plane.origin.z - 15.).abs() < 1e-6);
    assert!(sketch.sketch.links.iter().all(|link| !link.lost));
    assert_eq!(sketch.sketch.curves.iter().map(|curve| curve.id.clone()).collect::<Vec<_>>(), old_ids);
    let max_x = sketch.sketch.points.iter().map(|point| point.pos.x).fold(f64::NEG_INFINITY, f64::max);
    let min_x = sketch.sketch.points.iter().map(|point| point.pos.x).fold(f64::INFINITY, f64::min);
    let max_y = sketch.sketch.points.iter().map(|point| point.pos.y).fold(f64::NEG_INFINITY, f64::max);
    let min_y = sketch.sketch.points.iter().map(|point| point.pos.y).fold(f64::INFINITY, f64::min);
    assert!(((max_x - min_x) * (max_y - min_y) - 160.).abs() < 1e-5);
    let reopened = Session::new(solvecraft_doc::Document::from_json(&s.doc.to_json()).unwrap());
    assert!(reopened.model.state().sketch(id).unwrap().sketch.links.iter().all(|link| !link.lost));
}

#[test]
fn nested_offset_retains_exact_attachment_during_height_change() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Base","name":"BaseFeature"}));
    let pick = top_pick(&s, "Base");
    let id = run(&mut s, "sketch.create", json!({"plane":pick,"offset":"3 mm","project_edges":false}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.finish", json!({}));
    let height = s.doc.find_feature("BaseFeature").unwrap().param_names[2].clone();
    run(&mut s, "parameters.change", json!({"name":height,"expression":"20 mm"}));
    assert!((s.model.state().sketch(id).unwrap().plane.origin.z - 23.).abs() < 1e-6);
}

#[test]
fn exact_nested_source_and_cross_owner_pending_paths_capture_only_involved_positions() {
    let mut s = Session::default();
    let parent = run(&mut s, "component.create", json!({"name":"Parent","parent":0}));
    let second = run(&mut s, "occurrence.copy", json!({"occurrence":parent["occurrence"],"translate":[40,0,0]}))["occurrence"].as_u64().unwrap();
    run(&mut s, "component.activate", json!({"component":parent["component"],"occurrence":second,"path":[second]}));
    let child = run(&mut s, "component.create", json!({"name":"Child"}));
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Base"}));
    let child_occ = child["occurrence"].as_u64().unwrap();
    run(&mut s, "occurrence.move", json!({"occurrence":second,"translate":[10,0,0],"capture":false}));
    run(&mut s, "occurrence.move", json!({"occurrence":child_occ,"translate":[0,20,0],"capture":false}));
    let unrelated = run(&mut s, "component.create", json!({"name":"Unrelated","parent":0}));
    run(&mut s, "occurrence.move", json!({"occurrence":unrelated["occurrence"],"translate":[300,0,0],"capture":false}));
    let author = run(&mut s, "component.create", json!({"name":"Author","parent":0}));
    run(&mut s, "occurrence.move", json!({"occurrence":author["occurrence"],"translate":[100,0,0],"angle":"90 deg","capture":false}));
    let world = s.world_state();
    let key = s
        .placed_document()
        .body_frames_with_source(&s.model.state())
        .into_iter()
        .find(|(_, source, _, path, _)| source == "Base" && *path == vec![second, child_occ])
        .unwrap()
        .0;
    assert!(world.body(&key).is_some());
    let mut pick = top_pick(&s, &key);
    pick["path"] = json!([second, child_occ]);
    let before = s.doc.to_json();
    let pending = s.pending_moves.clone();
    let id = run(&mut s, "sketch.create", json!({"plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    let solvecraft_doc::FeatureKind::Sketch { plane, .. } = &s.doc.feature(id).unwrap().kind else { panic!("not sketch") };
    let anchor = super::anchor(plane).unwrap();
    assert_eq!(anchor.path, vec![second, child_occ]);
    assert_eq!(anchor.author_path, vec![author["occurrence"].as_u64().unwrap()]);
    assert_eq!(s.pending_moves.len(), 1);
    assert!(s.pending_moves.contains_key(&unrelated["occurrence"].as_u64().unwrap()));
    let saved = s.doc.to_json();
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc.to_json(), before);
    assert_eq!(s.pending_moves, pending);
    run(&mut s, "edit.redo", json!({}));
    let mut reopened = Session::new(solvecraft_doc::Document::from_json(&saved).unwrap());
    assert!(reopened.model.result(id).unwrap().error.is_none());
    run(&mut reopened, "component.activate", json!({"component":0}));
    let original = reopened.doc.to_json();
    let undo = reopened.undo.len();
    let dirty = reopened.is_dirty();
    run(&mut reopened, "sketch.edit", json!({"sketch":id}));
    assert_eq!(reopened.doc.to_json(), original);
    assert_eq!(reopened.undo.len(), undo);
    assert_eq!(reopened.is_dirty(), dirty);
    assert_eq!(reopened.active_component, author["component"].as_u64().unwrap());
    assert_eq!(reopened.active_occurrence_path, Some(vec![author["occurrence"].as_u64().unwrap()]));
}

#[test]
fn fixed_rebind_freezes_current_resized_borders_and_only_detaches_auto_links() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":10,"width":8,"height":10,"body_name":"Base","name":"BaseFeature"}));
    let pick = top_pick(&s, "Base");
    let id = run(&mut s, "sketch.create", json!({"plane":pick}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.project", json!({"refs":[{"origin":true}]}));
    run(&mut s, "sketch.finish", json!({}));
    let length = s.doc.find_feature("BaseFeature").unwrap().param_names[0].clone();
    run(&mut s, "parameters.change", json!({"name":length,"expression":"20 mm"}));
    let visible = s.model.state().sketch(id).unwrap().sketch.clone();
    let auto: Vec<_> = visible.links.iter().filter(|link| link.auto_face).map(|link| link.id.clone()).collect();
    let manual: Vec<_> = visible.links.iter().filter(|link| !link.auto_face).map(|link| link.id.clone()).collect();
    let before = s.doc.to_json();
    let mut pick = top_pick(&s, "Base");
    pick["fixed"] = json!(true);
    run(&mut s, "sketch.redefine", json!({"sketch":id,"plane":pick}));
    let fixed = s.doc.sketch(id).unwrap();
    assert!(auto.iter().all(|id| fixed.link(id).is_none()));
    assert!(manual.iter().all(|id| fixed.link(id).is_some()));
    assert_eq!(
        fixed.curves.iter().map(|curve| curve.id.clone()).collect::<Vec<_>>(),
        visible.curves.iter().map(|curve| curve.id.clone()).collect::<Vec<_>>()
    );
    assert_eq!(fixed.points.iter().map(|point| point.pos).collect::<Vec<_>>(), visible.points.iter().map(|point| point.pos).collect::<Vec<_>>());
    let reopened = Session::new(solvecraft_doc::Document::from_json(&s.doc.to_json()).unwrap());
    assert!(reopened.model.result(id).unwrap().error.is_none());
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc.to_json(), before);
}

#[test]
fn strict_anchor_timeline_copy_remaps_producer_and_refuses_external_translation_atomically() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Base","name":"BaseFeature"}));
    let pick = top_pick(&s, "Base");
    let id = run(&mut s, "sketch.create", json!({"name":"Attached","plane":pick}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "timeline.copy", json!({"features":["Attached"]}));
    let before = s.doc.to_json();
    let pending = s.pending_moves.clone();
    let error = s.execute("timeline.paste", &json!({"translate":[30,0,0]})).unwrap_err();
    assert!(error.to_string().contains("external exact face"), "{error}");
    assert_eq!(s.doc.to_json(), before);
    assert_eq!(s.pending_moves, pending);
    run(&mut s, "timeline.copy", json!({"features":["BaseFeature","Attached"]}));
    let pasted = run(&mut s, "timeline.paste", json!({"translate":[30,0,0]}));
    assert!(pasted["errors"].as_array().unwrap().is_empty(), "{pasted}");
    let new_id = pasted["pasted"][1]["id"].as_u64().unwrap();
    let solvecraft_doc::FeatureKind::Sketch { plane, .. } = &s.doc.feature(new_id).unwrap().kind else { panic!("not sketch") };
    let anchor = super::anchor(plane).unwrap();
    assert_ne!(anchor.target.origin_feature, s.doc.find_feature("BaseFeature").unwrap().id);
    assert_ne!(anchor.target.body, "Base");
    assert!(s.model.state().sketch(new_id).unwrap().sketch.links.iter().all(|link| !link.lost));
    assert!(s.model.state().sketch(id).unwrap().sketch.links.iter().all(|link| !link.lost));
    let reopened = Session::new(solvecraft_doc::Document::from_json(&s.doc.to_json()).unwrap());
    assert!(reopened.model.result(new_id).unwrap().error.is_none());
}

#[test]
fn strict_anchor_deep_clone_is_independent_and_rename_preserves_certified_projection() {
    let mut s = Session::default();
    let part = run(&mut s, "component.create", json!({"name":"Part","parent":0}));
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Base","name":"BaseFeature"}));
    let pick = top_pick(&s, "Base");
    let id = run(&mut s, "sketch.create", json!({"name":"Attached","plane":pick}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.finish", json!({}));
    let height = s.doc.find_feature("BaseFeature").unwrap().param_names[2].clone();
    let pasted = run(&mut s, "component.paste_new", json!({"component":part["component"],"translate":[30,0,0]}));
    let owner = pasted["component"].as_u64().unwrap();
    let new_feature = s
        .doc
        .features
        .iter()
        .find(|feature| feature.component == owner && matches!(feature.kind, solvecraft_doc::FeatureKind::Sketch { .. }))
        .unwrap();
    let new_id = new_feature.id;
    let solvecraft_doc::FeatureKind::Sketch { plane, .. } = &new_feature.kind else { panic!("not sketch") };
    let anchor = super::anchor(plane).unwrap();
    let body = anchor.target.body.clone();
    assert_eq!(anchor.component, owner);
    run(&mut s, "parameters.change", json!({"name":height,"expression":"20 mm"}));
    assert!((s.model.state().sketch(id).unwrap().plane.origin.z - 20.).abs() < 1e-6);
    assert!((s.model.state().sketch(new_id).unwrap().plane.origin.z - 10.).abs() < 1e-6);
    run(&mut s, "body.rename", json!({"body":body,"name":"Independent"}));
    assert!(s.model.result(new_id).unwrap().error.is_none());
    assert!(s.model.state().sketch(new_id).unwrap().sketch.links.iter().all(|link| !link.lost));
    let reopened = Session::new(solvecraft_doc::Document::from_json(&s.doc.to_json()).unwrap());
    assert!(reopened.model.result(new_id).unwrap().error.is_none());
}

#[test]
fn legacy_or_fixed_rebind_needs_explicit_author_path_for_repeated_owner() {
    let mut s = Session::default();
    let part = run(&mut s, "component.create", json!({"name":"Part","parent":0}));
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Base"}));
    let mut pick = top_pick(&s, "Base");
    pick["fixed"] = json!(true);
    let id = run(&mut s, "sketch.create", json!({"plane":pick}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.finish", json!({}));
    let second = run(&mut s, "occurrence.copy", json!({"occurrence":part["occurrence"],"translate":[20,0,0]}))["occurrence"].as_u64().unwrap();
    run(&mut s, "component.activate", json!({"component":0}));
    let pick = top_pick(&s, "Base");
    let before = s.doc.to_json();
    let error = s.execute("sketch.redefine", &json!({"sketch":id,"plane":pick})).unwrap_err();
    assert!(error.to_string().contains("ambiguous"), "{error}");
    assert_eq!(s.doc.to_json(), before);
    run(&mut s, "sketch.redefine", json!({"sketch":id,"plane":pick,"author_path":[second]}));
    let solvecraft_doc::FeatureKind::Sketch { plane, .. } = &s.doc.feature(id).unwrap().kind else { panic!("not sketch") };
    assert_eq!(super::anchor(plane).unwrap().author_path, vec![second]);
    assert_eq!(s.active_component, 0);
}

#[test]
fn replacement_producer_with_same_body_name_cannot_heal_lost_anchor() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Base","name":"Original"}));
    let pick = top_pick(&s, "Base");
    let id = run(&mut s, "sketch.create", json!({"plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "timeline.suppress", json!({"feature":"Original","suppressed":true}));
    run(&mut s, "timeline.rollback", json!({"position":1}));
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":30,"body_name":"Base","name":"Replacement"}));
    run(&mut s, "timeline.rollback", json!({}));
    assert!(s.model.state().body("Base").is_some());
    assert!(s.model.result(id).unwrap().error.is_some());
    let before = s.doc.to_json();
    let pick = top_pick(&s, "Base");
    run(&mut s, "sketch.redefine", json!({"sketch":id,"plane":pick}));
    assert!((s.model.state().sketch(id).unwrap().plane.origin.z - 30.).abs() < 1e-6);
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc.to_json(), before);
    assert!(s.model.result(id).unwrap().error.is_some());
}

#[test]
fn freezing_rebuilt_arc_borders_keeps_authored_dimension_entity_indices() {
    let mut s = Session::default();
    run(&mut s, "parameters.add", json!({"name":"turn","expression":"270 deg","unit":"deg"}));
    let profile = run(&mut s, "sketch.create", json!({"plane":"XY"}))["sketch"].as_u64().unwrap();
    let arc = run(
        &mut s,
        "sketch.draw.dimensioned",
        json!({"command":"sketch.arc.center_point","params":{"center":[0,0],"start":[10,0],"sweep":270},"dimensions":{"radius":10,"sweep":"turn"}}),
    )["curves"][0]
        .as_str()
        .unwrap()
        .to_owned();
    run(&mut s, "sketch.line", json!({"points":[format!("{arc}.end"),format!("{arc}.start")],"infer":false}));
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "solid.extrude", json!({"sketch":profile,"distance":10,"operation":"new","body_name":"Sector"}));
    let pick = top_pick(&s, "Sector");
    let id = run(&mut s, "sketch.create", json!({"plane":pick}))["sketch"].as_u64().unwrap();
    let circle = run(&mut s, "sketch.circle.center", json!({"center":[20,20],"radius":2}))["curves"][0].as_str().unwrap().to_owned();
    run(&mut s, "sketch.dimension", json!({"entities":[circle],"type":"radius","value":"2 mm"}));
    run(&mut s, "sketch.finish", json!({}));
    let before_count = s.model.state().sketch(id).unwrap().sketch.curves.len();
    run(&mut s, "parameters.change", json!({"name":"turn","expression":"300 deg"}));
    assert!(s.model.result(id).unwrap().error.is_none());
    let visible = s.model.state().sketch(id).unwrap().sketch.clone();
    assert_ne!(visible.curves.len(), before_count, "fixture must rebuild projected topology");
    let mut pick = top_pick(&s, "Sector");
    pick["fixed"] = json!(true);
    let before = s.doc.to_json();
    run(&mut s, "sketch.redefine", json!({"sketch":id,"plane":pick}));
    let frozen = s.doc.sketch(id).unwrap();
    let circle_index = frozen.curve_index(&circle).unwrap();
    let dimension = frozen
        .constraints
        .iter()
        .find(|constraint| matches!(constraint.kind, solvecraft_sketch::ConstraintKind::Radius { c, .. } if c == circle_index))
        .unwrap();
    assert!((solvecraft_sketch::measure_dimension(frozen, &dimension.kind).unwrap() - 2.).abs() < 1e-6);
    assert_eq!(frozen.curves.iter().map(|curve| &curve.id).collect::<Vec<_>>(), visible.curves.iter().map(|curve| &curve.id).collect::<Vec<_>>());
    assert!(s.model.result(id).unwrap().error.is_none());
    let reopened = Session::new(solvecraft_doc::Document::from_json(&s.doc.to_json()).unwrap());
    assert!(reopened.model.result(id).unwrap().error.is_none());
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc.to_json(), before);
}

#[test]
fn angled_attachment_tracks_source_plane_and_reopens() {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":10,"width":10,"height":10,"body_name":"Base","name":"BaseFeature"}));
    let pick = top_pick(&s, "Base");
    let id = run(&mut s, "sketch.create", json!({"plane":pick,"project_edges":false}))["sketch"].as_u64().unwrap();
    run(&mut s, "sketch.finish", json!({}));
    let solvecraft_doc::FeatureKind::Sketch { plane, .. } = &mut s.doc_mut().feature_mut(id).unwrap().kind else { panic!("not sketch") };
    *plane = solvecraft_doc::PlaneRef::AtAngle { base: Box::new(plane.clone()), axis_origin: Vec3::ZERO, axis_dir: Vec3::X, angle: "90 deg".into() };
    s.refresh();
    let height = s.doc.find_feature("BaseFeature").unwrap().param_names[2].clone();
    run(&mut s, "parameters.change", json!({"name":height,"expression":"20 mm"}));
    let placed = s.model.state().sketch(id).unwrap().plane;
    assert!((placed.origin.y + 20.).abs() < 1e-6);
    assert!(placed.origin.z.abs() < 1e-6);
    let reopened = Session::new(solvecraft_doc::Document::from_json(&s.doc.to_json()).unwrap());
    assert_eq!(reopened.model.state().sketch(id).unwrap().plane, placed);
}
