//! Integrated product preview. Shares the installed device/pipelines, not Design's scene policy.

use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use egui::{Color32, Sense};
use serde_json::json;
use solidcraft_engine::doc::render_settings::RenderSetup;
use solidcraft_engine::render::{
    Availability, Camera, ExecutorDescriptor, ExecutorId, ProductScene, RenderLimits, RenderMode, RenderOperation, RenderProfile, RenderRequest,
    ScratchLayout,
};

use crate::gpu::{GpuScene, SceneSlot, ViewportCallback};
use crate::{SolveApp, theme::Tokens};

const MAX_GEOMETRY_BYTES: usize = 128 * 1024 * 1024;

#[cfg(test)]
#[path = "render_workspace_tests.rs"]
mod tests;

pub struct State {
    pub host: Option<ExecutorDescriptor>,
    pub view: ViewState,
    revision: u64,
    design_camera: Option<(Camera, bool)>,
    slot: SceneSlot,
    key: Option<u64>,
    scene_signature: Option<(u64, usize, u64)>,
    scene: Option<ProductScene>,
}

/// Small per-document editor state; GPU uploads and host capabilities stay outside tabs.
#[derive(Clone, Debug)]
pub struct ViewState {
    pub name: String,
    draft: RenderSetup,
    dirty: bool,
}

impl Default for ViewState {
    fn default() -> Self {
        Self { name: "Studio".into(), draft: RenderSetup::default(), dirty: false }
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            host: None,
            view: Default::default(),
            revision: u64::MAX,
            design_camera: None,
            slot: Arc::new(Mutex::new(None)),
            key: None,
            scene_signature: None,
            scene: None,
        }
    }
}

pub fn active(app: &SolveApp) -> bool {
    app.ui.tab == "RENDER"
}

pub fn host_executor(max_dimension: u32, depth: bool, samples: u32) -> Result<ExecutorDescriptor, String> {
    if !matches!(samples, 1 | 2 | 4 | 8 | 16) {
        return Err("unsupported GPU sample count".into());
    }
    Ok(ExecutorDescriptor {
        id: ExecutorId::new("gpu-viewport").map_err(|e| e.to_string())?,
        availability: if depth && max_dimension > 0 {
            Availability::Available
        } else {
            Availability::Unavailable { reason: "product preview requires an installed device with a depth buffer".into() }
        },
        profiles: vec![RenderProfile { mode: RenderMode::Preview, operation: RenderOperation::Viewport, formats: Vec::new(), features: Vec::new() }],
        limits: RenderLimits { max_dimension: u64::from(max_dimension.clamp(1, 4096)), ..Default::default() },
        scratch: ScratchLayout {
            sample_scale: 1,
            sample_bytes_per_pixel: 8 * u64::from(samples),
            output_bytes_per_pixel: 8,
            encoding_bytes_per_pixel: 0,
        },
    })
}

pub fn sync_backend(app: &mut SolveApp) {
    if app.session.render_viewport() == app.render_workspace.host.as_ref() {
        return;
    }
    if let Err(e) = app.session.set_render_viewport(app.render_workspace.host.clone()) {
        app.set_status(e.to_string(), true);
    }
}

pub fn reset_document(app: &mut SolveApp) {
    let host = app.render_workspace.host.clone();
    app.render_workspace = State { host, ..Default::default() };
    if active(app) {
        app.ui.tab = "SOLID".into();
    }
    sync_backend(app);
}

pub fn invalidate_scene(app: &mut SolveApp) {
    app.render_workspace.revision = u64::MAX;
    app.render_workspace.scene_signature = None;
    app.render_workspace.scene = None;
    app.render_workspace.key = None;
}

pub fn enter(app: &mut SolveApp, render: bool) -> Result<(), String> {
    if render {
        if app.session.active_sketch.is_some() || app.tool.is_some() || app.dialog.is_some() || app.file_load.busy() {
            return Err("finish the current modelling operation before entering Render".into());
        }
        app.run("render.scene", json!({}))?;
        if !active(app) {
            app.render_workspace.design_camera = Some((app.cam, app.ui.perspective));
            app.ui.tab = "RENDER".into();
            let scene = solidcraft_engine::product_scene::extract(&app.session).map_err(|e| e.to_string())?;
            if !scene.bounds.is_empty() {
                app.cam.fit(&scene.bounds);
            }
        }
    } else {
        if let Some((camera, perspective)) = app.render_workspace.design_camera.take() {
            app.cam = camera;
            app.ui.up_axis = camera.up;
            app.ui.perspective = perspective;
        }
        app.ui.tab = "SOLID".into();
    }
    app.cancel_view_animation();
    Ok(())
}

