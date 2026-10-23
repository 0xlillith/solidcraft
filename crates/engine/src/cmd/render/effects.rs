//! Read-only discovery for the original effect editors; no device or capture.
use crate::{Result, Session, params::bad};
use serde_json::{Value, json};
use solidcraft_render::job::{
    aging::{AgingKind, AgingLayer},
    looks::{LookKind, LookLayer},
};
fn empty(p: &Value, command: &str) -> Result<()> {
    if !p.as_object().is_some_and(|value| value.is_empty()) {
        return Err(bad(command, "this discovery command accepts an empty object only"));
    }
    Ok(())
}
pub(super) fn aging(_: &mut Session, p: &Value) -> Result<Value> {
    empty(p, "render.aging.describe")?;
    Ok(json!({"maximum_layers":8,"storage":"GPU saved job.options.aging","order":"bottom to top; later layers modify earlier surface results",
        "presets":AgingKind::ALL.map(AgingLayer::preset),"target":"all eligible opaque captured bodies or exact body_index/body_name; passive stages and glass are excluded",
        "coordinate_space":"world millimetres, explicit origin and 4096-cell periodic field; not object-following motion",
        "preserves":["engineering geometry","mass and physical material cards","pose","source textures","visibility"],
        "limitations":["authored optical weathering, not measured corrosion or age prediction","no geometric grooves or edge-curvature detection","no aging on glass or participating-medium boundaries","stack changes transport and requires a new render"],
        "airborne":"use the separately validated finite homogeneous GPU medium; it is not deposited surface dust or a physical lunar atmosphere"}))
}
pub(super) fn presentation(_: &mut Session, p: &Value) -> Result<Value> {
    empty(p, "render.effects.describe")?;
    Ok(json!({"maximum_look_layers":8,"storage":"GPU saved job.options.finishing; retained-master presentation uses the same schema",
        "looks":LookKind::ALL.map(|kind|LookLayer{kind,..Default::default()}),
        "photographic_default":solidcraft_render::job::photographic::Photographic::default(),
        "flare_source":"up to four separated scene-linear high-luminance samples from the actual image; no arbitrary light source is invented",
        "preserves":["raw FLOAT32 EXR bytes","captured geometry and lighting","engineering materials"],
        "limitations":["original artistic transforms, not film-brand LUTs or calibrated camera emulation","image-space flare/distortion/chromatic shifts are not physical lens transport","spatial photographic effects require opaque output and use clamped edges","regrading does not rerun denoising or trace rays"]}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effect_discovery_is_read_only_bounded_and_rejects_mutation_arguments() {
        let mut s = Session::default();
        let document = s.doc.to_json();
        let revision = s.revision;
        for command in ["render.aging.describe", "render.effects.describe"] {
            let reply = s.execute(command, &json!({})).unwrap();
            assert!(reply.is_object());
            assert!(s.execute(command, &json!({"apply":true})).is_err());
            assert_eq!(s.doc.to_json(), document);
            assert_eq!(s.revision, revision);
        }
        assert_eq!(s.execute("render.aging.describe", &json!({})).unwrap()["presets"].as_array().unwrap().len(), 7);
        assert_eq!(s.execute("render.effects.describe", &json!({})).unwrap()["looks"].as_array().unwrap().len(), 8);
    }
}
