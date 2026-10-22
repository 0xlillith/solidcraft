use super::*;
use solidcraft_render::job::textures::{BaseColorMode, TextureMap};

#[test]
fn authored_library_search_is_bounded_and_concepts_are_not_runtime_presets() {
    let mut s = Session::default();
    for (kind, count) in [
        ("materials", 600),
        ("lighting", 280),
        ("environments", 280),
        ("surfaces", 48),
        ("selene_appearances", 48),
        ("hdr", 24),
        ("rigs", 24),
        ("scenes", 24),
        ("quality", 10),
    ] {
        let response = s.execute("render.library.list", &json!({"kind":kind,"limit":1})).unwrap();
        assert_eq!(response["total"], count);
        assert_eq!(response["items"].as_array().unwrap().len(), 1);
        assert_eq!(response["next_offset"], 1);
    }
    let response = s.execute("render.library.describe", &json!({"kind":"materials","id":"MAT-001"})).unwrap();
    assert_eq!(response["projection"]["status"], "specification_only");
    assert_eq!(response["resource"]["status"], "concept");
    for params in [
        json!({"limit":129}),
        json!({"limit":0}),
        json!({"offset":4097}),
        json!({"query":"x".repeat(257)}),
        json!({"kind":"not-a-kind"}),
        json!({"extra":true}),
    ] {
        assert!(s.execute("render.library.list", &params).is_err());
    }
    assert_eq!(s.execute("render.library.list", &json!({"kind":"surfaces","query":"SURF-001"})).unwrap()["total"], 1);
}

#[test]
fn built_in_maps_keep_exact_published_bytes_and_explicit_channel_roles() {
    let mut count = 0;
    for surface in catalog("surface_manifest", "test").unwrap() {
        let id = surface["id"].as_str().unwrap();
        for (name, role) in [
            ("base_color", MapRole::BaseColor),
            ("normal_open_gl", MapRole::NormalOpenGl),
            ("roughness", MapRole::Roughness),
            ("metallic", MapRole::Metallic),
        ] {
            let key = format!("{id}/{name}");
            let bytes = texture_bytes(&key, role, 16 * 1024 * 1024, "test").unwrap();
            assert_eq!(HdrId::of(&bytes).to_string(), surface["maps"][name]["sha256"].as_str().unwrap());
            assert_eq!(TextureMap::new(bytes, role).unwrap().dimensions(), [512, 512]);
            count += 1;
        }
    }
    assert_eq!(count, 192);
    for key in ["../SURF-001/base_color", "SURF-001/../base_color", "SURF-001/height", "SURF-999/base_color"] {
        assert!(texture_bytes(key, MapRole::BaseColor, 16 * 1024 * 1024, "test").is_err());
    }
    assert!(texture_bytes("SURF-001/normal_open_gl", MapRole::BaseColor, 16 * 1024 * 1024, "test").is_err());
    assert!(texture_bytes("SURF-001/base_color", MapRole::BaseColor, 1, "test").is_err());
}

#[test]
fn builtin_hdr_capture_is_atomic_undoable_deduplicated_and_path_independent() {
    let mut s = Session::default();
    let empty = s.doc.to_json();
    let request = json!({"id":"HDR-001","name":"Library CPU","set":{"width":64,"height":32,"job":{"method":"cpu_raytrace","options":{}}}});
    let reply = s.execute("render.library.environment.apply", &request).unwrap();
    assert_eq!(reply["source_sha256"], reply["resource"]);
    assert_eq!(reply["native_rotation_rad"], std::f64::consts::PI);
    assert_eq!(reply["self_contained"], true);
    let Some(RenderJob::CpuRaytrace { options }) = &s.doc.render.setups["Library CPU"].job else {
        panic!("CPU intent");
    };
    let id = options.hdr.unwrap().resource;
    assert_eq!(s.doc.render.resources.required(&id).unwrap().bytes(), super::super::library_data::asset("HDR-001").unwrap().0);
    let saved = s.doc.to_json();
    let reopened = solidcraft_doc::Document::from_json(&saved).unwrap();
    assert_eq!(reopened.render.resources.required(&id).unwrap().id(), id);
    let before = s.undo.len();
    assert!(s.execute("render.library.environment.apply", &request).is_err(), "replacement must be explicit");
    assert_eq!(s.doc.to_json(), saved);
    assert_eq!(s.undo.len(), before);
    let mut identical = request;
    identical["replace_environment"] = json!(true);
    s.execute("render.library.environment.apply", &identical).unwrap();
    assert_eq!(s.undo.len(), before, "identical resource application is a no-op");
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(s.doc.to_json(), empty);
}

