//! Renderer-independent product geometry. No modelling overlays or device resources.

use std::sync::Arc;

use solidcraft_geom::{Aabb3, Mesh};

#[derive(Clone, Debug, PartialEq)]
pub struct FaceAppearance {
    /// The existing mesh's B-rep face index, resolved by the document's appearance policy.
    pub face: u32,
    pub color: [u8; 3],
    pub opacity: f64,
}

#[derive(Clone, Debug)]
pub struct ProductMesh {
    /// Existing computed body name, including occurrence identity where applicable.
    pub name: String,
    /// Established world-space mesh. Adapters must not apply occurrence transforms again.
    pub mesh: Arc<Mesh>,
    pub color: [u8; 3],
    pub opacity: f64,
    pub faces: Vec<FaceAppearance>,
}

#[derive(Clone, Debug)]
pub struct ProductScene {
    pub meshes: Vec<ProductMesh>,
    pub bounds: Aabb3,
}
