//! SolidCraft native, versioned binary geometry. No STEP conversion or display meshes.
//! A cache remains provisional until the document is rebuilt; hashes are not trust.
use crate::{Body, KernelError, Result, guard};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use std::sync::Arc;

mod adapter;
mod binary;
mod preview;
pub use preview::native_preview_mesh;

const MAGIC: &[u8; 8] = b"SCBREP02";
const MAX_BYTES: usize = 64 << 20;
const MAX_ITEMS: usize = 100_000;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedBody {
    solid: solidcraft_geom::brep::Brep,
    color: Option<[f32; 3]>,
    paint: Option<crate::Paint>,
    splits: Option<Vec<Vec<solidcraft_geom::Vec3>>>,
    face_sources: Option<Vec<Option<crate::FaceSource>>>,
    history: Option<crate::history::History>,
    regularization_protected: bool,
}
fn invalid(detail: impl std::fmt::Display) -> KernelError {
    KernelError::Invalid(format!("native B-rep: {detail}"))
}

/// Exact topology, native curves/surfaces, construction certificates and history.
/// Failure is a cache miss at the IO boundary, never a failed document save.
pub fn encode_native_body(body: &Body) -> Result<Vec<u8>> {
    guard("encode native B-rep", || {
        for source in body.face_sources.iter().flat_map(|s| s.iter()).flatten() {
            let mut at = source;
            let mut depth = 0usize;
            while let crate::FaceSource::ShellPlanar { source, .. } = at {
                depth += 1;
                if depth > 32 {
                    return Err(invalid("face-source nesting"));
                }
                at = source;
            }
        }
        if body.face_count() > MAX_ITEMS
            || body.solid.edge_iter().take(MAX_ITEMS + 1).count() > MAX_ITEMS
            || body.mesh.as_ref().is_some_and(|m| m.positions.len() > MAX_ITEMS || m.triangles.len() > MAX_ITEMS)
        {
            return Err(invalid("native geometry budget"));
        }
        if body.mesh.is_some() {
            return Err(invalid("mesh bodies are source blobs, not B-rep"));
        }
        let saved = SavedBody {
            solid: adapter::encode(&body.solid)?,
            color: body.color,
            paint: body.paint.as_deref().cloned(),
            splits: body.splits.as_deref().cloned(),
            face_sources: body.face_sources.as_deref().cloned(),
            history: body.history.as_deref().cloned(),
            regularization_protected: body.regularization_protected,
        };
        // Saving already evaluated geometry must not re-run intersection projections.
        validate_topology(&saved)?;
        let value = serde_json::to_value(saved).map_err(invalid)?;
        validate_value(&value, 0)?;
        if value_nodes(&value) > 2_000_000 {
            return Err(invalid("value budget"));
        }
        let mut out = MAGIC.to_vec();
        binary::write(&value, &mut out, 0)?;
        Ok(out)
    })
}

/// Decode bounded untrusted data. Validate indices before topology extraction.
/// Callers must not treat the decoded body as authoritative editing geometry.
pub fn decode_native_body(bytes: &[u8]) -> Result<Body> {
    if bytes.len() > MAX_BYTES || bytes.get(..8) != Some(MAGIC.as_slice()) {
        return Err(invalid("version or size"));
    }
    guard("decode native B-rep", || {
        let mut input = bytes.get(8..).ok_or_else(|| invalid("header"))?;
        let mut remaining = 2_000_000usize;
        let value = binary::read(&mut input, 0, &mut remaining)?;
        if !input.is_empty() {
            return Err(invalid("trailing data"));
        }
        validate_value(&value, 0)?;
        let saved: SavedBody = serde_json::from_value(value).map_err(invalid)?;
        validate_topology(&saved)?;
        let solid = adapter::decode(saved.solid)?;
        Ok(Body {
            solid: Arc::new(solid),
            coarse_missing: None,
            mesh: None,
            color: saved.color,
            paint: saved.paint.map(Arc::new),
            splits: saved.splits.map(Arc::new),
            face_sources: saved.face_sources.map(Arc::new),
            history: saved.history.map(Arc::new),
            regularization_protected: saved.regularization_protected,
        })
    })
}

