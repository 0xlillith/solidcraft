//! W38a's first record slice: sheet-owned, paper-mm geometry, with no model evaluation.
//! The Sketch payload stays the shared type. Links, patterns and offsets await the
//! receiver-aware store: refusing them here is explicit, never silent data loss.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use solidcraft_geom::Vec2;
use solidcraft_sketch::{ConstraintKind, CurveKind, MAX_CONSTRAINTS, MAX_COORD, MAX_CURVES, MAX_POINTS, MAX_SPLINE_POINTS, Sketch};

use super::{Drawing, MAX_DRAWING_BYTES, check_name, require};
use crate::{DocError, Result};

/// Maximum sheet sketches in one drawing; independently of the number of sheets.
pub const MAX_SHEET_SKETCHES: usize = 1000;
// One drawing cannot multiply the shared sketch array caps by its sketch count.
const MAX_ENTITIES: usize = MAX_POINTS + MAX_CURVES + MAX_CONSTRAINTS;
const MAX_VALIDATION_WORK: usize = 2_000_000;
const MAX_STORED_TEXT_BYTES: usize = 4096;
const MAX_LOCAL_ID_BYTES: usize = 512;
const AUXILIARY: [&str; 5] = ["links", "patterns", "offsets", "freeform_offsets", "wires"];

/// Authored sheet geometry. Points and lengths are paper XY millimetres, regardless
/// of the drawing's display units. This record has no view scale or source path.
/// Expressions are retained verbatim; interpreting them is the later read-only store's job.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SheetSketch {
    #[serde(with = "crate::ids")]
    pub id: u64,
    #[serde(with = "crate::ids")]
    pub sheet: u64,
    pub name: String,
    pub sketch: Sketch,
}

fn invalid(message: impl Into<String>) -> DocError {
    DocError::Invalid(format!("sheet sketch: {}", message.into()))
}

fn charge(used: &mut usize, amount: usize, limit: usize, what: &str) -> Result<()> {
    *used = used.checked_add(amount).filter(|n| *n <= limit).ok_or_else(|| invalid(format!("{what} limit exceeded")))?;
    Ok(())
}

fn text(s: &str) -> Result<()> {
    require(!s.trim().is_empty() && s.len() <= MAX_STORED_TEXT_BYTES && !s.chars().any(char::is_control), || {
        "sheet sketch expression is empty, too long or contains control characters".into()
    })
}

fn position(p: Vec2) -> Result<()> {
    require(p.is_finite() && p.x.abs() < MAX_COORD && p.y.abs() < MAX_COORD, || "sheet sketch point is out of range".into())
}

fn identity<'a>(seen: &mut BTreeSet<&'a str>, id: &'a str) -> Result<()> {
    require(!id.trim().is_empty() && id.len() <= MAX_LOCAL_ID_BYTES && !id.chars().any(char::is_control), || {
        "sheet sketch local id is empty, too long or contains control characters".into()
    })?;
    require(seen.insert(id), || "sheet sketch local ids must be unique".into())
}

