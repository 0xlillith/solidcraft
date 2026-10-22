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

#[test]
fn rational_resolution_lock_preserves_anchor_rounding_and_rejects_without_partial_changes() {
    let mut edit = Resolution { locked: true, ..Default::default() };
    let mut size = [3840, 2160];
    size = edit.edit(size, 0, 1920, 8192).unwrap();
    assert_eq!(size, [1920, 1080]);
    size = edit.edit(size, 1, 720, 8192).unwrap();
    assert_eq!(size, [1280, 720]);
    let anchor = edit.anchor;
    for _ in 0..100 {
        size = edit.edit(size, 0, 1001, 8192).unwrap();
        assert_eq!(size, [1001, 563]);
        size = edit.edit(size, 0, 1920, 8192).unwrap();
        assert_eq!(size, [1920, 1080]);
    }
    assert_eq!(edit.anchor, anchor, "rounded edits must not drift the anchor");
    assert!(edit.edit(size, 1, 8192, 8192).is_none());
    assert_eq!(edit.observed, Some(size), "a refused linked dimension must not partly change size");
    assert!(edit.rejected);
    assert!(edit.edit(size, 0, 0, 8192).is_none());
    assert!(edit.edit(size, 2, 100, 8192).is_none());
    let mut extreme = Resolution { locked: true, ..Default::default() };
    assert!(extreme.edit([1, u32::MAX], 0, u32::MAX, u32::MAX).is_none());
    assert!(Resolution { locked: true, ..Default::default() }.edit([0, 1], 0, 1, 8192).is_none());
}

#[test]
fn resolution_lock_reanchors_for_explicit_presets_and_does_not_affect_other_intent() {
    let mut edit = Resolution { locked: true, ..Default::default() };
    assert_eq!(edit.edit([3840, 2160], 0, 1920, 8192), Some([1920, 1080]));
    edit.observe([1080, 1920]);
    assert_eq!(edit.edit([1080, 1920], 1, 960, 8192), Some([540, 960]));
    edit.observe([1000, 1000]);
    assert_eq!(edit.edit([1000, 1000], 0, 777, 8192), Some([777, 777]));
    edit.locked = false;
    assert_eq!(edit.edit([777, 777], 0, 555, 8192), Some([555, 777]));
    edit.locked = true;
    assert_eq!(edit.edit([555, 777], 0, 1110, 8192), Some([1110, 1554]));
}

#[test]
fn actual_output_ratio_and_orientation_controls_are_local_and_preserve_saved_capture() {
    let mut h = fixture();
    super::super::select_image_method(&mut h.app, ImageMethod::GpuRaytrace);
    h.app.render_workspace.view.draft.width = 1920;
    h.app.render_workspace.view.draft.height = 1080;
    h.app.render_workspace.view.samples = 37;
    h.app.render_workspace.view.inspector.page = Page::Output;
    setups::save(&mut h.app).unwrap();
    let saved = h.app.session.doc.to_json();
    let camera = h.app.cam;
    h.frames(3);
    h.step(&json!({"click":{"handle":"render:aspect-lock"}})).unwrap();
    assert!(h.app.render_workspace.view.inspector.resolution.locked);
    assert!(!h.app.render_workspace.view.dirty, "coupling editor coordinates is not a saved numerical edit");
    h.step(&json!({"click":{"handle":"render:swap-size"}})).unwrap();
    assert_eq!((h.app.render_workspace.view.draft.width, h.app.render_workspace.view.draft.height), (1080, 1920));
    assert_eq!(h.app.render_workspace.view.samples, 37);
    assert_eq!(h.app.cam, camera);
    assert_eq!(h.app.session.doc.to_json(), saved);
    assert!(h.app.render_workspace.view.dirty);
    setups::load(&mut h.app);
    h.frames(3);
    assert_eq!(h.app.render_workspace.view.inspector.resolution.anchor, Some([1920, 1080]));
}

#[test]
fn contextual_scope_notes_wrap_with_native_tokens_instead_of_clipping() {
    let mut h = fixture();
    h.app.render_workspace.view.inspector.page = Page::Scene;
    h.frames(3);
    let rect = crate::scenario::rect_of("dialog:note:render.scene_preserves_design").unwrap();
    let inspector = crate::scenario::rect_of("render:inspector").unwrap();
    let footer = crate::scenario::rect_of("render:job-footer").unwrap();
    assert!(rect.height() > Tokens::get().font_small + Tokens::get().space_8, "long scope notice must have more than one line");
    assert!(inspector.contains_rect(rect));
    assert!(rect.bottom() < footer.top());
}

