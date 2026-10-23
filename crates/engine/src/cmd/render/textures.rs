//! Bounded immutable file capture; decoding is deferred to the admitted worker.
use crate::{Result, params::bad};
use serde::Deserialize;
use solidcraft_render::{
    ProductScene,
    job::textures::{BaseColorMode, MAX_TEXTURE_BINDINGS, MAX_TEXTURE_SOURCE_BYTES, PlanarFrame, RuntimeTextures, TextureBinding, TextureMap},
    texture_input::MapRole,
};
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Override {
    #[serde(default)]
    height_linear: Option<String>,
    #[serde(default)]
    displacement: Option<solidcraft_render::job::displacement::Displacement>,
    body_index: usize,
    body_name: String,
    frame: PlanarFrame,
    #[serde(default)]
    base_color_srgb: Option<String>,
    #[serde(default)]
    normal_opengl: Option<String>,
    #[serde(default)]
    roughness_linear: Option<String>,
    #[serde(default)]
    metallic_linear: Option<String>,
    #[serde(default)]
    base_color_mode: BaseColorMode,
    #[serde(default)]
    flip_v: bool,
    #[serde(default = "normal_strength")]
    normal_strength: f64,
}
fn normal_strength() -> f64 {
    1.0
}
fn load(
    path: &str,
    role: MapRole,
    remaining: &mut u64,
    command: &str,
    captured: &mut std::collections::BTreeMap<(String, u8), TextureMap>,
) -> Result<TextureMap> {
    if path.is_empty() || path.len() > 4096 || path.chars().any(char::is_control) {
        return Err(bad(command, "texture path must be a bounded local path"));
    }
    let role_key = match role {
        MapRole::BaseColor => 0,
        MapRole::NormalOpenGl => 1,
        MapRole::Roughness => 2,
        MapRole::Metallic => 3,
        MapRole::AmbientOcclusion => 4,
    };
    let key = (path.to_string(), role_key);
    if let Some(map) = captured.get(&key) {
        return Ok(map.clone());
    }
    let maximum = (*remaining).min(solidcraft_render::texture_input::MAX_TEXTURE_BYTES as u64);
    let bytes = if let Some(key) = path.strip_prefix("builtin://") {
        super::library::texture_bytes(key, role, maximum, command)?
    } else {
        solidcraft_io::vfs::read(path, maximum as usize).map_err(|e| bad(command, format!("could not capture texture: {e}")))?
    };
    if bytes.is_empty() {
        return Err(bad(command, "texture file must not be empty"));
    }
    let map = TextureMap::new(bytes, role).map_err(|e| bad(command, e.to_string()))?;
    *remaining = remaining.checked_sub(map.retained_bytes() as u64).ok_or_else(|| bad(command, "encoded texture capacity exceeds budget"))?;
    captured.insert(key, map.clone());
    Ok(map)
}
pub(super) fn validate_targets(scene: &ProductScene, overrides: &[Override], command: &str) -> Result<()> {
    if overrides.len() > MAX_TEXTURE_BINDINGS {
        return Err(bad(command, "at most 128 runtime texture bindings"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for input in overrides {
        input.frame.validate().map_err(|e| bad(command, e.to_string()))?;
        if input.height_linear.is_some() != input.displacement.is_some() {
            return Err(bad(command, "height source and displacement controls must be supplied together"));
        }
        if let Some(height) = input.displacement {
            height.validate().map_err(|e| bad(command, e.to_string()))?;
            if input.normal_opengl.is_some() && input.normal_strength != 0.0 && height.amplitude_mm != 0.0 {
                return Err(bad(command, "disable matching normal-map detail explicitly before geometric displacement"));
            }
        }
        if !seen.insert(input.body_index) {
            return Err(bad(command, "duplicate texture target"));
        }
        let body = scene
            .meshes
            .get(input.body_index)
            .filter(|b| b.name == input.body_name)
            .ok_or_else(|| bad(command, "texture ordinal and name must both match render.scene"))?;
        if body.transmission.is_some() || body.opacity != 1.0 || body.faces.iter().any(|f| f.transmission.is_some() || f.opacity != 1.0) {
            return Err(bad(command, "runtime maps support opaque bodies only"));
        }
        let bounds = body.mesh.bounds();
        for x in [bounds.min.x, bounds.max.x] {
            for y in [bounds.min.y, bounds.max.y] {
                for z in [bounds.min.z, bounds.max.z] {
                    input.frame.uv(solidcraft_geom::Vec3::new(x, y, z)).map_err(|e| bad(command, e.to_string()))?;
                }
            }
        }
    }
    Ok(())
}
pub(super) fn capture(scene: &ProductScene, overrides: &[Override], command: &str) -> Result<Option<Arc<RuntimeTextures>>> {
    if overrides.is_empty() {
        return Ok(None);
    }
    if overrides.len() > MAX_TEXTURE_BINDINGS {
        return Err(bad(command, "at most 128 runtime texture bodies are supported"));
    }
    validate_targets(scene, overrides, command)?;
    let mut remaining = MAX_TEXTURE_SOURCE_BYTES;
    let mut captured = std::collections::BTreeMap::new();
    let mut heights = std::collections::BTreeMap::<String, solidcraft_render::job::displacement::HeightSource>::new();
    let mut bindings = Vec::new();
    bindings.try_reserve_exact(overrides.len()).map_err(|_| bad(command, "could not reserve texture bindings"))?;
    for input in overrides {
        input.frame.validate().map_err(|e| bad(command, e.to_string()))?;
        // Refuse stale identity or unsupported optics before touching any file.
        let body = scene
            .meshes
            .get(input.body_index)
            .filter(|b| b.name == input.body_name)
            .ok_or_else(|| bad(command, "texture ordinal and name must both match render.scene"))?;
        if body.transmission.is_some() || body.opacity != 1.0 || body.faces.iter().any(|f| f.transmission.is_some() || f.opacity != 1.0) {
            return Err(bad(command, "runtime maps support opaque bodies only"));
        }
        let base_color = input.base_color_srgb.as_ref().map(|p| load(p, MapRole::BaseColor, &mut remaining, command, &mut captured)).transpose()?;
        let normal = input.normal_opengl.as_ref().map(|p| load(p, MapRole::NormalOpenGl, &mut remaining, command, &mut captured)).transpose()?;
        let roughness = input.roughness_linear.as_ref().map(|p| load(p, MapRole::Roughness, &mut remaining, command, &mut captured)).transpose()?;
        let metallic = input.metallic_linear.as_ref().map(|p| load(p, MapRole::Metallic, &mut remaining, command, &mut captured)).transpose()?;
        let displacement = match (&input.height_linear, input.displacement) {
            (None, None) => None,
            (Some(path), Some(controls)) => {
                let source = if let Some(source) = heights.get(path) {
                    source.clone()
                } else {
                    if path.is_empty() || path.len() > 4096 || path.chars().any(char::is_control) {
                        return Err(bad(command, "height source path must be bounded and explicit"));
                    }
                    let maximum = remaining.min(16 * 1024 * 1024);
                    let bytes = if let Some(key) = path.strip_prefix("builtin://") {
                        let id = key.strip_suffix("/height").ok_or_else(|| bad(command, "expected builtin://SURF-nnn/height"))?;
                        super::library::height_bytes(id, maximum, command)?
                    } else {
                        solidcraft_io::vfs::read(path, maximum as usize).map_err(|e| bad(command, e.to_string()))?
                    };
                    let source = solidcraft_render::job::displacement::HeightSource::new(bytes).map_err(|e| bad(command, e.to_string()))?;
                    remaining =
                        remaining.checked_sub(source.retained_bytes()).ok_or_else(|| bad(command, "encoded height exceeds texture source budget"))?;
                    heights.insert(path.clone(), source.clone());
                    source
                };
                Some(solidcraft_render::job::displacement::HeightDisplacement { source, controls })
            }
            _ => return Err(bad(command, "height source and controls must be supplied together")),
        };
        bindings.push(TextureBinding {
            displacement,
            body_index: input.body_index,
            body_name: input.body_name.clone(),
            frame: input.frame,
            normal_strength: input.normal_strength,
            base_color,
            normal,
            roughness,
            metallic,
            base_color_mode: input.base_color_mode,
            flip_v: input.flip_v,
        });
    }
    let resources = RuntimeTextures::new(bindings).map_err(|e| bad(command, e.to_string()))?;
    resources.validate_scene(scene).map_err(|e| bad(command, e.to_string()))?;
    Ok(Some(Arc::new(resources)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use solidcraft_geom::{Aabb3, Mesh};
    #[test]
    fn unsupported_or_stale_map_targets_fail_before_file_io() {
        let scene = ProductScene {
            bounds: Aabb3::EMPTY,
            meshes: vec![solidcraft_render::ProductMesh {
                name: "Glass".into(),
                mesh: Arc::new(Mesh::default()),
                color: [255; 3],
                opacity: 1.,
                surface: None,
                transmission: Some(solidcraft_render::transmission::Transmission::new(solidcraft_geom::Vec3::ZERO, Some(1.)).unwrap()),
                faces: vec![],
            }],
        };
        let value =
            json!({"body_index":0,"body_name":"Glass","frame":{"origin":[0,0,0],"u":[1,0,0],"v":[0,1,0]},"base_color_srgb":"nonexistent-image.png"});
        let input: Override = serde_json::from_value(value.clone()).unwrap();
        assert!(capture(&scene, &[input], "render.start").unwrap_err().to_string().contains("opaque bodies"));
        let mut stale = value.clone();
        stale["body_name"] = json!("Stale");
        let input: Override = serde_json::from_value(stale).unwrap();
        assert!(capture(&scene, &[input], "render.start").unwrap_err().to_string().contains("ordinal and name"));
        let mut unknown = value;
        unknown["metallic_map"] = json!("foo");
        assert!(serde_json::from_value::<Override>(unknown).is_err());
    }
}