#[test]
fn authored_hdr_orientation_matches_the_documented_longitude_origin() {
    use std::f64::consts::{PI, TAU};
    for u in [0.0, 0.125, 0.25, 0.5, 0.75, 0.999] {
        for rotation in [0.0, 0.3, -1.2] {
            let world_phi = TAU * (u - 0.5) + rotation;
            let native_u = ((world_phi - hdr_rotation(rotation, "test").unwrap()) / TAU).rem_euclid(1.0);
            let distance = (native_u - u).abs();
            assert!(distance.min(1.0 - distance) < 1e-12);
        }
    }
    assert_eq!(hdr_rotation(0.0, "test").unwrap(), PI);
    for v in [f64::NAN, f64::INFINITY, 1e7] {
        assert!(hdr_rotation(v, "test").is_err());
    }
}

#[test]
fn explicit_surface_prepare_has_physical_scale_and_does_not_assign_materials() {
    let mut s = Session::default();
    s.execute("solid.box", &json!({"length":10,"width":20,"height":5,"body_name":"Target"})).unwrap();
    let before = s.doc.to_json();
    let undo = s.undo.len();
    let reply = s.execute("render.library.surface.prepare", &json!({"id":"SURF-001","body_index":0,"body_name":"Target","scale":2.0})).unwrap();
    assert_eq!(reply["texture"]["frame"]["u"], json!([200.0, 0.0, 0.0]));
    assert_eq!(reply["texture"]["flip_v"], true);
    assert_eq!(reply["texture"]["base_color_mode"], "replace");
    assert!(reply["texture"].get("height").is_none());
    let input: super::super::textures::Override = serde_json::from_value(reply["texture"].clone()).unwrap();
    let scene = crate::product_scene::extract(&s).unwrap();
    let textures = super::super::textures::capture(&scene, &[input], "test").unwrap().unwrap();
    let binding = &textures.bindings()[0];
    assert!(binding.flip_v && binding.roughness.is_some() && binding.metallic.is_some());
    assert_eq!(binding.base_color_mode, BaseColorMode::Replace);
    assert_eq!(s.doc.to_json(), before);
    assert_eq!(s.undo.len(), undo);
    assert!(s.execute("render.library.surface.prepare", &json!({"id":"SURF-001","body_index":0,"body_name":"Old name"})).is_err());
}

#[test]
fn uniform_appearance_requires_acknowledged_projection_and_preserves_physical_data() {
    let mut s = Session::default();
    s.execute("solid.box", &json!({"length":10,"width":20,"height":5,"body_name":"Target"})).unwrap();
    let before = s.doc.to_json();
    let before_value: Value = serde_json::from_str(&before).unwrap();
    let undo = s.undo.len();
    assert!(s.execute("render.library.appearance.apply", &json!({"id":"MAT-001","variant":"uniform_opaque","bodies":["Target"]})).is_err());
    assert_eq!(s.doc.to_json(), before);
    let reply = s.execute("render.library.appearance.apply", &json!({"id":"SURF-001","variant":"uniform_opaque","bodies":["Target"]})).unwrap();
    assert_eq!(reply["engineering_properties_unchanged"], true);
    let after: Value = serde_json::from_str(&s.doc.to_json()).unwrap();
    assert_eq!(after["materials"], before_value["materials"]);
    assert_eq!(s.undo.len(), undo + 1);
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(s.doc.to_json(), before);
}

#[test]
fn studio_adapter_does_not_invent_passive_cards_calibration_or_extra_suns() {
    let mut s = Session::default();
    let request = json!({"id":"RIG-002","variant":"complete","center_mm":[10,20,30],"subject_extent_mm":100.0});
    let reply = s.execute("render.library.studio.prepare", &request).unwrap();
    let lighting: StudioLighting = serde_json::from_value(reply["lighting"].clone()).unwrap();
    assert_eq!(lighting.area_lights.iter().count(), 3);
    assert!(lighting.sun.is_none());
    for id in ["RIG-018", "RIG-019", "RIG-021", "RIG-023", "RIG-024"] {
        let mut p = request.clone();
        p["id"] = json!(id);
        assert!(s.execute("render.library.studio.prepare", &p).is_err());
    }
    let mut with_cards = request.clone();
    with_cards["id"] = json!("RIG-001");
    assert!(s.execute("render.library.studio.prepare", &with_cards).is_err());
    with_cards["variant"] = json!("emitters_only");
    assert_eq!(s.execute("render.library.studio.prepare", &with_cards).unwrap()["projection"]["passive_cards_omitted"], 1);
    let mut sun = request;
    sun["id"] = json!("RIG-020");
    let reply = s.execute("render.library.studio.prepare", &sun).unwrap();
    let lighting: StudioLighting = serde_json::from_value(reply["lighting"].clone()).unwrap();
    let actual = lighting.sun.unwrap();
    assert!((actual.angular_radius_rad - 0.00465).abs() < 1e-12);
    assert!(actual.direction[2] > 0.0, "runtime direction points toward the sun, not along incoming photons");
}
