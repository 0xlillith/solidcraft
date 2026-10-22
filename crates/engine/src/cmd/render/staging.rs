//! Explicit saved render-only ground. Does not add engineering bodies or material cards.
use crate::{Result, Session, params::bad};
use serde::Deserialize;
use serde_json::{Value, json};
use solidcraft_doc::render_settings::{RenderJob, RenderSetup};
use solidcraft_render::{
    ProductScene,
    job::CancelToken,
    stage::{Stage, StagePanel},
};
use std::sync::Arc;
fn patch() -> Value {
    json!({})
}
fn margin() -> f64 {
    0.25
}
fn gap() -> f64 {
    0.1
}
fn reflectance() -> [f64; 3] {
    [0.18; 3]
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ground {
    name: String,
    #[serde(default = "patch")]
    set: Value,
    #[serde(default = "margin")]
    margin_fraction: f64,
    #[serde(default = "gap")]
    gap_mm: f64,
    #[serde(default = "reflectance")]
    reflectance_linear: [f64; 3],
    #[serde(default)]
    replace_stage: bool,
}
pub(super) fn apply(scene: &mut ProductScene, setup: &RenderSetup, command: &str) -> Result<()> {
    setup.stage.append_to(scene, &CancelToken::default()).map_err(|e| bad(command, e.to_string()))
}
fn save(s: &mut Session, name: String, setup: RenderSetup, command: &str) -> Result<Value> {
    let mut settings = (*s.doc.render).clone();
    settings.setups.insert(name.clone(), setup.clone());
    settings.validate().map_err(|e| bad(command, e.to_string()))?;
    if settings != *s.doc.render {
        s.doc_mut().render = Arc::new(settings);
    }
    Ok(json!({"name":name,"setup":setup,"engineering_geometry_unchanged":true,"stage_only":true,
        "clearance":"conservative product AABB separation; not a B-rep collision proof"}))
}
pub(super) fn ground(s: &mut Session, params: &Value) -> Result<Value> {
    let command = "render.stage.ground";
    let input: Ground = serde_json::from_value(params.clone()).map_err(|e| bad(command, e.to_string()))?;
    if !input.margin_fraction.is_finite()
        || !(0.0..=10.0).contains(&input.margin_fraction)
        || !input.gap_mm.is_finite()
        || !(1e-4..=1000.0).contains(&input.gap_mm)
    {
        return Err(bad(command, "ground margin must be 0..10 and explicit clearance gap 0.0001..1000 mm"));
    }
    let mut setup = super::patched_settings(s, &input.name, &input.set, command)?;
    if !matches!(setup.job, Some(RenderJob::CpuRaytrace { .. } | RenderJob::GpuPath { .. })) {
        return Err(bad(command, "ground requires an explicit CPU raytrace or GPU path setup"));
    }
    if !setup.stage.is_empty() && !input.replace_stage {
        return Err(bad(command, "existing stage requires replace_stage: true"));
    }
    let mut scene = crate::product_scene::extract(s)?;
    if scene.bounds.is_empty() {
        return Err(bad(command, "ground fit requires a nonempty visible product"));
    }
    let bounds = scene.bounds;
    let size = bounds.max - bounds.min;
    let center = bounds.center();
    let extent = size.x.max(size.y).max(size.z).max(0.001);
    let u = (size.x * 0.5).max(0.001) + extent * input.margin_fraction;
    let v = (size.y * 0.5).max(0.001) + extent * input.margin_fraction;
    setup.stage = Stage::new(vec![StagePanel {
        id: "ground".into(),
        center_mm: [center.x, center.y, bounds.min.z - input.gap_mm],
        half_u_mm: [u, 0., 0.],
        half_v_mm: [0., v, 0.],
        reflectance_linear: input.reflectance_linear,
    }])
    .map_err(|e| bad(command, e.to_string()))?;
    apply(&mut scene, &setup, command)?;
    save(s, input.name, setup, command)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Clear {
    name: String,
    #[serde(default = "patch")]
    set: Value,
}
pub(super) fn clear(s: &mut Session, params: &Value) -> Result<Value> {
    let command = "render.stage.clear";
    let input: Clear = serde_json::from_value(params.clone()).map_err(|e| bad(command, e.to_string()))?;
    let mut setup = super::patched_settings(s, &input.name, &input.set, command)?;
    setup.stage = Stage::default();
    save(s, input.name, setup, command)
}