#[test]
fn temporary_library_maps_preserve_the_reviewed_plane_scale_and_optical_exclusivity() {
    let mut h = fixture();
    super::super::select_image_method(&mut h.app, ImageMethod::GpuRaytrace);
    let targets = h.app.run("render.scene", json!({})).unwrap();
    let target: materials::Target = serde_json::from_value(targets["bodies"][0].clone()).unwrap();
    let before = h.app.session.doc.to_json();
    materials::install_library_surface(&mut h.app, "SURF-001", target.clone(), "yz", 2.5).unwrap();
    let binding = &h.app.render_workspace.view.materials.bindings[0];
    assert_eq!(binding.maps.frame.u, [0.0, 250.0, 0.0]);
    assert_eq!(binding.maps.frame.v, [0.0, 0.0, 250.0]);
    assert_eq!(h.app.session.doc.to_json(), before, "temporary maps must not change the native model");
    let original = binding.maps.frame;
    assert!(materials::install_library_surface(&mut h.app, "SURF-001", target.clone(), "unknown", 1.0).is_err());
    assert!(materials::install_library_surface(&mut h.app, "SURF-001", target.clone(), "xy", 0.0).is_err());
    assert_eq!(h.app.render_workspace.view.materials.bindings[0].maps.frame, original);
    h.app.render_workspace.view.materials.bindings[0].glass_enabled = true;
    assert!(materials::install_library_surface(&mut h.app, "SURF-001", target, "xy", 1.0).is_err());
    assert!(h.app.render_workspace.view.materials.bindings[0].glass_enabled);
    assert_eq!(h.app.session.doc.to_json(), before);
}

#[test]
fn native_composition_review_actions_are_visible_before_auxiliary_resource_details() {
    for dark in [false, true] {
        let mut h = fixture();
        h.call("ui.set", json!({"dark":dark}));
        super::super::select_image_method(&mut h.app, ImageMethod::GpuRaytrace);
        setups::save(&mut h.app).unwrap();
        library::open_category(&mut h.app, "native_scenes");
        h.frames(3);
        h.step(&json!({"click":{"handle":"render:library:NATIVE-STUDIO-002"}})).unwrap();
        let footer = crate::scenario::rect_of("render:job-footer").unwrap();
        let resolve = crate::scenario::rect_of("render:scene-resolve").unwrap();
        assert!(resolve.bottom() < footer.top(), "the review action must precede supplementary prose");
        h.step(&json!({"click":{"handle":"render:scene-resolve"}})).unwrap();
        h.frames(3);
        for name in ["render:scene-consent", "render:scene-apply"] {
            let rect = crate::scenario::rect_of(name).unwrap();
            assert!(rect.bottom() < footer.top(), "{name} must remain above the pinned job actions: rect={rect:?}, footer={footer:?}, dark={dark}");
        }
    }
}

#[test]
fn ribbon_navigation_reuses_inspector_and_preserves_exact_unsaved_intent() {
    let mut h = fixture();
    h.app.render_workspace.view.samples = 37;
    h.app.render_workspace.view.draft.width = 777;
    let doc = h.app.session.doc.to_json();
    let undo = h.app.session.undo.len();
    for (id, page) in [
        ("render.preset.resolve", Page::Scene),
        ("render.settings.get", Page::Scene),
        ("render.library.appearance.apply", Page::Appearance),
        ("render.library.surface.bind", Page::Appearance),
        ("render.library.studio.prepare", Page::Lighting),
        ("render.library.studio.apply", Page::Lighting),
        ("render.library.environment.apply", Page::Environment),
        ("render.stage.backdrop", Page::Scene),
        ("render.quality.prepare", Page::Output),
        ("render.status", Page::Diagnostics),
        ("render.camera.get", Page::Camera),
    ] {
        h.app.start(id);
        h.frames(3);
        assert_eq!(h.app.render_workspace.view.inspector.page, page, "{id}");
        assert_eq!(h.app.render_workspace.view.samples, 37);
        assert_eq!(h.app.render_workspace.view.draft.width, 777);
        assert_eq!(h.app.session.doc.to_json(), doc, "navigation may not execute a parameterless apply");
        assert_eq!(h.app.session.undo.len(), undo);
        assert!(!crate::icon_svg::resolve(solidcraft_engine::find_command(id).unwrap().icon).placeholder);
    }
}