fn validate_topology(saved: &SavedBody) -> Result<()> {
    saved.solid.validate().map_err(invalid)?;
    let faces: usize = saved.solid.shells.iter().map(|s| s.faces.len()).sum();
    let edges: usize = saved.solid.shells.iter().map(|s| s.edges.len()).sum();
    if saved.face_sources.as_ref().is_some_and(|s| s.len() != faces) {
        return Err(invalid("face source count"));
    }
    if let Some(history) = &saved.history
        && (history.faces.len() != faces
            || history.edges.len() != edges
            || history.operands > MAX_ITEMS
            || history.edges.iter().flat_map(|e| &e.faces).any(|&f| f >= faces))
    {
        return Err(invalid("history topology"));
    }
    if saved.color.is_some_and(|c| c.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))) {
        return Err(invalid("colour"));
    }
    if let Some(paint) = &saved.paint
        && (!paint.opacity.is_finite()
            || !(0.0..=1.0).contains(&paint.opacity)
            || paint.faces.iter().any(|p| {
                p.face >= faces
                    || !p.opacity.is_finite()
                    || !(0.0..=1.0).contains(&p.opacity)
                    || p.color.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            }))
    {
        return Err(invalid("face paint"));
    }
    Ok(())
}

// The binary reader bounds allocation before deserialization. Geometry semantics
// are checked by the independent schema validator, not by backend serde internals.
fn validate_value(v: &Value, depth: usize) -> Result<()> {
    if depth > 48 {
        return Err(invalid("nesting"));
    }
    match v {
        Value::String(s) if s.len() > 1 << 20 => return Err(invalid("string budget")),
        Value::Number(n) if n.is_f64() && n.as_f64().is_none_or(|x| !x.is_finite() || x.abs() > 1e12) => return Err(invalid("non-finite metadata")),
        Value::Array(a) => {
            if a.len() > MAX_ITEMS {
                return Err(invalid("array budget"));
            }
            for x in a {
                validate_value(x, depth + 1)?;
            }
        }
        Value::Object(o) => {
            for x in o.values() {
                validate_value(x, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn value_nodes(v: &Value) -> usize {
    match v {
        Value::Array(a) => a.iter().fold(1usize, |n, v| n.saturating_add(value_nodes(v))),
        Value::Object(o) => o.values().fold(1usize, |n, v| n.saturating_add(value_nodes(v))),
        _ => 1,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use solidcraft_geom::Vec3;
    fn equal(a: &Body, b: &Body) {
        assert_eq!(a.face_count(), b.face_count());
        assert_eq!(a.face_sources, b.face_sources);
        assert_eq!(a.history, b.history);
        assert_eq!(a.paint, b.paint);
        assert_eq!(a.color, b.color);
        assert_eq!(a.splits, b.splits);
        assert_eq!(a.regularization_protected, b.regularization_protected);
        assert_eq!(encode_native_body(a).unwrap(), encode_native_body(b).unwrap());
        let am = crate::measure(a).unwrap();
        let bm = crate::measure(b).unwrap();
        assert!((am.volume - bm.volume).abs() < 1e-7);
    }
    #[test]
    fn native_solids_keep_geometry_metadata_and_later_modelling() {
        let cube = crate::box_solid(Vec3::ZERO, Vec3::new(40., 30., 10.)).unwrap();
        let rounded = crate::fillet(&cube, &[Vec3::new(20., 0., 10.)], 2.).unwrap();
        let bore = crate::cylinder(Vec3::new(10., 10., -1.), Vec3::Z, 3., 12.).unwrap();
        let cut = crate::boolean(&rounded, &bore, crate::BoolOp::Cut).unwrap().unwrap();
        for body in [cube, rounded, cut] {
            let body = body.with_color(Some([0.1, 0.5, 0.9]));
            let bytes = encode_native_body(&body).unwrap();
            assert!(bytes.starts_with(MAGIC));
            let decoded = decode_native_body(&bytes).unwrap();
            equal(&body, &decoded);
            let a = crate::boolean(&body, &bore, crate::BoolOp::Cut).unwrap().unwrap();
            let b = crate::boolean(&decoded, &bore, crate::BoolOp::Cut).unwrap().unwrap();
            assert!((crate::measure(&a).unwrap().volume - crate::measure(&b).unwrap().volume).abs() < 1e-7);
        }
    }
    #[test]
    fn native_spheres_tori_and_open_faces_round_trip_without_conversion() {
        let cube = crate::box_solid(Vec3::ZERO, Vec3::new(40., 30., 10.)).unwrap();
        let surface = crate::copy_faces(&cube, &[Vec3::new(20., 15., 10.)]).unwrap();
        let sphere = crate::sphere(Vec3::ZERO, 12.).unwrap();
        let torus = crate::torus(Vec3::ZERO, 20., 4.).unwrap();
        for body in [surface, sphere, torus] {
            let encoded = encode_native_body(&body).unwrap();
            let restored = decode_native_body(&encoded).unwrap();
            assert_eq!(body.is_surface(), restored.is_surface());
            assert_eq!(body.face_count(), restored.face_count());
            assert_eq!(encoded, encode_native_body(&restored).unwrap());
            if !body.is_surface() {
                equal(&body, &restored);
            }
        }
    }

    #[test]
    fn malformed_native_geometry_is_bounded_and_never_panics() {
        let body = crate::box_solid(Vec3::ZERO, Vec3::new(10., 20., 30.)).unwrap();
        let bytes = encode_native_body(&body).unwrap();
        for n in [0, 1, 7, 8, 9, bytes.len() - 1] {
            assert!(decode_native_body(&bytes[..n]).is_err());
        }
        for i in (0..bytes.len()).step_by((bytes.len() / 256).max(1)) {
            let mut bad = bytes.clone();
            bad[i] ^= 0xff;
            assert!(std::panic::catch_unwind(|| decode_native_body(&bad)).is_ok());
        }
        let mut raw = &bytes[8..];
        let value = binary::read(&mut raw, 0, &mut 2_000_000).unwrap();
        for key in ["vertices", "index"] {
            let mut v = value.clone();
            if key == "vertices" {
                v["solid"]["shells"][0]["edges"][0]["vertices"] = serde_json::json!([u64::MAX, 0]);
            } else {
                v["solid"]["shells"][0]["faces"][0]["loops"][0][0]["edge"] = serde_json::json!(u64::MAX);
            }
            let mut bad = MAGIC.to_vec();
            binary::write(&v, &mut bad, 0).unwrap();
            assert!(decode_native_body(&bad).is_err());
        }
        let mut huge = MAGIC.to_vec();
        huge.push(7);
        huge.extend(u32::MAX.to_le_bytes());
        assert!(decode_native_body(&huge).is_err());
    }

    #[test]
    fn native_preserves_face_appearance_and_authored_boundaries() {
        let mut body = crate::box_solid(Vec3::new(1., 2., 3.), Vec3::new(10., 20., 30.))
            .unwrap()
            .with_paint(Some(crate::Paint { opacity: 0.7, faces: vec![crate::FacePaint { face: 2, color: [0.25, 0.5, 0.75], opacity: 0.6 }] }));
        body.splits = Some(Arc::new(vec![vec![Vec3::new(1., 2., 3.), Vec3::new(11., 2., 3.)]]));
        body.regularization_protected = true;
        let restored = decode_native_body(&encode_native_body(&body).unwrap()).unwrap();
        equal(&body, &restored);
    }
}