pub fn panel(app: &mut SolveApp, ui: &mut egui::Ui) {
    if app.render_workspace.revision != app.session.revision && !app.render_workspace.view.dirty {
        app.render_workspace.view.draft = app.session.doc.render.setups.get(&app.render_workspace.view.name).cloned().unwrap_or_default();
        app.render_workspace.revision = app.session.revision;
    }
    egui::Panel::right("sc_render_settings")
        .exact_size(285.0)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(Tokens::get().panel).inner_margin(10))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(app.tr("cmd.render_settings_get"));
                ui.label(app.tr("render.preview"));
                ui.small(app.tr("render.preview_help"));
                ui.separator();
                let names: Vec<_> = app.session.doc.render.setups.keys().cloned().collect();
                let old_name = app.render_workspace.view.name.clone();
                egui::ComboBox::from_id_salt("render_setups").selected_text(&old_name).show_ui(ui, |ui| {
                    for name in names {
                        ui.selectable_value(&mut app.render_workspace.view.name, name.clone(), name);
                    }
                });
                ui.label(app.tr("render.setup_name"));
                ui.add(egui::TextEdit::singleline(&mut app.render_workspace.view.name).char_limit(128));
                if app.render_workspace.view.name != old_name {
                    app.render_workspace.view.draft = app.session.doc.render.setups.get(&app.render_workspace.view.name).cloned().unwrap_or_default();
                    app.render_workspace.view.dirty = false;
                }
                ui.separator();
                ui.label(app.tr("render.background"));
                if ui.color_edit_button_srgb(&mut app.render_workspace.view.draft.background).changed() {
                    app.render_workspace.view.dirty = true;
                }
                let mut perspective = app.cam.fov > 0.0;
                if ui.checkbox(&mut perspective, app.tr("render.perspective")).changed() {
                    app.cam.fov = if perspective { std::f64::consts::FRAC_PI_4 } else { 0.0 };
                    app.render_workspace.view.dirty = true;
                }
                if ui.button(app.tr("render.fit")).clicked() {
                    match solidcraft_engine::product_scene::extract(&app.session) {
                        Ok(scene) if !scene.bounds.is_empty() => app.cam.fit(&scene.bounds),
                        Err(e) => app.set_status(e.to_string(), true),
                        _ => {}
                    }
                }
                if ui.button(app.tr("render.restore_camera")).clicked() {
                    let name = app.render_workspace.view.name.clone();
                    if app.run("render.settings.apply", json!({"name":name})).is_ok() {
                        app.follow_view_request(false);
                    }
                }
                if ui.button(app.tr("render.save_setup")).clicked() {
                    let name = app.render_workspace.view.name.clone();
                    let setup = RenderSetup { camera: app.cam, ..app.render_workspace.view.draft.clone() };
                    if app.run("render.settings.set", json!({"name":name,"set":setup})).is_ok() {
                        app.render_workspace.view.dirty = false;
                    }
                }
                if app.render_workspace.view.dirty {
                    ui.label(app.tr("render.unsaved"));
                }
                if ui.button(app.tr("render.reload_settings")).clicked() {
                    app.render_workspace.view.dirty = false;
                    app.render_workspace.revision = u64::MAX;
                }
                if app.render_workspace.view.draft.backend.as_str() != "builtin" || app.render_workspace.view.draft.mode != RenderMode::Preview {
                    ui.label(app.tr("render.unavailable_preview"));
                    if ui.button(app.tr("render.use_preview")).clicked() {
                        app.render_workspace.view.draft.backend = solidcraft_engine::render::builtin_backend().id;
                        app.render_workspace.view.draft.mode = RenderMode::Preview;
                        app.render_workspace.view.dirty = true;
                    }
                }
                ui.separator();
                ui.small(app.tr("render.navigation_help"));
                if ui.button(app.tr("render.return_design")).clicked()
                    && let Err(e) = enter(app, false)
                {
                    app.set_status(e, true);
                }
            });
        });
}

/// Convert only product surfaces; geometry budgets are admitted before vertex allocation.
pub fn gpu_scene(scene: &ProductScene) -> Result<(u64, GpuScene), String> {
    let bytes = scene
        .meshes
        .iter()
        .try_fold(0usize, |n, m| m.mesh.triangles.len().checked_mul(3 * crate::gpu::TRI_SIZE).and_then(|x| n.checked_add(x)))
        .ok_or("product geometry size overflow")?;
    if bytes > MAX_GEOMETRY_BYTES {
        return Err("product preview exceeds the 128 MiB geometry upload budget".into());
    }
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    let mut gpu = GpuScene::default();
    for body in &scene.meshes {
        if !body.opacity.is_finite()
            || !(0.0..=1.0).contains(&body.opacity)
            || body.faces.iter().any(|f| !f.opacity.is_finite() || !(0.0..=1.0).contains(&f.opacity))
        {
            return Err("invalid product appearance opacity".into());
        }
        (Arc::as_ptr(&body.mesh) as usize).hash(&mut hash);
        body.name.hash(&mut hash);
        body.color.hash(&mut hash);
        body.opacity.to_bits().hash(&mut hash);
        let looks: std::collections::BTreeMap<_, _> = body.faces.iter().map(|l| (l.face, l)).collect();
        for face in &body.faces {
            face.face.hash(&mut hash);
            face.color.hash(&mut hash);
            face.opacity.to_bits().hash(&mut hash);
        }
        for (index, triangle) in body.mesh.triangles.iter().enumerate() {
            let face = body.mesh.tri_face.get(index).and_then(|f| looks.get(f));
            let (color, opacity) = face.map_or((body.color, body.opacity), |l| (l.color, l.opacity));
            if opacity <= 0.0 {
                continue;
            }
            let vertices = triangle.map(|i| {
                let p = body.mesh.positions.get(i as usize)?.to_f32();
                let n = body.mesh.normals.get(i as usize)?.to_f32();
                p.into_iter().chain(n).all(f32::is_finite).then_some((p, n))
            });
            let [Some(a), Some(b), Some(c)] = vertices else { return Err("invalid product triangle".into()) };
            let rgba = [color[0], color[1], color[2], (opacity * 255.0).round() as u8];
            for (p, n) in [a, b, c] {
                if opacity < 0.999 {
                    gpu.glass_tri(p, n, rgba);
                } else {
                    gpu.tri(p, n, rgba);
                }
            }
        }
    }
    Ok((hash.finish(), gpu))
}

