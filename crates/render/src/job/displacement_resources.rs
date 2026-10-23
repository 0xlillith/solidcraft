//! Plan real displacement before allocating any micromesh. Input CAD geometry
//! remains immutable and retained by its existing scene snapshot.
use super::{
    CancelToken, ErrorCode, JobError, Result,
    displacement::{self, MAX_DISPLACED_TRIANGLES, MAX_DISPLACEMENT_SCRATCH},
    textures::RuntimeTextures,
};
use crate::ProductScene;
use solidcraft_geom::Aabb3;
use std::sync::Arc;
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct DisplacementPlan {
    pub source_triangles: u64,
    pub rendered_triangles: u64,
    pub rendered_vertices: u64,
    pub generated_mesh_bytes: u64,
    pub scratch_bytes: u64,
    pub displaced_bodies: usize,
}
impl RuntimeTextures {
    pub fn displacement_plan(&self, scene: &ProductScene) -> Result<DisplacementPlan> {
        self.validate_scene(scene)?;
        let mut result = DisplacementPlan {
            source_triangles: 0,
            rendered_triangles: 0,
            rendered_vertices: 0,
            generated_mesh_bytes: 0,
            scratch_bytes: 0,
            displaced_bodies: 0,
        };
        for (ordinal, body) in scene.meshes.iter().enumerate() {
            let source = body.mesh.triangles.len() as u64;
            result.source_triangles += source;
            let mapping = self.bindings().iter().find(|b| b.body_index == ordinal);
            let binding = mapping.and_then(|b| b.displacement.as_ref()).filter(|d| d.controls.amplitude_mm != 0.0);
            if let Some(height) = binding {
                if body.mesh.normals.len() != body.mesh.positions.len() || body.mesh.tri_face.len() != body.mesh.triangles.len() {
                    return Err(JobError::new(ErrorCode::InvalidScene, "height displacement requires exact normals and source face identities"));
                }
                let (triangles, vertices) = height.controls.counts(source)?;
                result.rendered_triangles += triangles;
                result.rendered_vertices += vertices;
                result.generated_mesh_bytes += vertices * 48 + triangles * 16;
                result.scratch_bytes = result.scratch_bytes.max(height.source.decoded_bytes() + 32 * 1024 * 1024);
                result.displaced_bodies += 1;
            } else {
                result.rendered_triangles += source;
                result.rendered_vertices += body.mesh.positions.len() as u64;
            }
            if result.generated_mesh_bytes + result.scratch_bytes > MAX_DISPLACEMENT_SCRATCH
                || (result.displaced_bodies > 0 && result.rendered_triangles > super::MAX_GPU_PATH_TRIANGLES)
            {
                return Err(JobError::new(
                    ErrorCode::LimitExceeded,
                    "complete displacement request exceeds its bounded preparation or triangle budget",
                ));
            }
        }
        let generated = self
            .bindings()
            .iter()
            .filter_map(|b| b.displacement.as_ref().filter(|h| h.controls.amplitude_mm != 0.0).map(|h| (b, h)))
            .try_fold(0_u64, |total, (b, h)| {
                Ok::<_, JobError>(total + h.controls.counts(scene.meshes[b.body_index].mesh.triangles.len() as u64)?.0)
            })?;
        if generated > MAX_DISPLACED_TRIANGLES {
            return Err(JobError::new(ErrorCode::LimitExceeded, "combined displaced surfaces exceed 2 million generated triangles"));
        }
        Ok(result)
    }
    pub fn apply_displacement(&self, scene: &mut ProductScene, cancel: &CancelToken) -> Result<DisplacementPlan> {
        cancel.check()?;
        let plan = self.displacement_plan(scene)?;
        for binding in self.bindings() {
            let Some(height) = binding.displacement.as_ref().filter(|d| d.controls.amplitude_mm != 0.0) else { continue };
            cancel.check()?;
            let body =
                scene.meshes.get_mut(binding.body_index).ok_or_else(|| JobError::new(ErrorCode::InvalidScene, "displacement target disappeared"))?;
            let mesh = displacement::mesh(&body.mesh, binding.frame, height, binding.flip_v, cancel)?;
            body.mesh = Arc::new(mesh);
        }
        scene.bounds = scene.meshes.iter().fold(Aabb3::EMPTY, |a, b| a.union(&b.mesh.bounds()));
        cancel.check()?;
        Ok(plan)
    }
}
