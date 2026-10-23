//! Bounded render-only projected displacement from original linear L16 PNGs.
//! Physical millimetres, periodic bilinear height and exact source identity. The
//! finite micromesh changes silhouette/visibility; the engineering mesh is shared
//! unchanged. This is a fixed-resolution projection, not adaptive displacement.
use super::{CancelToken, ErrorCode, JobError, Result, textures::PlanarFrame};
use image::ImageDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use solidcraft_geom::{Mesh, Vec3};
use std::{
    io::{BufReader, Cursor},
    sync::Arc,
};
const MAX_HEIGHT_BYTES: usize = 16 * 1024 * 1024;
const MAX_HEIGHT_PIXELS: u64 = 4_194_304;
pub const MAX_DISPLACED_TRIANGLES: u64 = 2_000_000;
pub const MAX_DISPLACEMENT_SCRATCH: u64 = 384 * 1024 * 1024;
fn invalid(message: impl Into<String>) -> JobError {
    JobError::new(ErrorCode::InvalidRequest, message)
}
fn allocation() -> JobError {
    JobError::new(ErrorCode::AllocationFailed, "displacement allocation failed")
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Displacement {
    /// Full black-to-white height span. Midlevel is the unchanged surface value.
    pub amplitude_mm: f64,
    pub midlevel: f64,
    /// Uniform power-of-two edge subdivisions; no automatic quality reduction.
    pub subdivision: u32,
}
impl Default for Displacement {
    fn default() -> Self {
        Self { amplitude_mm: 0.1, midlevel: 0.5, subdivision: 3 }
    }
}
impl Displacement {
    pub fn validate(self) -> Result<()> {
        if !self.amplitude_mm.is_finite()
            || !(-10000.0..=10000.0).contains(&self.amplitude_mm)
            || !self.midlevel.is_finite()
            || !(0.0..=1.0).contains(&self.midlevel)
            || self.subdivision > 6
        {
            return Err(invalid("displacement requires finite amplitude -10000..10000 mm, midlevel 0..1 and subdivision 0..6"));
        }
        Ok(())
    }
    pub fn counts(self, triangles: u64) -> Result<(u64, u64)> {
        self.validate()?;
        let n = 1_u64 << self.subdivision;
        let faces = triangles.checked_mul(n * n).ok_or_else(|| invalid("displacement triangle count overflow"))?;
        let vertices = triangles.checked_mul((n + 1) * (n + 2) / 2).ok_or_else(|| invalid("displacement vertex count overflow"))?;
        if faces > MAX_DISPLACED_TRIANGLES || vertices > u64::from(u32::MAX) {
            return Err(JobError::new(ErrorCode::LimitExceeded, "displacement exceeds 2 million generated triangles; choose subdivision explicitly"));
        }
        Ok((faces, vertices))
    }
}
#[derive(Clone, Debug)]
pub struct HeightSource {
    bytes: Arc<Vec<u8>>,
    sha256: [u8; 32],
    dimensions: [u32; 2],
}
impl HeightSource {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        if bytes.is_empty() || bytes.capacity() > MAX_HEIGHT_BYTES {
            return Err(invalid("height source requires a nonempty PNG within 16 MiB"));
        }
        let mut decoder = image::codecs::png::PngDecoder::new(BufReader::new(Cursor::new(bytes.as_slice())))
            .map_err(|e| invalid(format!("invalid height PNG: {e}")))?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(32 * 1024 * 1024);
        decoder.set_limits(limits).map_err(|e| invalid(e.to_string()))?;
        let (width, height) = decoder.dimensions();
        if decoder.color_type() != image::ColorType::L16 || width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_HEIGHT_PIXELS {
            return Err(invalid("height input must be explicit single-channel linear unsigned 16-bit PNG, at most 4,194,304 pixels"));
        }
        let expected = u64::from(width) * u64::from(height) * 2;
        if decoder.total_bytes() != expected {
            return Err(invalid("height source metadata disagrees with its pixel layout"));
        }
        drop(decoder);
        Ok(Self { sha256: Sha256::digest(&bytes).into(), bytes: Arc::new(bytes), dimensions: [width, height] })
    }
    pub fn identity(&self) -> [u8; 32] {
        self.sha256
    }
    pub fn retained_bytes(&self) -> u64 {
        self.bytes.capacity() as u64
    }
    pub fn decoded_bytes(&self) -> u64 {
        u64::from(self.dimensions[0]) * u64::from(self.dimensions[1]) * 2
    }
    pub fn shares_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.bytes, &other.bytes)
    }
    pub fn decode(&self, cancel: &CancelToken) -> Result<HeightImage> {
        cancel.check()?;
        let mut decoder =
            image::codecs::png::PngDecoder::new(BufReader::new(Cursor::new(self.bytes.as_slice()))).map_err(|e| invalid(e.to_string()))?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(32 * 1024 * 1024);
        decoder.set_limits(limits).map_err(|e| invalid(e.to_string()))?;
        if decoder.color_type() != image::ColorType::L16 || decoder.dimensions() != (self.dimensions[0], self.dimensions[1]) {
            return Err(invalid("height decoder identity changed"));
        }
        let count = self.decoded_bytes() as usize;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(count).map_err(|_| allocation())?;
        if bytes.capacity() != count {
            return Err(invalid("height decode capacity exceeds admission"));
        }
        bytes.resize(count, 0);
        decoder.read_image(&mut bytes).map_err(|e| invalid(e.to_string()))?;
        cancel.check()?;
        // image's ImageDecoder emits native-endian 16-bit values, not PNG network byte order.
        Ok(HeightImage { bytes, dimensions: self.dimensions })
    }
}
#[derive(Clone, Debug)]
pub struct HeightDisplacement {
    pub source: HeightSource,
    pub controls: Displacement,
}
pub struct HeightImage {
    bytes: Vec<u8>,
    dimensions: [u32; 2],
}
impl HeightImage {
    pub fn sample(&self, uv: [f64; 2]) -> Result<(f64, [f64; 2])> {
        if uv.iter().any(|v| !v.is_finite() || v.abs() > 128.0) {
            return Err(invalid("height projection exceeds 128 finite repeats"));
        }
        let [w, h] = self.dimensions;
        let x = uv[0].rem_euclid(1.0) * f64::from(w) - 0.5;
        let y = uv[1].rem_euclid(1.0) * f64::from(h) - 0.5;
        let ix = x.floor() as i64;
        let iy = y.floor() as i64;
        let fx = x - x.floor();
        let fy = y - y.floor();
        let value = |x: i64, y: i64| -> Result<f64> {
            let index = (y.rem_euclid(i64::from(h)) as usize * w as usize + x.rem_euclid(i64::from(w)) as usize) * 2;
            let pair = self.bytes.get(index..index + 2).ok_or_else(|| invalid("height sample outside decoded storage"))?;
            Ok(f64::from(u16::from_ne_bytes([pair[0], pair[1]])) / 65535.0)
        };
        let [a, b, c, d] = [value(ix, iy)?, value(ix + 1, iy)?, value(ix, iy + 1)?, value(ix + 1, iy + 1)?];
        let height = (a * (1.0 - fx) + b * fx) * (1.0 - fy) + (c * (1.0 - fx) + d * fx) * fy;
        let du = ((b - a) * (1.0 - fy) + (d - c) * fy) * f64::from(w);
        let dv = ((c - a) * (1.0 - fx) + (d - b) * fx) * f64::from(h);
        Ok((height, [du, dv]))
    }
}
fn vector(a: [f64; 3]) -> Vec3 {
    Vec3::new(a[0], a[1], a[2])
}
fn reserve<T>(count: usize) -> Result<Vec<T>> {
    let mut value = Vec::new();
    value.try_reserve_exact(count).map_err(|_| allocation())?;
    if value.capacity() != count {
        return Err(invalid("displaced mesh capacity exceeds admission"));
    }
    Ok(value)
}
/// The projected map is a shear along the plane normal: UV stays unchanged, and
/// normals follow the exact inverse-transpose Jacobian of the bilinear height.
fn point(position: Vec3, normal: Vec3, frame: PlanarFrame, image: &HeightImage, controls: Displacement, flip_v: bool) -> Result<(Vec3, Vec3)> {
    let [u, v] = frame.uv(position)?;
    let (height, [du, dv]) = image.sample([u, if flip_v { -v } else { v }])?;
    let fu = vector(frame.u);
    let fv = vector(frame.v);
    let axis = fu.cross(fv).normalized().ok_or_else(|| invalid("degenerate displacement projection"))?;
    let gradient = (fu * (du / fu.len2()) + fv * (dv * if flip_v { -1.0 } else { 1.0 } / fv.len2())) * controls.amplitude_mm;
    let n = (normal - gradient * normal.dot(axis)).normalized().ok_or_else(|| invalid("displacement normal collapsed"))?;
    let p = position + axis * ((height - controls.midlevel) * controls.amplitude_mm);
    if !p.is_finite() || p.x.abs().max(p.y.abs()).max(p.z.abs()) > 1e12 {
        return Err(invalid("displacement produced an unbounded vertex"));
    }
    Ok((p, n))
}
pub fn mesh(source: &Mesh, frame: PlanarFrame, height: &HeightDisplacement, flip_v: bool, cancel: &CancelToken) -> Result<Mesh> {
    cancel.check()?;
    frame.validate()?;
    height.controls.validate()?;
    let (faces, vertices) = height.controls.counts(source.triangles.len() as u64)?;
    let required = vertices * 48 + faces * 16 + height.source.decoded_bytes() + 32 * 1024 * 1024;
    if required > MAX_DISPLACEMENT_SCRATCH {
        return Err(JobError::new(ErrorCode::LimitExceeded, "displacement scratch exceeds 384 MiB"));
    }
    if source.normals.len() != source.positions.len() || source.tri_face.len() != source.triangles.len() {
        return Err(invalid("displacement requires exact per-vertex normals and per-triangle source face identities"));
    }
    let image = height.source.decode(cancel)?;
    let mut mesh = Mesh {
        positions: reserve(vertices as usize)?,
        normals: reserve(vertices as usize)?,
        triangles: reserve(faces as usize)?,
        tri_face: reserve(faces as usize)?,
        ..Default::default()
    };
    let n = 1usize << height.controls.subdivision;
    let row = |i: usize| -> usize { i * (n + 1) - i.saturating_sub(1) * i / 2 };
    for (ordinal, triangle) in source.triangles.iter().enumerate() {
        cancel.check()?;
        let mut p = [Vec3::ZERO; 3];
        let mut norm = [Vec3::ZERO; 3];
        for k in 0..3 {
            p[k] = *source.positions.get(triangle[k] as usize).ok_or_else(|| invalid("displacement source index is out of bounds"))?;
            norm[k] = *source.normals.get(triangle[k] as usize).ok_or_else(|| invalid("displacement source normal is missing"))?;
            if !p[k].is_finite() || !norm[k].is_finite() {
                return Err(invalid("nonfinite displacement source"));
            }
        }
        let geometric = (p[1] - p[0]).cross(p[2] - p[0]).normalized().ok_or_else(|| invalid("displacement source contains a degenerate triangle"))?;
        let offset = mesh.positions.len() as u32;
        for i in 0..=n {
            cancel.check()?;
            for j in 0..=n - i {
                let b = i as f64 / n as f64;
                let c = j as f64 / n as f64;
                let a = 1.0 - b - c;
                let normal = (norm[0] * a + norm[1] * b + norm[2] * c).normalized().unwrap_or(geometric);
                let (position, normal) = point(p[0] * a + p[1] * b + p[2] * c, normal, frame, &image, height.controls, flip_v)?;
                mesh.positions.push(position);
                mesh.normals.push(normal);
            }
        }
        for i in 0..n {
            for j in 0..n - i {
                let a = offset + (row(i) + j) as u32;
                let b = offset + (row(i + 1) + j) as u32;
                let c = a + 1;
                mesh.triangles.push([a, b, c]);
                mesh.tri_face.push(source.tri_face[ordinal]);
                if j + 1 < n - i {
                    mesh.triangles.push([b, b + 1, c]);
                    mesh.tri_face.push(source.tri_face[ordinal]);
                }
            }
        }
    }
    if mesh.triangles.len() as u64 != faces || mesh.positions.len() as u64 != vertices {
        return Err(invalid("displacement count disagrees with admitted work"));
    }
    cancel.check()?;
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;
    fn height(values: &[u16], w: u32, h: u32) -> HeightSource {
        let mut png = Vec::new();
        let bytes: Vec<_> = values.iter().flat_map(|v| v.to_ne_bytes()).collect();
        image::codecs::png::PngEncoder::new(&mut png).write_image(&bytes, w, h, image::ExtendedColorType::L16).unwrap();
        HeightSource::new(png).unwrap()
    }
    #[test]
    fn full_16bit_height_precision_and_bilinear_derivatives_are_not_reduced_to_eight_bits() {
        let source = height(&[32768, 32769, 32770, 32771], 2, 2);
        let decoded = source.decode(&CancelToken::default()).unwrap();
        let (a, _) = decoded.sample([0.25, 0.25]).unwrap();
        let (b, _) = decoded.sample([0.75, 0.25]).unwrap();
        assert!((b - a - 1.0 / 65535.0).abs() < 1e-14);
        let (center, derivative) = decoded.sample([0.5, 0.5]).unwrap();
        assert!((center - 32769.5 / 65535.0).abs() < 1e-14);
        assert!((derivative[0] - 2.0 / 65535.0).abs() < 1e-14 && (derivative[1] - 4.0 / 65535.0).abs() < 1e-14);
        assert_eq!(decoded.sample([0.25, 0.25]).unwrap(), decoded.sample([-0.75, 1.25]).unwrap());
        assert!(decoded.sample([f64::NAN, 0.0]).is_err());
        let clone = source.clone();
        assert!(source.shares_storage(&clone));
    }
    #[test]
    fn real_subdivision_changes_positions_not_cad_source_and_preserves_winding_and_face_ids() {
        let source = Mesh {
            positions: vec![Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0)],
            normals: vec![Vec3::Z; 3],
            triangles: vec![[0, 1, 2]],
            tri_face: vec![7],
            ..Default::default()
        };
        let frame = PlanarFrame { origin: [0.0; 3], u: [2.0, 0.0, 0.0], v: [0.0, 2.0, 0.0] };
        let height =
            HeightDisplacement { source: height(&[65535], 1, 1), controls: Displacement { amplitude_mm: 4.0, midlevel: 0.5, subdivision: 2 } };
        let actual = mesh(&source, frame, &height, false, &CancelToken::default()).unwrap();
        assert_eq!((actual.positions.len(), actual.triangles.len()), (15, 16));
        assert!(actual.positions.iter().all(|p| p.z == 2.0));
        assert!(actual.tri_face.iter().all(|f| *f == 7));
        assert!(actual.normals.iter().all(|n| *n == Vec3::Z));
        assert!(source.positions.iter().all(|p| p.z == 0.0));
        for t in &actual.triangles {
            assert!(
                (actual.positions[t[1] as usize] - actual.positions[t[0] as usize])
                    .cross(actual.positions[t[2] as usize] - actual.positions[t[0] as usize])
                    .z
                    > 0.0
            );
        }
        let cancel = CancelToken::default();
        cancel.cancel();
        assert!(mesh(&source, frame, &height, false, &cancel).is_err());
        assert!(Displacement { subdivision: 7, ..Default::default() }.validate().is_err());
        assert!(Displacement { subdivision: 6, ..Default::default() }.counts(1000).is_err());
        assert!(Displacement { amplitude_mm: f64::INFINITY, ..Default::default() }.validate().is_err());
    }
}