pub fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    app.viewport.rect = Some(rect);
    let response = ui.allocate_rect(rect, Sense::click_and_drag());
    let delta = ui.input(|i| i.pointer.delta());
    if response.dragged_by(egui::PointerButton::Primary) {
        app.cam.orbit(f64::from(delta.x), f64::from(delta.y));
    }
    if response.dragged_by(egui::PointerButton::Middle) {
        app.cam.pan(f64::from(delta.x), f64::from(delta.y), f64::from(rect.height()));
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            app.cam.zoom_at((-f64::from(scroll) * 0.0015 * crate::prefs::zoom_sign(app)).exp(), None);
        }
    }
    let bg = app.render_workspace.view.draft.background;
    ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(bg[0], bg[1], bg[2]));
    let result = (|| -> Result<(), String> {
        if app.viewport.gpu.is_none() {
            return Err("GPU product preview is unavailable on this host. Saved setups remain available.".into());
        }
        let ppp = ui.ctx().pixels_per_point();
        let setup = &app.render_workspace.view.draft;
        let request = RenderRequest {
            backend: Some(setup.backend.clone()),
            mode: setup.mode,
            operation: RenderOperation::Viewport,
            width: (rect.width() * ppp).max(1.0) as u64,
            height: (rect.height() * ppp).max(1.0) as u64,
            format: None,
            features: Vec::new(),
            policy: Default::default(),
        };
        solidcraft_engine::render::resolve_request(&[app.session.render_backend()], &request).map_err(|e| e.to_string())?;
        if app.session.model.is_stale() {
            return Err("finish the modelling gesture before previewing".into());
        }
        let mut visibility = std::collections::hash_map::DefaultHasher::new();
        app.session.visibility.hidden_bodies.hash(&mut visibility);
        app.session.visibility.hidden_occurrences.hash(&mut visibility);
        let signature = (app.session.revision, Arc::as_ptr(&app.session.world_state()) as usize, visibility.finish());
        if app.render_workspace.scene_signature != Some(signature) {
            let scene = solidcraft_engine::product_scene::extract(&app.session).map_err(|e| e.to_string())?;
            let (key, gpu) = gpu_scene(&scene)?;
            if app.render_workspace.key != Some(key) {
                if let Ok(mut slot) = app.render_workspace.slot.lock() {
                    *slot = Some(gpu);
                }
                app.render_workspace.key = Some(key);
            }
            app.render_workspace.scene = Some(scene);
            app.render_workspace.scene_signature = Some(signature);
        }
        let scene = app.render_workspace.scene.as_ref().ok_or("no product scene")?;
        let key = app.render_workspace.key.ok_or("no product geometry")?;
        if scene.bounds.is_empty() {
            return Err("Create a visible solid in Design to preview it here.".into());
        }
        let camera = app.cam;
        let radius = scene.bounds.diagonal().max(10.0) + scene.bounds.center().dist(camera.target);
        let eye = camera.eye();
        let slot = app.render_workspace.slot.clone();
        let callback = ViewportCallback {
            product: true,
            key,
            slot: slot.clone(),
            sketch_key: 0,
            sketch_slot: slot.clone(),
            hl_key: 0,
            hl_slot: slot.clone(),
            pv_key: 0,
            pv_slot: slot,
            view_proj: camera.view_proj(f64::from(rect.width() / rect.height().max(1.0)), radius).to_f32(),
            back: camera.back().to_f32(),
            size_px: [rect.width() * ppp, rect.height() * ppp],
            clip: [0.0; 4],
            cap: [0.0; 4],
            images: Vec::new(),
            analysis: [0.0; 8],
            access: None,
            outline: false,
            eye: [eye.x as f32, eye.y as f32, eye.z as f32, if camera.fov > 0.0 { 1.0 } else { 0.0 }],
        };
        ui.painter().add(egui_wgpu::Callback::new_paint_callback(rect, callback));
        Ok(())
    })();
    if let Err(error) = result {
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, error, egui::FontId::proportional(14.0), Tokens::get().text);
    }
}
