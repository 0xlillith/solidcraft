use super::*;
fn subject() -> Session {
    let mut s = Session::default();
    s.execute("solid.box", &json!({"length":20,"width":15,"height":10,"body_name":"Subject"})).unwrap();
    s.execute(
        "render.settings.set",
        &json!({"name":"Shot","set":{"width":96,"height":64,
        "job":{"method":"gpu_path","options":{"samples":37,"max_depth":7,"seed":91}}}}),
    )
    .unwrap();
    s
}
#[test]
fn native_studios_resolve_without_mutation_and_apply_one_complete_undo() {
    for id in ["NATIVE-STUDIO-001", "NATIVE-STUDIO-002", "NATIVE-STUDIO-003", "NATIVE-STUDIO-004"] {
        let mut s = subject();
        let before = s.doc.to_json();
        let undo = s.undo.len();
        let old = s.doc.render.setups["Shot"].clone();
        let revision = s.revision;
        let plan = s.execute("render.preset.resolve", &json!({"id":id,"name":"Shot","expected_revision":revision})).unwrap();
        assert_eq!(plan["document_changed"], false);
        assert_eq!(s.doc.to_json(), before);
        assert_eq!(s.undo.len(), undo);
        let request = json!({"id":id,"name":"Shot","expected_revision":revision,"replace_lighting_and_environment":true});
        let applied = s.execute("render.preset.apply", &request).unwrap();
        assert_eq!(applied["document_changed"], true);
        assert_eq!(s.undo.len(), undo + 1);
        let saved = &s.doc.render.setups["Shot"];
        assert_eq!((saved.width, saved.height, saved.camera), (old.width, old.height, old.camera));
        let Some(RenderJob::GpuPath { options, .. }) = &saved.job else { panic!("explicit GPU intent lost") };
        assert_eq!((options.samples, options.max_depth, options.seed), (37, 7, 91));
        assert!(options.hdr.is_some());
        assert!(!saved.stage.is_empty());
        assert_eq!(s.doc.features.len(), 1);
        assert!(s.doc.materials.is_empty());
        let reopened = solidcraft_doc::Document::from_json(&s.doc.to_json()).unwrap();
        assert_eq!(reopened.render.setups["Shot"], *saved);
        s.execute("edit.undo", &json!({})).unwrap();
        assert_eq!(s.doc.to_json(), before);
    }
}
#[test]
fn native_studio_refusals_are_atomic_and_never_silently_replace_resources() {
    let mut s = subject();
    let revision = s.revision;
    let before = s.doc.to_json();
    for request in [
        json!({"id":"SCENE-001","name":"Shot","expected_revision":revision,"replace_lighting_and_environment":true}),
        json!({"id":"NATIVE-STUDIO-001","name":"Shot","expected_revision":revision}),
        json!({"id":"NATIVE-STUDIO-001","name":"Shot","replace_lighting_and_environment":true}),
        json!({"id":"NATIVE-STUDIO-001","name":"Shot","expected_revision":revision+1,"replace_lighting_and_environment":true}),
        json!({"id":"NATIVE-STUDIO-001","name":"Shot","expected_revision":revision,"replace_lighting_and_environment":true,"set":{"job":{"method":"raster"}}}),
    ] {
        assert!(s.execute("render.preset.apply", &request).is_err());
        assert_eq!(s.doc.to_json(), before);
    }
    s.execute("render.stage.ground", &json!({"name":"Shot"})).unwrap();
    let before = s.doc.to_json();
    let revision = s.revision;
    assert!(
        s.execute(
            "render.preset.apply",
            &json!({"id":"NATIVE-STUDIO-001","name":"Shot","expected_revision":revision,
        "set":{"stage":[]},"replace_lighting_and_environment":true})
        )
        .is_err()
    );
    assert_eq!(s.doc.to_json(), before);
    let mut empty = Session::default();
    assert!(
        empty
            .execute("render.preset.resolve", &json!({"id":"NATIVE-STUDIO-001","name":"E","set":{"job":{"method":"gpu_path","options":{}}}}))
            .is_err()
    );
}
#[test]
fn saved_shot_commands_preserve_exact_resources_reject_collisions_and_undo() {
    let mut s = subject();
    let before = s.doc.to_json();
    let revision = s.revision;
    s.execute("render.shot.duplicate", &json!({"name":"Shot","new_name":"Copy","expected_revision":revision})).unwrap();
    assert_eq!(s.doc.render.setups["Shot"], s.doc.render.setups["Copy"]);
    let duplicated = s.doc.to_json();
    for (command, p) in [
        ("render.shot.duplicate", json!({"name":"Shot","new_name":"Copy","expected_revision":s.revision})),
        ("render.shot.rename", json!({"name":"Shot","new_name":"Copy","expected_revision":s.revision})),
        ("render.shot.remove", json!({"name":"Shot","expected_revision":revision})),
        ("render.shot.remove", json!({"name":"Shot","new_name":"other","expected_revision":s.revision})),
        ("render.shot.duplicate", json!({"name":"Shot","new_name":"x".repeat(129),"expected_revision":s.revision})),
    ] {
        assert!(s.execute(command, &p).is_err());
        assert_eq!(s.doc.to_json(), duplicated);
    }
    s.execute("edit.undo", &json!({})).unwrap();
    assert_eq!(s.doc.to_json(), before);
    s.execute("render.shot.rename", &json!({"name":"Shot","new_name":"Renamed","expected_revision":s.revision})).unwrap();
    assert!(!s.doc.render.setups.contains_key("Shot"));
    assert!(s.doc.render.setups.contains_key("Renamed"));
    s.execute("render.shot.remove", &json!({"name":"Renamed","expected_revision":s.revision})).unwrap();
    assert!(s.doc.render.setups.is_empty());
    s.execute("edit.undo", &json!({})).unwrap();
    assert!(s.doc.render.setups.contains_key("Renamed"));
}
#[test]
fn quality_projection_never_claims_adaptive_or_unbounded_profiles() {
    let mut s = subject();
    let before = s.doc.to_json();
    for id in ["Q-001", "Q-002", "Q-003"] {
        let plan = s.execute("render.quality.prepare", &json!({"id":id,"variant":"fixed_samples_png","acknowledge_omissions":true})).unwrap();
        assert_eq!(plan["document_changed"], false);
        assert_eq!(plan["status"], "fixed_budget_projection_not_full_quality_profile");
        assert!(plan["omitted"].as_array().unwrap().iter().any(|v| v == "adaptive_stopping_and_error_target"));
        assert!(plan["gpu_options"]["samples"].as_u64().unwrap() <= 4096);
        assert_eq!(s.doc.to_json(), before);
    }
    for id in ["Q-004", "Q-005", "Q-006", "Q-007", "Q-008", "Q-009", "Q-010", "missing"] {
        assert!(s.execute("render.quality.prepare", &json!({"id":id,"variant":"fixed_samples_png","acknowledge_omissions":true})).is_err());
    }
    assert!(s.execute("render.quality.prepare", &json!({"id":"Q-002","variant":"full","acknowledge_omissions":true})).is_err());
    assert!(s.execute("render.quality.prepare", &json!({"id":"Q-002","variant":"fixed_samples_png","acknowledge_omissions":false})).is_err());
    assert_eq!(s.doc.to_json(), before);
}
#[test]
fn resource_availability_and_original_previews_are_bounded_and_nonmutating() {
    let mut s = subject();
    let before = s.doc.to_json();
    assert_eq!(s.execute("render.library.list", &json!({"kind":"native_scenes","availability":"usable"})).unwrap()["total"], 4);
    assert_eq!(s.execute("render.library.list", &json!({"kind":"materials","availability":"usable"})).unwrap()["total"], 0);
    assert_eq!(s.execute("render.library.list", &json!({"kind":"materials","availability":"specification"})).unwrap()["total"], 600);
    assert_eq!(s.execute("render.library.list", &json!({"kind":"selene_shots"})).unwrap()["total"], 12);
    assert!(s.execute("render.library.list", &json!({"availability":"invented"})).is_err());
    let preview = s.execute("render.library.preview", &json!({"id":"SURF-001"})).unwrap();
    let bytes: Vec<u8> = serde_json::from_value(preview["bytes"].clone()).unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(preview["sha256"], json!(solidcraft_doc::render_resources::HdrId::of(&bytes)));
    assert_eq!(preview["fidelity"], "authored_base_color_source_map_not_a_render");
    for id in ["../SURF-001", "HDR-001", "MAT-001", "SURF-999"] {
        assert!(s.execute("render.library.preview", &json!({"id":id})).is_err());
    }
    let targets = s.execute("render.library.targets", &json!({"query":"Subject"})).unwrap();
    assert_eq!(targets["items"][0]["name"], "Subject");
    assert_eq!(targets["items"][0]["opacity"], 1.0);
    assert_eq!(s.doc.to_json(), before);
}
