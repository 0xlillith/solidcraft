//! Versioned design packages. Geometry is rebuilt from authoritative inputs; no
//! evaluated cache is trusted here. All archive access stays in memory, never on disk.
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use solvecraft_doc::{Document, FeatureKind, MeshData};

use crate::{IoError, MAX_DESIGN_BYTES, Result, design_metadata::replace};

const VERSION: &str = "solvecraft-container/1";
const MAX_ENTRIES: usize = 10_000;
const MAX_EXPANDED: usize = 512 << 20;
const MESH_MAGIC: &[u8; 8] = b"SCMESH01";

/// Fast favors interactive file operations; Compact trades CPU time for space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DesignCompression {
    #[default]
    Fast,
    Compact,
}

fn invalid(message: impl std::fmt::Display) -> IoError {
    IoError::Invalid(format!("design container: {message}"))
}

/// Detect the package signature independently of the file name.
pub fn is_design_container(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04")
}

fn fields(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> Map<String, Value> {
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct PackageWriter {
    zip: zip::ZipWriter<Cursor<Vec<u8>>>,
    names: BTreeSet<String>,
    expanded: usize,
    resolved: usize,
}

impl PackageWriter {
    fn part(&mut self, name: &str, bytes: &[u8], compressed: bool) -> Result<()> {
        if bytes.len() > MAX_DESIGN_BYTES {
            return Err(invalid("part too large"));
        }
        if !self.names.insert(name.to_owned()) {
            return Ok(());
        }
        self.expanded = self.expanded.checked_add(bytes.len()).ok_or_else(|| invalid("expanded size overflow"))?;
        if self.expanded > MAX_EXPANDED || self.names.len() > MAX_ENTRIES {
            return Err(invalid("package too large"));
        }
        let method = if compressed { zip::CompressionMethod::Deflated } else { zip::CompressionMethod::Stored };
        let options = zip::write::SimpleFileOptions::default().compression_method(method).compression_level(compressed.then_some(1));
        self.zip.start_file(name, options).map_err(invalid)?;
        self.zip.write_all(bytes).map_err(invalid)
    }
    fn blob(&mut self, bytes: &[u8], extension: &str, compressed: bool) -> Result<Value> {
        self.resolved =
            self.resolved.checked_add(bytes.len()).filter(|&n| n <= MAX_EXPANDED).ok_or_else(|| invalid("resolved payloads too large"))?;
        let name = format!("blobs/{}.{extension}", digest(bytes));
        self.part(&name, bytes, compressed)?;
        Ok(Value::String(name))
    }
}

/// Write a compact container, deduplicating payloads by SHA-256. Mesh arrays are
/// stored directly for low save/open latency; small metadata uses level-one
/// Deflate. Use the explicit Compact option to compress STEP and meshes too.
pub fn write_design_container(doc: &Document) -> Result<Vec<u8>> {
    write_design_container_with_compression(doc, DesignCompression::Fast)
}

/// Write a container with an explicit payload compression policy.
pub fn write_design_container_with_compression(doc: &Document, compression: DesignCompression) -> Result<Vec<u8>> {
    let mut writer = PackageWriter { zip: zip::ZipWriter::new(Cursor::new(Vec::new())), names: BTreeSet::new(), expanded: 0, resolved: 0 };
    let mut features = Vec::with_capacity(doc.features.len());
    for feature in &doc.features {
        let replacements = match &feature.kind {
            FeatureKind::Import { step, .. } => {
                let blob = writer.blob(step.as_bytes(), "step", compression == DesignCompression::Compact)?;
                fields([("step", Value::String(String::new())), ("step_blob", blob)])
            }
            FeatureKind::MeshImport { meshes, .. } => {
                let mut metadata = Vec::with_capacity(meshes.len());
                for mesh in meshes {
                    let bytes = encode_mesh(mesh)?;
                    let blob = writer.blob(&bytes, "mesh", compression == DesignCompression::Compact)?;
                    let mut value = replace(mesh, fields([("positions", json!([])), ("triangles", json!([]))])).map_err(invalid)?;
                    value.as_object_mut().ok_or_else(|| invalid("mesh metadata"))?.insert("mesh_blob".into(), blob);
                    metadata.push(value);
                }
                fields([("meshes", Value::Array(metadata))])
            }
            _ => Map::new(),
        };
        // Extra reference fields are added after serializing existing fields.
        let step_blob = replacements.get("step_blob").cloned();
        let mut value = replace(feature, replacements).map_err(invalid)?;
        if let Some(blob) = step_blob {
            value.as_object_mut().ok_or_else(|| invalid("feature metadata"))?.insert("step_blob".into(), blob);
        }
        features.push(value);
    }
    let mut canvases = Vec::with_capacity(doc.canvases.len());
    for canvas in &doc.canvases {
        if canvas.data.len() > solvecraft_doc::canvas::MAX_CANVAS_BYTES.div_ceil(3) * 4 {
            return Err(invalid("canvas too large"));
        }
        let bytes = canvas.bytes().ok_or_else(|| invalid("invalid canvas base64"))?;
        if !matches!(canvas.format.as_str(), "png" | "jpeg") || bytes.len() > solvecraft_doc::canvas::MAX_CANVAS_BYTES {
            return Err(invalid("invalid canvas format or size"));
        }
        let blob = writer.blob(&bytes, &canvas.format, false)?;
        let mut value = replace(canvas, fields([("data", json!(""))])).map_err(invalid)?;
        value.as_object_mut().ok_or_else(|| invalid("canvas metadata"))?.insert("image_blob".into(), blob);
        canvases.push(value);
    }
    let metadata = replace(doc, fields([("features", Value::Array(features)), ("canvases", Value::Array(canvases))])).map_err(invalid)?;
    writer.part("document.json", &serde_json::to_vec(&metadata).map_err(invalid)?, true)?;
    writer.part("container.json", &serde_json::to_vec(&json!({"format": VERSION})).map_err(invalid)?, false)?;
    let bytes = writer.zip.finish().map_err(invalid)?.into_inner();
    if bytes.len() > MAX_DESIGN_BYTES {
        return Err(invalid("compressed file too large"));
    }
    Ok(bytes)
}

fn validate_mesh(mesh: &MeshData) -> Result<()> {
    if !mesh.positions.len().is_multiple_of(3)
        || !mesh.triangles.len().is_multiple_of(3)
        || mesh.positions.iter().any(|v| !v.is_finite() || v.abs() > 1e9)
        || mesh.triangles.iter().any(|&v| v as usize >= mesh.positions.len() / 3)
    {
        return Err(invalid("invalid mesh coordinates or indices"));
    }
    Ok(())
}

fn encode_mesh(mesh: &MeshData) -> Result<Vec<u8>> {
    validate_mesh(mesh)?;
    let size = mesh
        .positions
        .len()
        .checked_mul(8)
        .and_then(|v| mesh.triangles.len().checked_mul(4).and_then(|t| v.checked_add(t)))
        .and_then(|v| v.checked_add(24))
        .filter(|&n| n <= MAX_DESIGN_BYTES)
        .ok_or_else(|| invalid("mesh too large"))?;
    let mut bytes = Vec::with_capacity(size);
    bytes.extend_from_slice(MESH_MAGIC);
    bytes.extend_from_slice(&(mesh.positions.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&(mesh.triangles.len() as u64).to_le_bytes());
    for value in &mesh.positions {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in &mesh.triangles {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    Ok(bytes)
}

fn decode_mesh(bytes: &[u8], mesh: &mut MeshData) -> Result<()> {
    if bytes.get(..8) != Some(MESH_MAGIC.as_slice()) {
        return Err(invalid("unsupported mesh blob version"));
    }
    let count = |range: std::ops::Range<usize>| -> Result<usize> {
        let array: [u8; 8] = bytes.get(range).ok_or_else(|| invalid("truncated mesh blob"))?.try_into().map_err(invalid)?;
        usize::try_from(u64::from_le_bytes(array)).map_err(invalid)
    };
    let positions = count(8..16)?;
    let triangles = count(16..24)?;
    let middle = positions.checked_mul(8).and_then(|n| n.checked_add(24)).ok_or_else(|| invalid("mesh size overflow"))?;
    let total = triangles.checked_mul(4).and_then(|n| n.checked_add(middle)).ok_or_else(|| invalid("mesh size overflow"))?;
    if total != bytes.len() || !positions.is_multiple_of(3) || !triangles.is_multiple_of(3) {
        return Err(invalid("invalid mesh blob lengths"));
    }
    mesh.positions =
        bytes.get(24..middle).ok_or_else(|| invalid("truncated mesh"))?.as_chunks::<8>().0.iter().map(|&v| f64::from_le_bytes(v)).collect();
    mesh.triangles =
        bytes.get(middle..).ok_or_else(|| invalid("truncated mesh"))?.as_chunks::<4>().0.iter().map(|&v| u32::from_le_bytes(v)).collect();
    validate_mesh(mesh)
}

struct PackageReader<'a> {
    zip: zip::ZipArchive<Cursor<&'a [u8]>>,
    blobs: BTreeMap<String, Vec<u8>>,
    hydrated: usize,
}

impl<'a> PackageReader<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self> {
        // zip's name index can coalesce duplicate central-directory entries. Our
        // bounded format does not need ZIP64/multiple disks: verify the declared
        // entry count as well so duplicate names cannot silently select a payload.
        let tail = bytes.get(bytes.len().saturating_sub(65_557)..).ok_or_else(|| invalid("missing ZIP directory"))?;
        let eocd = tail.windows(4).rposition(|v| v == b"PK\x05\x06").ok_or_else(|| invalid("missing ZIP directory"))?;
        let end = tail.get(eocd..).filter(|v| v.len() >= 22).ok_or_else(|| invalid("truncated ZIP directory"))?;
        let u16_at = |offset| -> Result<u16> {
            let array: [u8; 2] = end.get(offset..offset + 2).ok_or_else(|| invalid("truncated ZIP directory"))?.try_into().map_err(invalid)?;
            Ok(u16::from_le_bytes(array))
        };
        let declared = u16_at(10)? as usize;
        if declared > MAX_ENTRIES || u16_at(4)? != 0 || u16_at(6)? != 0 || u16_at(8)? as usize != declared || end.len() != 22 + u16_at(20)? as usize {
            return Err(invalid("too many entries or unsupported ZIP directory"));
        }
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(invalid)?;
        if u16_at(4)? != 0
            || u16_at(6)? != 0
            || u16_at(8)? as usize != zip.len()
            || u16_at(10)? as usize != zip.len()
            || end.len() != 22 + u16_at(20)? as usize
        {
            return Err(invalid("duplicate entries or unsupported ZIP directory"));
        }
        let mut names = BTreeSet::new();
        let mut total = 0u64;
        for i in 0..zip.len() {
            let part = zip.by_index_raw(i).map_err(invalid)?;
            let name = part.name();
            if name.is_empty()
                || name.contains('\\')
                || name.split('/').any(|c| c.is_empty() || c == "." || c == "..")
                || !names.insert(name.to_owned())
            {
                return Err(invalid("duplicate or invalid archive entry"));
            }
            total = total.checked_add(part.size()).ok_or_else(|| invalid("expanded size overflow"))?;
            if part.size() > MAX_DESIGN_BYTES as u64 || total > MAX_EXPANDED as u64 {
                return Err(invalid("expanded package too large"));
            }
        }
        Ok(Self { zip, blobs: BTreeMap::new(), hydrated: 0 })
    }
    fn part(&mut self, name: &str) -> Result<Vec<u8>> {
        let part = self.zip.by_name(name).map_err(|e| invalid(format!("required part `{name}`: {e}")))?;
        let expected = part.size();
        if expected > MAX_DESIGN_BYTES as u64 {
            return Err(invalid("part too large"));
        }
        let mut bytes = Vec::with_capacity(expected as usize);
        part.take(MAX_DESIGN_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(invalid)?;
        if bytes.len() > MAX_DESIGN_BYTES || bytes.len() as u64 != expected {
            return Err(invalid("part too large or length mismatch"));
        }
        Ok(bytes)
    }
    fn blob(&mut self, value: &Value, field: &str, extension: &str) -> Result<&[u8]> {
        let name = value.get(field).and_then(Value::as_str).ok_or_else(|| invalid(format!("missing `{field}` reference")))?;
        let hash = name
            .strip_prefix("blobs/")
            .and_then(|n| n.strip_suffix(&format!(".{extension}")))
            .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
            .ok_or_else(|| invalid("invalid blob reference"))?;
        if !self.blobs.contains_key(name) {
            let bytes = self.part(name)?;
            if digest(&bytes) != hash {
                return Err(invalid(format!("blob hash mismatch: {name}")));
            }
            self.blobs.insert(name.to_owned(), bytes);
        }
        let bytes = self.blobs.get(name).ok_or_else(|| invalid("missing blob"))?;
        // Count repeated references too: a tiny archive must not expand into unbounded
        // owned document strings/arrays when the same blob is referenced repeatedly.
        self.hydrated =
            self.hydrated.checked_add(bytes.len()).filter(|&n| n <= MAX_EXPANDED).ok_or_else(|| invalid("resolved payloads too large"))?;
        Ok(bytes)
    }
}

pub(super) fn read(bytes: &[u8]) -> Result<Document> {
    let mut reader = PackageReader::new(bytes)?;
    let version: Value = serde_json::from_slice(&reader.part("container.json")?).map_err(invalid)?;
    if version.get("format").and_then(Value::as_str) != Some(VERSION) {
        return Err(invalid("unsupported container version"));
    }
    let bytes = reader.part("document.json")?;
    let metadata: Value = serde_json::from_slice(&bytes).map_err(invalid)?;
    let text = std::str::from_utf8(&bytes).map_err(invalid)?;
    // Migrations and all document validation run on the small parametric metadata.
    // Binary arrays have their own equivalent numeric/index validation below.
    let mut doc = Document::from_json(text)?;
    for (index, feature) in doc.features.iter_mut().enumerate() {
        let value = metadata.get("features").and_then(|v| v.get(index)).ok_or_else(|| invalid("missing feature metadata"))?;
        match &mut feature.kind {
            FeatureKind::Import { step, .. } => {
                if !step.is_empty() {
                    return Err(invalid("ambiguous embedded STEP and blob"));
                }
                *step = std::str::from_utf8(reader.blob(value, "step_blob", "step")?).map_err(invalid)?.to_owned();
            }
            FeatureKind::MeshImport { meshes, .. } => {
                for (index, mesh) in meshes.iter_mut().enumerate() {
                    if !mesh.positions.is_empty() || !mesh.triangles.is_empty() {
                        return Err(invalid("ambiguous embedded mesh and blob"));
                    }
                    let value = value.get("meshes").and_then(|v| v.get(index)).ok_or_else(|| invalid("missing mesh metadata"))?;
                    decode_mesh(reader.blob(value, "mesh_blob", "mesh")?, mesh)?;
                }
            }
            _ => {}
        }
    }
    for (index, canvas) in doc.canvases.iter_mut().enumerate() {
        if !canvas.data.is_empty() || !matches!(canvas.format.as_str(), "png" | "jpeg") {
            return Err(invalid("invalid canvas metadata"));
        }
        let value = metadata.get("canvases").and_then(|v| v.get(index)).ok_or_else(|| invalid("missing canvas metadata"))?;
        let bytes = reader.blob(value, "image_blob", &canvas.format)?;
        if bytes.len() > solvecraft_doc::canvas::MAX_CANVAS_BYTES {
            return Err(invalid("canvas too large"));
        }
        canvas.data = solvecraft_doc::canvas::base64_encode(bytes);
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solvecraft_geom::Vec2;

    fn mesh() -> MeshData {
        MeshData {
            name: "Precise mesh".into(),
            positions: vec![0.123456789012345, -0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            triangles: vec![0, 1, 2],
            color: Some([0.25, 0.5, 0.75]),
        }
    }

    fn document() -> Document {
        let mut doc = Document::new("Payloads");
        for _ in 0..2 {
            doc.add_feature(
                FeatureKind::Import { file: "example.step".into(), step: "ISO-10303-21;\nEND-ISO-10303-21;".into(), components: Vec::new() },
                None,
            )
            .unwrap();
        }
        let mut other = mesh();
        other.name = "Same triangles, separate appearance".into();
        other.color = None;
        doc.add_feature(FeatureKind::MeshImport { file: "example.obj".into(), meshes: vec![mesh(), other] }, None).unwrap();
        doc.canvases.push(solvecraft_doc::canvas::Canvas {
            id: 99,
            name: "Reference".into(),
            format: "png".into(),
            data: solvecraft_doc::canvas::base64_encode(b"original image bytes"),
            pixels: [1, 1],
            plane: solvecraft_doc::PlaneRef::Origin { name: "XY".into() },
            center: Vec2::new(0.0, 0.0),
            width: 10.0,
            angle: 0.0,
            opacity: 0.5,
            flip: false,
            visible: true,
            component: 0,
        });
        doc
    }

    fn parts(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        (0..zip.len())
            .map(|i| {
                let mut entry = zip.by_index(i).unwrap();
                let name = entry.name().to_owned();
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                (name, bytes)
            })
            .collect()
    }

    fn package(parts: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in parts {
            zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn round_trip_deduplicates_and_preserves_binary_precision() {
        let source = Document::from_json(&document().to_json()).unwrap();
        let bytes = write_design_container(&source).unwrap();
        assert!(is_design_container(&bytes));
        let reopened = crate::read_design(&bytes).unwrap();
        assert_eq!(source, reopened);
        let FeatureKind::MeshImport { meshes, .. } = &reopened.features[2].kind else { panic!() };
        assert_eq!(meshes[0].positions[1].to_bits(), (-0.0f64).to_bits());
        let entries = parts(&bytes);
        assert_eq!(entries.keys().filter(|n| n.ends_with(".step")).count(), 1);
        assert_eq!(entries.keys().filter(|n| n.ends_with(".mesh")).count(), 1);
        assert_eq!(entries.keys().filter(|n| n.ends_with(".png")).count(), 1);
        let json: Value = serde_json::from_slice(&entries["document.json"]).unwrap();
        assert_eq!(json["features"][2]["meshes"][0]["positions"], json!([]));
        assert_eq!(crate::read_design(&crate::write_design(&reopened)).unwrap(), source);
        assert_eq!(write_design_container(&source).unwrap(), bytes, "deterministic archive");
        let compact = write_design_container_with_compression(&source, DesignCompression::Compact).unwrap();
        assert_eq!(crate::read_design(&compact).unwrap(), source);
    }

    #[test]
    fn required_blobs_must_exist_and_match_hashes() {
        let bytes = write_design_container(&document()).unwrap();
        let mut entries = parts(&bytes);
        let name = entries.keys().find(|n| n.ends_with(".step")).unwrap().clone();
        let original = entries.remove(&name).unwrap();
        assert!(crate::read_design(&package(&entries)).unwrap_err().to_string().contains("required part"));
        entries.insert(name.clone(), b"damaged".to_vec());
        assert!(crate::read_design(&package(&entries)).unwrap_err().to_string().contains("hash mismatch"));
        entries.insert(name, original);
        entries.insert("cache/obsolete.bin".into(), b"untrusted optional cache".to_vec());
        assert!(crate::read_design(&package(&entries)).is_ok());
    }

    #[test]
    fn refuses_hostile_mesh_lengths_numbers_indices_and_versions() {
        let valid = encode_mesh(&mesh()).unwrap();
        for offset in [8, 16] {
            let mut bytes = valid.clone();
            bytes[offset..offset + 8].copy_from_slice(&u64::MAX.to_le_bytes());
            assert!(decode_mesh(&bytes, &mut mesh()).is_err());
        }
        let mut bytes = valid.clone();
        bytes[24..32].copy_from_slice(&f64::NAN.to_le_bytes());
        assert!(decode_mesh(&bytes, &mut mesh()).is_err());
        let mut bytes = valid.clone();
        let offset = bytes.len() - 4;
        bytes[offset..].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode_mesh(&bytes, &mut mesh()).is_err());
        for length in 0..valid.len() {
            assert!(decode_mesh(&valid[..length], &mut mesh()).is_err());
        }
        let mut bytes = valid;
        bytes[7] = b'2';
        assert!(decode_mesh(&bytes, &mut mesh()).is_err());
    }

    #[test]
    fn hostile_archive_paths_versions_and_truncations_are_errors() {
        let bytes = write_design_container(&document()).unwrap();
        let mut entries = parts(&bytes);
        entries.insert("../escape".into(), vec![]);
        assert!(crate::read_design(&package(&entries)).is_err());
        entries.remove("../escape");
        entries.insert("container.json".into(), br#"{"format":"solvecraft-container/999"}"#.to_vec());
        assert!(crate::read_design(&package(&entries)).unwrap_err().to_string().contains("unsupported container version"));
        for length in (0..bytes.len()).step_by(17) {
            assert!(crate::read_design(&bytes[..length]).is_err());
        }
    }

    #[test]
    fn rejects_duplicate_entries_and_declared_expansion_before_reading_payloads() {
        let bytes = write_design_container(&document()).unwrap();
        let mut entries = parts(&bytes);
        entries.insert("cache/one.bin".into(), vec![1]);
        entries.insert("cache/two.bin".into(), vec![2]);
        let mut duplicate = package(&entries);
        let offsets: Vec<_> = duplicate.windows(13).enumerate().filter_map(|(i, v)| (v == b"cache/two.bin").then_some(i)).collect();
        // Both the local header and central-directory name are changed.
        for i in offsets {
            duplicate[i..i + 13].copy_from_slice(b"cache/one.bin");
        }
        assert!(crate::read_design(&duplicate).is_err());
        let mut bomb = bytes.clone();
        let central = bomb.windows(4).position(|v| v == b"PK\x01\x02").unwrap();
        bomb[central + 24..central + 28].copy_from_slice(&((MAX_DESIGN_BYTES + 1) as u32).to_le_bytes());
        assert!(crate::read_design(&bomb).unwrap_err().to_string().contains("expanded package too large"));
        let mut excessive = bytes;
        let end = excessive.windows(4).rposition(|v| v == b"PK\x05\x06").unwrap();
        excessive[end + 8..end + 12].copy_from_slice(&[0xff; 4]);
        assert!(crate::read_design(&excessive).is_err());
    }

    #[test]
    fn rejects_invalid_references_and_repeated_payload_expansion() {
        let bytes = write_design_container(&document()).unwrap();
        let mut entries = parts(&bytes);
        let mut metadata: Value = serde_json::from_slice(&entries["document.json"]).unwrap();
        let reference = metadata["features"][0].clone();
        let mut reader = PackageReader::new(&bytes).unwrap();
        reader.hydrated = MAX_EXPANDED;
        assert!(reader.blob(&reference, "step_blob", "step").unwrap_err().to_string().contains("resolved payloads too large"));
        for name in ["../escape.step", "blobs/not-a-hash.step", "blobs/00.png"] {
            metadata["features"][0]["step_blob"] = json!(name);
            entries.insert("document.json".into(), serde_json::to_vec(&metadata).unwrap());
            assert!(crate::read_design(&package(&entries)).is_err());
        }
    }

    #[test]
    fn deterministic_archive_mutations_never_panic() {
        let bytes = write_design_container(&document()).unwrap();
        let mut seed = 0x303_u64;
        for _ in 0..512 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mut changed = bytes.clone();
            let index = seed as usize % changed.len();
            changed[index] ^= (seed >> 32) as u8 | 1;
            assert!(std::panic::catch_unwind(|| crate::read_design(&changed)).is_ok());
        }
    }

    #[test]
    fn metadata_round_trip_preserves_new_and_existing_fields() {
        let mut doc = Document::new("Empty");
        doc.units = "in".into();
        doc.named_views.push(solvecraft_doc::NamedView { name: "View".into(), ..Default::default() });
        doc.param_comments.insert("test".into(), "metadata".into());
        let expected = crate::read_design(&crate::write_design(&doc)).unwrap();
        assert_eq!(crate::read_design(&write_design_container(&doc).unwrap()).unwrap(), expected);
    }

    /// Public-safe generated data, no geometry evaluation. Print medians without
    /// timing assertions, which would be unreliable on shared build machines.
    #[test]
    #[ignore = "manual IO performance measurement"]
    fn benchmark_design_storage() {
        let mut doc = Document::new("Generated mesh benchmark");
        let mut large = mesh();
        large.positions = (0..300_000).map(|i| (i as f64 * 0.001234567890123).sin() * 100.0).collect();
        large.triangles = (0..300_000).map(|i| (i % 100_000) as u32).collect();
        doc.add_feature(FeatureKind::MeshImport { file: "generated.obj".into(), meshes: vec![large] }, None).unwrap();
        measure_storage(&doc, "mesh");
    }

    #[test]
    #[ignore = "manual IO performance measurement"]
    fn benchmark_step_storage() {
        use solvecraft_geom::Vec3;
        let bodies: Vec<_> = (0..512)
            .map(|i| {
                let corner = Vec3::new((i % 32) as f64 * 12.0, (i / 32) as f64 * 12.0, 0.0);
                solvecraft_kernel::box_solid(corner, corner + Vec3::new(10.0, 10.0, 10.0)).unwrap()
            })
            .collect();
        let step = solvecraft_kernel::step_export(&bodies.iter().collect::<Vec<_>>(), "SolidCraft storage benchmark").unwrap();
        let mut doc = Document::new("Generated STEP benchmark");
        doc.add_feature(FeatureKind::Import { file: "generated.step".into(), step, components: Vec::new() }, None).unwrap();
        measure_storage(&doc, "step");
    }

    fn measure_storage(doc: &Document, label: &str) {
        let json = crate::write_design(doc);
        let container = write_design_container(doc).unwrap();
        let dir = std::env::var_os("CARGO_TARGET_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("target"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("issue303-{label}.json")), &json).unwrap();
        std::fs::write(dir.join(format!("issue303-{label}.solvecraft")), &container).unwrap();
        let mut timings = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for _ in 0..9 {
            let start = std::time::Instant::now();
            std::hint::black_box(crate::write_design(doc));
            timings[0].push(start.elapsed().as_secs_f64() * 1000.0);
            let start = std::time::Instant::now();
            std::hint::black_box(write_design_container(doc).unwrap());
            timings[1].push(start.elapsed().as_secs_f64() * 1000.0);
            let start = std::time::Instant::now();
            let plain = crate::read_design(&json).unwrap();
            timings[2].push(start.elapsed().as_secs_f64() * 1000.0);
            let start = std::time::Instant::now();
            let zipped = crate::read_design(&container).unwrap();
            timings[3].push(start.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(plain, zipped);
        }
        for times in &mut timings {
            times.sort_by(f64::total_cmp);
        }
        eprintln!(
            "{label}: bytes json={} container={}; median ms save_json={:.3} save_container={:.3} read_json={:.3} read_container={:.3}",
            json.len(),
            container.len(),
            timings[0][4],
            timings[1][4],
            timings[2][4],
            timings[3][4]
        );
        let mut saves = [Vec::new(), Vec::new()];
        for round in 0..9 {
            // Alternate ordering; include encoding, flushing, replacement and backup.
            for index in if round % 2 == 0 { [0, 1] } else { [1, 0] } {
                let start = std::time::Instant::now();
                let bytes = if index == 0 { crate::write_design(doc) } else { write_design_container(doc).unwrap() };
                crate::write_atomic(&dir.join(format!("issue303-{label}-atomic-{index}.solvecraft")), &bytes, true).unwrap();
                saves[index].push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        for times in &mut saves {
            times.sort_by(f64::total_cmp);
        }
        eprintln!("{label}: median atomic-save ms json={:.3} container={:.3}", saves[0][4], saves[1][4]);
    }
}