/// Every collection is bounded before serde allocates the typed sketch payload.
/// A raw JSON Value already exists here; the drawing reader separately caps encoded bytes.
pub(super) fn check_raw(value: &Value) -> Result<()> {
    let Some(records) = value.get("sketches") else { return Ok(()) };
    let records = records.as_array().ok_or_else(|| invalid("sketches must be an array"))?;
    require(records.len() <= MAX_SHEET_SKETCHES, || "too many sheet sketches".into())?;
    let mut total = 0;
    let mut nodes = 0;
    let mut bytes = 0;
    for record in records {
        raw_tree(record, 0, &mut nodes, &mut bytes)?;
        let Some(sketch) = record.get("sketch") else { continue };
        for (key, limit) in [("points", MAX_POINTS), ("curves", MAX_CURVES), ("constraints", MAX_CONSTRAINTS)] {
            if let Some(array) = sketch.get(key) {
                let array = array.as_array().ok_or_else(|| invalid(format!("{key} must be an array")))?;
                require(array.len() <= limit, || format!("sheet sketch {key} limit exceeded"))?;
                charge(&mut total, array.len(), MAX_ENTITIES, "drawing sketch entity")?;
            }
        }
        for key in AUXILIARY {
            if let Some(array) = sketch.get(key) {
                let array = array.as_array().ok_or_else(|| invalid(format!("{key} must be an array")))?;
                require(array.is_empty(), || format!("not supported yet: sheet sketch {key}"))?;
            }
        }
        if let Some(curves) = sketch.get("curves").and_then(Value::as_array) {
            for curve in curves {
                if let Some(points) = curve.get("pts").and_then(Value::as_array) {
                    require(points.len() <= MAX_SPLINE_POINTS, || "sheet sketch spline point limit exceeded".into())?;
                    charge(&mut total, points.len(), MAX_ENTITIES, "drawing sketch entity")?;
                }
            }
        }
        if let Some(counters) = sketch.get("counters").and_then(Value::as_object) {
            require(counters.len() <= 64, || "sheet sketch counter prefix limit exceeded".into())?;
            require(counters.keys().all(|key| key.len() <= MAX_LOCAL_ID_BYTES), || "sheet sketch counter prefix too long".into())?;
        }
    }
    Ok(())
}

