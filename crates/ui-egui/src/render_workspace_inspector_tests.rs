use super::*;
use crate::scenario::Harness;
fn fixture() -> Harness {
    let mut h = Harness::new();
    h.call("ui.resize", json!({"width":1440,"height":1000}));
    h.app.run("solid.box", json!({"length":20,"width":15,"height":10,"body_name":"Subject"})).unwrap();
    super::super::enter(&mut h.app, true).unwrap();
    h.frames(3);
    h
}
#[test]
fn native_inspector_defaults_to_scene_and_keeps_job_actions_on_screen_in_both_shells() {
    for dark in [false, true] {
        for layout in [crate::shell::LayoutMode::Docked, crate::shell::LayoutMode::Floating] {
            let mut h = fixture();
            h.app.preferences.theme = if dark { "dark" } else { "light" }.into();
            h.app.ui.layout = layout;
            h.call("ui.resize", json!({"width":1280,"height":900}));
            assert_eq!(h.app.render_workspace.view.inspector.page, Page::Scene);
            let before = h.app.session.doc.to_json();
            let revision = h.app.session.revision;
            for page in Page::ALL {
                h.app.render_workspace.view.inspector.page = page;
                h.frames(3);
                let inspector = crate::scenario::rect_of("render:inspector").unwrap();
                let footer = crate::scenario::rect_of("render:job-footer").unwrap();
                assert!(inspector.contains_rect(footer), "footer outside inspector at {page:?}: {inspector:?} {footer:?}");
                for name in ["render:check", "render:start", "render:cancel", "render:export"] {
                    let target = crate::scenario::rect_of(name).unwrap();
                    assert!(footer.contains_rect(target), "{name} clipped at {page:?}");
                }
                assert_eq!(h.app.session.doc.to_json(), before);
                assert_eq!(h.app.session.revision, revision);
            }
        }
    }
}
#[test]
fn icon_page_navigation_is_local_and_never_switches_executor_or_overwrites_drafts() {
    let mut h = fixture();
    let before = h.app.session.doc.to_json();
    h.app.render_workspace.view.samples = 37;
    h.app.render_workspace.view.draft.width = 777;
    for page in [Page::Appearance, Page::Camera, Page::Output, Page::Diagnostics, Page::Scene] {
        h.step(&json!({"click":{"handle":format!("render:page:{}",page.handle())}})).unwrap();
        assert_eq!(h.app.render_workspace.view.inspector.page, page);
        assert_eq!(h.app.render_workspace.view.method, ImageMethod::Raster);
        assert_eq!(h.app.render_workspace.view.samples, 37);
        assert_eq!(h.app.render_workspace.view.draft.width, 777);
        assert_eq!(h.app.session.doc.to_json(), before);
    }
}
#[test]
fn dirty_shot_switch_keep_discard_and_save_are_explicit_and_lossless() {
    let mut h = fixture();
    h.app.render_workspace.view.name = "A".into();
    setups::save(&mut h.app).unwrap();
    h.app.run("render.settings.set", json!({"name":"B","set":{"width":321,"job":{"method":"raster"}}})).unwrap();
    h.app.render_workspace.view.draft.width = 123;
    setups::refresh_dirty(&mut h.app);
    request_action(&mut h.app, PendingAction::Switch("B".into()));
    h.frames(3);
    h.step(&json!({"widget":h.app.tr("render.keep_editing")})).unwrap();
    assert_eq!(h.app.render_workspace.view.name, "A");
    assert_eq!(h.app.render_workspace.view.draft.width, 123);
    request_action(&mut h.app, PendingAction::Switch("B".into()));
    h.frames(3);
    h.step(&json!({"widget":h.app.tr("render.discard_continue")})).unwrap();
    assert_eq!(h.app.render_workspace.view.name, "B");
    assert_eq!(h.app.render_workspace.view.draft.width, 321);
    assert_ne!(h.app.session.doc.render.setups["A"].width, 123);
    h.app.render_workspace.view.draft.width = 456;
    setups::refresh_dirty(&mut h.app);
    request_action(&mut h.app, PendingAction::Switch("A".into()));
    h.frames(3);
    h.step(&json!({"widget":h.app.tr("render.save_continue")})).unwrap();
    assert_eq!(h.app.session.doc.render.setups["B"].width, 456);
    assert_eq!(h.app.render_workspace.view.name, "A");
}
#[test]
fn expired_confirmation_cannot_remove_a_changed_shot() {
    let mut h = fixture();
    setups::save(&mut h.app).unwrap();
    let name = h.app.render_workspace.view.name.clone();
    request_action(&mut h.app, PendingAction::Remove(name.clone()));
    h.app.run("solid.box", json!({"length":1,"width":1,"height":1,"body_name":"Later"})).unwrap();
    h.frames(3);
    assert!(h.app.render_workspace.view.inspector.pending.is_none());
    assert!(h.app.session.doc.render.setups.contains_key(&name));
}
#[test]
fn projected_quality_changes_only_requested_budget_fields_and_has_no_runtime_claim() {
    let mut h = fixture();
    super::super::select_image_method(&mut h.app, ImageMethod::GpuRaytrace);
    h.app.cam.fov = 0.6;
    h.app.render_workspace.view.studio.intensity = 0.42;
    h.app.render_workspace.view.draft.exposure_ev = 1.23;
    h.app.render_workspace.view.studio.ambient = [0.11, 0.22, 0.33];
    let before = h.app.session.doc.to_json();
    let camera = h.app.cam;
    let plan = h.app.run("render.quality.prepare", json!({"id":"Q-002","variant":"fixed_samples_png","acknowledge_omissions":true})).unwrap();
    apply_quality(&mut h.app, &plan).unwrap();
    assert_eq!((h.app.render_workspace.view.draft.width, h.app.render_workspace.view.draft.height), (1600, 900));
    assert_eq!(h.app.render_workspace.view.samples, 512);
    assert_eq!(h.app.cam, camera);
    assert_eq!(h.app.render_workspace.view.studio.intensity, 0.42);
    assert_eq!(h.app.render_workspace.view.studio.ambient, [0.11, 0.22, 0.33]);
    assert_eq!(h.app.render_workspace.view.draft.exposure_ev, 1.23);
    assert_eq!(h.app.session.doc.to_json(), before);
    super::super::select_image_method(&mut h.app, ImageMethod::CpuRaytrace);
    assert!(apply_quality(&mut h.app, &plan).is_err());
}
#[test]
fn complete_native_studio_can_be_reviewed_and_applied_from_real_ui_controls() {
    let mut h = fixture();
    super::super::select_image_method(&mut h.app, ImageMethod::GpuRaytrace);
    setups::save(&mut h.app).unwrap();
    library::open_category(&mut h.app, "native_scenes");
    h.frames(4);
    h.step(&json!({"click":{"handle":"render:library:NATIVE-STUDIO-002"}})).unwrap();
    let before = h.app.session.doc.to_json();
    h.step(&json!({"widget":h.app.tr("render.scene_review")})).unwrap();
    assert_eq!(h.app.session.doc.to_json(), before);
    assert!(h.app.render_workspace.view.inspector.scene_review.is_some());
    h.step(&json!({"widget":h.app.tr("render.scene_consent")})).unwrap();
    h.step(&json!({"widget":h.app.tr("render.scene_apply")})).unwrap();
    assert!(!h.app.render_workspace.view.draft.stage.is_empty());
    assert!(h.app.render_workspace.view.inspector.scene_review.is_none());
    assert!(!h.app.render_workspace.view.dirty);
}
#[test]
fn unsupported_output_is_visible_and_never_fakes_export_or_clamps_intent() {
    let mut h = fixture();
    super::super::select_image_method(&mut h.app, ImageMethod::GpuRaytrace);
    h.app.render_workspace.view.inspector.page = Page::Output;
    h.app.render_workspace.view.draft.width = 7000;
    h.app.render_workspace.view.draft.height = 6000;
    h.app.render_workspace.view.samples = 37;
    h.frames(4);
    assert_eq!((h.app.render_workspace.view.draft.width, h.app.render_workspace.view.draft.height), (7000, 6000));
    assert_eq!(h.app.render_workspace.view.samples, 37);
    assert!(super::super::image_request(&h.app).is_err());
    assert!(h.app.render_workspace.view.completed.is_none());
    assert!(h.app.tr("render.output_format_status").contains("EXR"));
}
