use super::*;
use crate::scenario::Harness;
use solidcraft_engine::Session;

#[test]
fn control_render_export_respects_upstream_file_scope() {
    let mut h = Harness::new();
    let root = h.dir.join("allowed");
    let outside = h.dir.join("outside");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    let roots = json!([root.to_string_lossy()]);
    let response = h.call("ui.workspace", json!({"workspace":"render","allow_paths":roots}));
    assert_eq!(response["ok"], true, "{response}");
    let response = h.call("engine.execute", json!({"command":"render.start","params":{"settings":{"width":16,"height":16}},"allow_paths":roots}));
    assert_eq!(response["ok"], true, "{response}");
    let job = response["result"]["job"].clone();
    for _ in 0..50 {
        h.frames(1);
    }
    assert_eq!(h.app.session.execute("render.status", &json!({"job":job})).unwrap()["progress"]["phase"], "completed");
    for path in [outside.join("denied.png"), root.join("../outside/traversal.png")] {
        for overwrite in [false, true] {
            let response = h.call(
                "engine.execute",
                json!({"command":"render.export","params":{"job":job,"path":path,"overwrite":overwrite},"allow_paths":roots}),
            );
            assert_eq!(response["ok"], false, "{response}");
            assert!(response.to_string().contains("outside the folders this agent may use"), "{response}");
        }
    }
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
    let path = root.join("allowed.png");
    let response = h.call("engine.execute", json!({"command":"render.export","params":{"job":job,"path":path},"allow_paths":roots}));
    assert_eq!(response["ok"], true, "{response}");
    assert!(path.is_file());
    let original = std::fs::read(&path).unwrap();
    let response = h.call("engine.execute", json!({"command":"render.export","params":{"job":job,"path":path,"overwrite":true},"allow_paths":roots}));
    assert_eq!(response["ok"], true, "{response}");
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn adopting_into_blank_document_resets_render_draft_and_preserves_hosts() {
    let mut h = Harness::new();
    h.app.render_workspace.host = Some(host_executor(2048, true, 4).unwrap());
    sync_backend(&mut h.app);
    h.app.render_workspace.view.name = "Old draft".into();
    enter(&mut h.app, true).unwrap();
    let mut incoming = Session::default();
    incoming.execute("solid.box", &json!({"length":10,"width":10,"height":10})).unwrap();
    crate::documents::adopt(&mut h.app, incoming);
    assert!(!active(&h.app));
    assert_eq!(h.app.render_workspace.view.name, "Studio");
    assert_eq!(h.app.session.render_backend().executors.len(), 3);
    assert!(h.app.session.render_job_host().is_some());
    assert_eq!(h.app.session.model.state().bodies.len(), 1);
}

#[test]
fn clicked_image_controls_complete_cancel_and_export_without_dirtying_the_design() {
    for language in ["en", "de", "ko"] {
        let mut h = Harness::new();
        h.app.preferences.language = language.into();
        h.frames(1);
        let check = h.app.tr("render.image_check").to_string();
        let start = h.app.tr("render.image_start").to_string();
        let export = h.app.tr("render.image_export").to_string();
        let cancel = h.app.tr("render.image_cancel").to_string();
        h.call("ui.resize", json!({"width":1600,"height":1400}));
        enter(&mut h.app, true).unwrap();
        h.app.render_workspace.view.draft.width = 16;
        h.app.render_workspace.view.draft.height = 16;
        h.app.render_workspace.view.dirty = true;
        h.frames(3);
        let doc = h.app.session.doc.clone();
        let undo = h.app.session.undo.len();
        h.step(&json!({"widget":check})).unwrap();
        h.step(&json!({"widget":start})).unwrap();
        for _ in 0..50 {
            h.frames(1);
            if h.app.render_workspace.view.completed.is_some() {
                break;
            }
        }
        let good = h.app.render_workspace.view.completed.unwrap();
        h.step(&json!({"widget":export})).unwrap();
        assert!(h.dir.join("render.png").is_file());
        h.app.render_workspace.view.draft.width = 1024;
        h.app.render_workspace.view.draft.height = 768;
        h.step(&json!({"widget":start})).unwrap();
        h.step(&json!({"widget":cancel})).unwrap();
        h.frames(5);
        assert_eq!(h.app.render_workspace.view.completed, Some(good));
        assert_eq!(
            h.app.session.execute("render.status", &json!({"job":h.app.render_workspace.view.job})).unwrap()["progress"]["phase"],
            "cancelled"
        );
        assert!(Arc::ptr_eq(&doc, &h.app.session.doc));
        assert_eq!(h.app.session.undo.len(), undo);
        assert!(!h.app.session.is_dirty());
    }
}

#[test]
fn saved_camera_survives_frames_and_product_fit_ignores_selection() {
    let mut h = Harness::new();
    h.app.run("solid.box", json!({"length":10,"width":20,"height":5,"body_name":"Product"})).unwrap();
    h.app.run("solid.box", json!({"corner":[100,0,0],"length":10,"width":20,"height":5,"body_name":"Other"})).unwrap();
    let camera = Camera { up: solidcraft_engine::render::UpAxis::Y, roll: 0.2, ..Default::default() };
    h.app.run("render.settings.set", json!({"name":"Studio","set":{"camera":camera,"background":[21,43,65]}})).unwrap();
    h.frames(3);
    let design = h.app.cam;
    enter(&mut h.app, true).unwrap();
    h.frames(3);
    h.step(&json!({"widget":"Restore Saved Camera"})).unwrap();
    h.frames(3);
    assert_eq!(h.app.cam, camera);
    assert_eq!(h.app.render_workspace.view.draft.background, [21, 43, 65]);
    h.app.session.selection = vec![solidcraft_engine::Sel::Body { name: "Product".into() }];
    let bounds = solidcraft_engine::product_scene::extract(&h.app.session).unwrap().bounds;
    let mut expected = camera;
    expected.fit(&bounds);
    h.app.animate_view("fit");
    h.frames(3);
    assert_eq!(h.app.cam, expected);
    enter(&mut h.app, false).unwrap();
    assert_eq!(h.app.cam, design);
}

#[test]
fn clicked_render_workspace_saves_one_setup_and_restores_design_view() {
    for language in ["en", "de", "ko"] {
        let mut h = Harness::new();
        h.app.preferences.language = language.into();
        h.app.run("solid.box", json!({"length":10,"width":20,"height":5})).unwrap();
        h.app.session.mark_saved();
        h.frames(3);
        let camera = h.app.cam;
        let flags = (h.app.ui.show_grid, h.app.ui.show_origin, h.app.ui.visual_style, h.app.ui.perspective);
        let undo = h.app.session.undo.len();
        let at = crate::scenario::rect_of("workspace_switcher").unwrap().center();
        h.step(&json!({"click":[at.x,at.y]})).unwrap();
        h.step(&json!({"click":{"handle":"menu:ui.workspace.render"}})).unwrap();
        assert!(active(&h.app));
        let heading = h.app.tr("cmd.render_settings_get").to_string();
        let perspective = h.app.tr("render.perspective").to_string();
        let save = h.app.tr("render.save_setup").to_string();
        let back = h.app.tr("render.return_design").to_string();
        assert!(h.widget(&heading).is_some(), "{language}");
        h.step(&json!({"widget":perspective})).unwrap();
        assert!(!h.app.session.is_dirty(), "preview edits are staged");
        h.step(&json!({"widget":save})).unwrap();
        assert_eq!(h.app.session.undo.len(), undo + 1);
        assert_eq!(h.app.session.doc.render.setups["Studio"].camera, h.app.cam);
        h.step(&json!({"widget":back})).unwrap();
        assert!(!active(&h.app));
        assert_eq!(h.app.cam, camera);
        assert_eq!(flags, (h.app.ui.show_grid, h.app.ui.show_origin, h.app.ui.visual_style, h.app.ui.perspective));
        h.app.run("edit.undo", json!({})).unwrap();
        assert!(!h.app.session.is_dirty());
    }
}

#[test]
fn render_drafts_and_capabilities_are_isolated_across_document_switches() {
    let mut h = Harness::new();
    h.app.render_workspace.host = Some(host_executor(2048, true, 4).unwrap());
    sync_backend(&mut h.app);
    h.app.run("solid.box", json!({"length":10,"width":10,"height":10})).unwrap();
    h.app.render_workspace.view.name = "First draft".into();
    h.app.render_workspace.view.draft.background = [20, 40, 60];
    h.app.render_workspace.view.dirty = true;
    enter(&mut h.app, true).unwrap();
    crate::documents::open_tab(&mut h.app, Session::default()).unwrap();
    assert!(!active(&h.app));
    assert_eq!(h.app.render_workspace.view.name, "Studio");
    assert_eq!(h.app.session.render_backend().executors.len(), 3);
    crate::documents::switch(&mut h.app, 0);
    assert_eq!(h.app.render_workspace.view.name, "First draft");
    assert_eq!(h.app.render_workspace.view.draft.background, [20, 40, 60]);
    assert!(h.app.render_workspace.view.dirty);
    assert_eq!(h.app.session.render_backend().executors.len(), 3);
    h.app.run("file.new", json!({})).unwrap();
    assert_eq!(h.app.render_workspace.view.name, "Studio");
    assert_eq!(h.app.session.render_backend().executors.len(), 3);
}

#[test]
fn product_gpu_buffers_preserve_opacity_and_never_contain_overlays() {
    let mut s = Session::default();
    s.execute("solid.box", &json!({"length":10,"width":10,"height":10,"body_name":"Glass"})).unwrap();
    s.execute("appearance.assign", &json!({"bodies":["Glass"],"color":"#336699","opacity":0.4})).unwrap();
    let scene = solidcraft_engine::product_scene::extract(&s).unwrap();
    let (key, gpu) = gpu_scene(&scene).unwrap();
    assert!(gpu.tris.is_empty());
    assert!(!gpu.glass.is_empty());
    assert!(gpu.glass.as_chunks::<{ crate::gpu::TRI_SIZE }>().0.iter().all(|v| v[24..] == [51, 102, 153, 102]));
    assert!(gpu.lines.is_empty() && gpu.overlay.is_empty() && gpu.under.is_empty() && gpu.hidden.is_empty() && gpu.ghost.is_empty());
    s.selection = vec![solidcraft_engine::Sel::Body { name: "Glass".into() }];
    s.section = Some((solidcraft_engine::geom::Vec3::ZERO, solidcraft_engine::geom::Vec3::X));
    let after = solidcraft_engine::product_scene::extract(&s).unwrap();
    assert_eq!(gpu_scene(&after).unwrap().0, key);
    s.execute("appearance.assign", &json!({"bodies":["Glass"],"color":"#336699","opacity":0.0})).unwrap();
    let invisible = solidcraft_engine::product_scene::extract(&s).unwrap();
    assert_eq!(invisible.meshes[0].opacity, 0.0);
    let (_, empty) = gpu_scene(&invisible).unwrap();
    assert!(empty.tris.is_empty() && empty.glass.is_empty());
}

#[test]
fn host_profiles_do_not_advertise_image_export_final_or_features() {
    use solidcraft_engine::render::*;
    let host = host_executor(512, true, 4).unwrap();
    assert_eq!(host.limits.max_dimension, 512);
    assert_eq!(host.profiles.len(), 1);
    assert!(host.profiles[0].features.is_empty() && host.profiles[0].formats.is_empty());
    let mut s = Session::default();
    s.set_render_viewport(Some(host)).unwrap();
    let mut request = RenderRequest {
        backend: None,
        mode: RenderMode::Preview,
        operation: RenderOperation::Viewport,
        width: 512,
        height: 512,
        format: None,
        features: Vec::new(),
        policy: Default::default(),
    };
    assert_eq!(resolve_request(&[s.render_backend()], &request).unwrap().executor.as_str(), "gpu-viewport");
    request.width = 513;
    assert!(resolve_request(&[s.render_backend()], &request).is_err());
    request.width = 512;
    request.mode = RenderMode::Final;
    assert!(resolve_request(&[s.render_backend()], &request).is_err());
    assert!(host_executor(1024, false, 1).unwrap().availability != Availability::Available);
    assert!(host_executor(1024, true, 0).is_err());
}