#[test]
fn render_ribbon_has_six_shared_groups_and_keeps_image_actions_at_narrow_widths() {
    assert_eq!(crate::workspace::panels("RENDER"), vec!["SCENE", "APPEARANCE", "LIGHTING", "ENVIRONMENT", "CAMERA", "IMAGE"]);
    for layout in [crate::shell::LayoutMode::Docked, crate::shell::LayoutMode::Floating] {
        let mut h = fixture();
        h.app.ui.layout = layout;
        for width in [800, 1024, 1440, 2000] {
            h.call("ui.resize", json!({"width":width,"height":900}));
            h.frames(3);
            for id in ["render.start", "render.preflight"] {
                let r = crate::scenario::rect_of(&format!("button:{id}")).expect("pinned Image action");
                assert!(r.left() >= 0. && r.right() <= width as f32, "{layout:?} {width} {id}: {r:?}");
            }
        }
        h.app.toolbar_custom.panels.insert("DESIGN/RENDER/SCENE".into(), vec!["render.scene".into()]);
        h.app.toolbar_custom.panels.insert("DESIGN/RENDER/IMAGE".into(), vec!["render.export".into(), "render.start".into()]);
        let before = h.app.toolbar_custom.prefs();
        h.frames(3);
        assert_eq!(h.app.toolbar_custom.prefs(), before, "default expansion must not replace personal pins");
        assert!(crate::scenario::rect_of("button:render.scene").is_some());
        assert!(crate::scenario::rect_of("button:render.export").is_some());
    }
}

#[test]
fn ribbon_and_sticky_footer_submit_the_current_draft_not_defaults_or_saved_shot() {
    let mut h = fixture();
    h.app.render_workspace.view.draft.width = 32;
    h.app.render_workspace.view.draft.height = 24;
    h.app.render_workspace.view.draft.background = [24, 63, 107];
    let before = h.app.session.doc.to_json();
    let export = solidcraft_engine::find_command("render.export").unwrap();
    assert!(!super::super::interactive_command_info(&h.app, export).enabled);
    assert!(export.info(&h.app.session).enabled, "UI state must not alter direct engine API semantics");
    h.app.start("render.preflight");
    let request = super::super::image_request(&h.app).unwrap();
    h.app.start("render.start");
    let job = h.app.render_workspace.view.job.expect("ribbon submitted an actual job");
    let handle = h.app.session.render_job_handle(job).unwrap();
    assert_eq!((handle.status().unwrap().request.width, handle.status().unwrap().request.height), (32, 24));
    assert_eq!(handle.status().unwrap().request.background, [24, 63, 107]);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while h.app.render_workspace.view.completed != Some(job) {
        assert!(std::time::Instant::now() < deadline, "CPU fixture did not complete");
        h.frames(1);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(super::super::image_request(&h.app).unwrap(), request);
    assert_eq!(h.app.session.doc.to_json(), before);
    assert!(super::super::interactive_command_info(&h.app, export).enabled);
    h.app.render_workspace.view.studio.export_exr = true;
    assert!(!super::super::interactive_command_info(&h.app, export).enabled, "a PNG-only result never pretends to have EXR");
}

#[test]
fn saved_bloom_intent_roundtrips_and_rejects_unsupported_transparent_output() {
    use solidcraft_engine::render::job::bloom::Bloom;
    let mut h = fixture();
    super::super::select_image_method(&mut h.app, ImageMethod::GpuRaytrace);
    h.app.render_workspace.view.studio.finishing.bloom = Some(Bloom { strength: 0.25, threshold: 2.5, radius: 12 });
    let expected = setups::snapshot(&h.app).unwrap();
    setups::save(&mut h.app).unwrap();
    h.app.render_workspace.view.studio.finishing.bloom = None;
    setups::load(&mut h.app);
    assert_eq!(setups::snapshot(&h.app).unwrap(), expected);
    let reopened = solidcraft_engine::doc::Document::from_json(&h.app.session.doc.to_json()).unwrap();
    assert_eq!(reopened.render.setups[&h.app.render_workspace.view.name], expected);
    h.app.render_workspace.view.transparent = true;
    assert!(setups::snapshot(&h.app).is_err());
    h.app.render_workspace.view.transparent = false;
    super::super::select_image_method(&mut h.app, ImageMethod::CpuRaytrace);
    assert!(super::super::image_request(&h.app).is_ok(), "inactive GPU settings remain portable but are not sent to CPU");
}