// These guards apply to from_value as well as from_json, and precede typed decoding.
fn raw_tree(value: &Value, depth: usize, nodes: &mut usize, bytes: &mut usize) -> Result<()> {
    require(depth <= 32, || "sheet sketch payload nested too deeply".into())?;
    charge(nodes, 1, MAX_ENTITIES * 16, "raw node")?;
    match value {
        Value::String(s) => {
            require(s.len() <= MAX_STORED_TEXT_BYTES, || "sheet sketch stored text too long".into())?;
            charge(bytes, s.len(), MAX_DRAWING_BYTES, "stored text")?;
        }
        Value::Array(array) => {
            require(array.len() <= MAX_ENTITIES, || "sheet sketch raw array too large".into())?;
            for item in array {
                raw_tree(item, depth + 1, nodes, bytes)?;
            }
        }
        Value::Object(object) => {
            require(object.len() <= MAX_ENTITIES, || "sheet sketch raw object too large".into())?;
            for (key, item) in object {
                require(key.len() <= MAX_LOCAL_ID_BYTES, || "sheet sketch field name too long".into())?;
                charge(bytes, key.len(), MAX_DRAWING_BYTES, "stored text")?;
                raw_tree(item, depth + 1, nodes, bytes)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn check_drawing(drawing: &Drawing) -> Result<()> {
    require(drawing.sketches.len() <= MAX_SHEET_SKETCHES, || "too many sheet sketches".into())?;
    let sheets: BTreeSet<_> = drawing.sheets.iter().map(|s| s.id).collect();
    let mut entities = 0;
    let mut work = 0;
    for record in &drawing.sketches {
        require(sheets.contains(&record.sheet), || "sheet sketch owner does not exist in this drawing".into())?;
        check_name(&record.name, "sheet sketch")?;
        let sk = &record.sketch;
        for (count, cap) in [(sk.points.len(), MAX_POINTS), (sk.curves.len(), MAX_CURVES), (sk.constraints.len(), MAX_CONSTRAINTS)] {
            require(count <= cap, || "sheet sketch array limit exceeded".into())?;
            charge(&mut entities, count, MAX_ENTITIES, "drawing sketch entity")?;
        }
        require(
            sk.links.is_empty() && sk.patterns.is_empty() && sk.offsets.is_empty() && sk.freeform_offsets.is_empty() && sk.wires.is_empty(),
            || "not supported yet: sheet sketch links, patterns, offsets or 3D wires".into(),
        )?;
        require(!sk.view.three_d && !sk.view.slice, || "model-only sketch view is not a sheet sketch".into())?;
        let origin = sk.points.first().ok_or_else(|| invalid("missing sketch origin"))?;
        require(origin.id == "origin" && origin.pos == Vec2::ZERO && origin.fixed && origin.link.is_none() && origin.exprs.is_none(), || {
            "sheet sketch point zero must be its fixed local origin".into()
        })?;
        let mut seen = BTreeSet::new();
        for point in &sk.points {
            identity(&mut seen, &point.id)?;
            position(point.pos)?;
            require(point.link.is_none(), || "not supported yet: linked sheet sketch point".into())?;
            if let Some(expressions) = &point.exprs {
                for expression in [&expressions.x, &expressions.y].into_iter().flatten() {
                    text(expression)?;
                }
            }
        }
        for curve in &sk.curves {
            identity(&mut seen, &curve.id)?;
            require(curve.link.is_none(), || "not supported yet: linked sheet sketch curve".into())?;
            if let CurveKind::Spline { pts, degree, control, .. } = &curve.kind {
                require(
                    (2..=MAX_SPLINE_POINTS).contains(&pts.len()) && (1..=5).contains(degree) && (!control || pts.len() > usize::from(*degree)),
                    || "invalid sheet sketch spline size or degree".into(),
                )?;
                charge(&mut entities, pts.len(), MAX_ENTITIES, "drawing sketch entity")?;
            }
            require(curve.kind.point_ids().iter().all(|&i| i < sk.points.len()), || "sheet sketch curve references an unknown point".into())?;
            match curve.kind {
                CurveKind::Line { a, b } => require(a != b, || "sheet sketch line needs distinct point identities".into())?,
                CurveKind::Circle { r, .. } | CurveKind::Ellipse { r, .. } => {
                    require(r.is_finite() && r > 0.0 && r < MAX_COORD, || "sheet sketch radius is out of range".into())?;
                }
                CurveKind::Conic { rho, .. } => {
                    require(rho.is_finite() && rho > 0.0 && rho < 1.0, || "sheet sketch conic rho is out of range".into())?
                }
                _ => {}
            }
            require(!curve.infinite || (curve.construction && matches!(curve.kind, CurveKind::Line { .. })), || {
                "only a construction line can be infinite in a sheet sketch".into()
            })?;
        }
        for constraint in &sk.constraints {
            identity(&mut seen, &constraint.id)?;
            // Slot validation can search the constraint list twice. Charge that upper
            // bound before invoking the shared validator, rather than allowing O(n^3).
            let cost = if matches!(constraint.kind, ConstraintKind::SlotOverallLength { .. }) {
                sk.constraints
                    .len()
                    .checked_mul(sk.constraints.len())
                    .and_then(|n| n.checked_mul(4))
                    .ok_or_else(|| invalid("validation work overflow"))?
            } else {
                1
            };
            charge(&mut work, cost, MAX_VALIDATION_WORK, "validation work")?;
            sk.validate(&constraint.kind).map_err(|e| invalid(e.to_string()))?;
            if let Some(value) = &constraint.param {
                text(value)?;
            }
            if let Some(at) = constraint.text {
                position(at)?;
            }
        }
        sk.validate_periodic_splines().map_err(|e| invalid(e.to_string()))?;
    }
    Ok(())
}

/// Sort references, not the live geometry. Sheet order stays authored; sketch
/// record order depends only on ids, so insertion order never changes saved bytes.
pub(super) fn serialize_sorted<S: Serializer>(records: &[Arc<SheetSketch>], serializer: S) -> std::result::Result<S::Ok, S::Error> {
    if records.len() > MAX_SHEET_SKETCHES {
        return Err(serde::ser::Error::custom("too many sheet sketches"));
    }
    let mut sorted: Vec<_> = records.iter().collect();
    sorted.sort_by_key(|record| record.id);
    sorted.serialize(serializer)
}
