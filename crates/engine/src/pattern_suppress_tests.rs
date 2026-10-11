//! A pattern of a suppressed feature has nothing to copy (#42): its copies go too, and the
//! pattern says so.

use serde_json::{Value, json};

use crate::Session;

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id} {p}: {e}"),
    }
}

fn volume(s: &Session, body: &str) -> f64 {
    s.model.state().body(body).map(|b| solvecraft_kernel::measure(&b.body).unwrap().volume).unwrap_or(0.0)
}

#[test]
fn suppressing_a_patterned_feature_removes_its_copies() {
    let mut s = Session::default();
    let script: Value = serde_json::from_str(include_str!("../../../examples/acceptance/sensor_enclosure.json")).unwrap();
    s.run_script(&script).unwrap();
    let full = volume(&s, "Case");
    run(&mut s, "timeline.suppress", json!({"feature": "Standoff", "suppressed": true}));
    let without = volume(&s, "Case");
    // All four standoffs go (each 116.8 mm³), not just the seed.
    assert!((full - without - 4.0 * 116.8).abs() < 2.0, "{full} → {without}");
    let pattern = s.doc.find_feature("RectangularPattern1").unwrap().id;
    let r = s.model.result(pattern).unwrap();
    assert!(r.error.is_none() && r.warning.as_deref().is_some_and(|w| w.contains("Standoff")), "{r:?}");
    // Back on: the four standoffs return.
    run(&mut s, "timeline.suppress", json!({"feature": "Standoff", "suppressed": false}));
    assert!((volume(&s, "Case") - full).abs() < 1e-6 * full);
}

#[test]
fn enclosure_datums_follow_wall_height_and_board_depth_with_material_proofs() {
    let mut s = Session::default();
    let script: Value = serde_json::from_str(include_str!("../../../examples/acceptance/sensor_enclosure.json")).unwrap();
    s.run_script(&script).unwrap();
    for (name, expression) in [("wall", "2.5 mm"), ("height", "30 mm"), ("board_d", "50 mm")] {
        run(&mut s, "parameters.change", json!({"name":name,"expression":expression}));
    }
    assert!(s.model.results.iter().all(|result| result.error.is_none()));
    for (name, z) in [("PCB standoff", 2.5), ("Lid boss", 2.5), ("Lid lip", 30.), ("Lid screw", 34.)] {
        let feature =
            s.doc.features.iter().find(|feature| feature.name == name && matches!(feature.kind, solvecraft_doc::FeatureKind::Sketch { .. })).unwrap();
        assert!((s.model.state().sketch(feature.id).unwrap().plane.origin.z - z).abs() < 1e-6);
    }
    let usb =
        s.doc.features.iter().find(|feature| feature.name == "USB" && matches!(feature.kind, solvecraft_doc::FeatureKind::Sketch { .. })).unwrap();
    assert!((s.model.state().sketch(usb.id).unwrap().plane.origin.y + 35.5).abs() < 1e-6);
    let assert_material = |session: &Session| {
        let state = session.world_state();
        let case = state.body("Case").unwrap().mesh();
        let lid = state.body("Lid").unwrap().mesh();
        assert!(case.contains(solvecraft_geom::Vec3::new(-24.5, -21.5, 7.)));
        assert!(!case.contains(solvecraft_geom::Vec3::new(-26.5, -21.5, 7.)));
        assert!(!case.contains(solvecraft_geom::Vec3::new(-10., -35.25, 11.35)));
        assert!(case.contains(solvecraft_geom::Vec3::new(-10., -35.25, 15.)));
        assert!(
            lid.contains(solvecraft_geom::Vec3::new(0., 32.2, 29.)),
            "lid bounds {:?}; lip {:?}; probes {:?}",
            lid.bounds(),
            session
                .model
                .state()
                .sketch(
                    session
                        .doc
                        .features
                        .iter()
                        .find(|feature| feature.name == "Lid lip" && matches!(feature.kind, solvecraft_doc::FeatureKind::Sketch { .. }))
                        .unwrap()
                        .id
                )
                .unwrap()
                .sketch
                .links,
            (20..35).map(|y| (y, lid.contains(solvecraft_geom::Vec3::new(0., f64::from(y) + 0.2, 29.)))).collect::<Vec<_>>()
        );
        assert!(!lid.contains(solvecraft_geom::Vec3::new(0., 32.9, 29.)));
        assert!(!lid.contains(solvecraft_geom::Vec3::new(-33.5, -28.5, 32.)));
        assert!(lid.contains(solvecraft_geom::Vec3::new(-31.5, -28.5, 32.)));
    };
    assert_material(&s);
    let reopened = Session::new(solvecraft_doc::Document::from_json(&s.doc.to_json()).unwrap());
    assert!(reopened.model.results.iter().all(|result| result.error.is_none()));
    assert_material(&reopened);
}

#[test]
fn enclosure_inward_offset_collapse_reports_an_error_instead_of_a_stale_lip() {
    let mut s = Session::default();
    let script: Value = serde_json::from_str(include_str!("../../../examples/acceptance/sensor_enclosure.json")).unwrap();
    s.run_script(&script).unwrap();
    run(&mut s, "parameters.change", json!({"name":"wall","expression":"3 mm"}));
    // R4 outer corners leave R1 inner corners. The requested 1.4 mm inward
    // offset needs a topology transition, which this offset operation refuses.
    let lip = s.doc.features.iter().find(|f| f.name == "Lid lip" && matches!(f.kind, solvecraft_doc::FeatureKind::Sketch { .. })).unwrap();
    let result = s.model.result(lip.id).unwrap();
    assert!(result.error.as_deref().is_some_and(|error| error.contains("offset collapses an arc")), "{result:?}");
    assert!(result.warning.as_deref().is_none_or(|warning| !warning.contains("identity is ambiguous")), "{result:?}");
    let state = s.model.state();
    assert!(state.sketch(lip.id).is_none());
    assert!(!state.body("Lid").unwrap().mesh().contains(solvecraft_geom::Vec3::new(0., 27.2, 24.)));
    let reopened = Session::new(solvecraft_doc::Document::from_json(&s.doc.to_json()).unwrap());
    assert!(reopened.model.result(lip.id).unwrap().error.as_deref().is_some_and(|error| error.contains("offset collapses an arc")));
    assert!(reopened.model.state().sketch(lip.id).is_none());
}
