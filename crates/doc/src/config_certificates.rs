//! Certificates are producer identities, never a nearest-surface selection.
use crate::{DocError, Document, FeatureKind, ModelBody, Result, SolvedSketch};
use serde::{Deserialize, Serialize};
use solvecraft_geom::Seg2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FaceCertificate {
    ExtrudeSide { sketch: u64, curve: String },
    Imported { payload_sha256: String, item: u64, face: u64 },
}

impl FaceCertificate {
    pub fn remap_features(&mut self, features: &std::collections::BTreeMap<u64, u64>) {
        if let Self::ExtrudeSide { sketch, .. } = self
            && let Some(mapped) = features.get(sketch)
        {
            *sketch = *mapped;
        }
    }
    pub fn validate(&self) -> Result<()> {
        let valid = match self {
            Self::ExtrudeSide { sketch, curve } => *sketch != 0 && !curve.is_empty() && curve.len() <= 128,
            Self::Imported { payload_sha256, item, face } => {
                *item != 0
                    && *face != 0
                    && payload_sha256.len() == 64
                    && payload_sha256.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            }
        };
        if valid { Ok(()) } else { Err(DocError::Invalid("invalid face source certificate".into())) }
    }
}

pub(crate) fn validate_certificate_json(value: &serde_json::Value) -> Result<()> {
    if value.is_null() {
        return Ok(());
    }
    let fields = value.as_object().ok_or_else(|| DocError::Invalid("face certificate must be an object".into()))?;
    let nonzero = |field: &str| fields.get(field).and_then(serde_json::Value::as_u64).is_some_and(|id| id != 0);
    let valid = match fields.get("kind").and_then(serde_json::Value::as_str) {
        Some("extrude_side") => {
            fields.len() == 3
                && fields.keys().all(|key| ["kind", "sketch", "curve"].contains(&key.as_str()))
                && nonzero("sketch")
                && fields.get("curve").and_then(serde_json::Value::as_str).is_some_and(|curve| !curve.is_empty() && curve.len() <= 128)
        }
        Some("imported") => {
            fields.len() == 4
                && fields.keys().all(|key| ["kind", "payload_sha256", "item", "face"].contains(&key.as_str()))
                && nonzero("item")
                && nonzero("face")
                && fields
                    .get("payload_sha256")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        }
        _ => false,
    };
    if valid { Ok(()) } else { Err(DocError::Invalid("malformed or oversized face source certificate".into())) }
}

/// Match a generated segment to one exact analytic sketch curve. Coincident
/// candidates are ambiguous and never resolved by distance or candidate order.
pub(crate) fn exact_sketch_curve(sketch: &SolvedSketch, generated: &Seg2) -> Option<String> {
    if sketch.sketch.curves.len() > 2000 {
        return None;
    }
    let mut found = None;
    for (index, curve) in sketch.sketch.curves.iter().enumerate() {
        if curve.construction
            || !matches!(
                curve.kind,
                solvecraft_sketch::CurveKind::Line { .. } | solvecraft_sketch::CurveKind::Arc { .. } | solvecraft_sketch::CurveKind::Circle { .. }
            )
        {
            continue;
        }
        let matches = if let solvecraft_sketch::CurveKind::Circle { c, r } = curve.kind {
            sketch
                .sketch
                .point(c)
                .is_some_and(|center| exact_segment_contains(&Seg2::Arc { center, radius: r, start: 0., sweep: std::f64::consts::TAU }, generated))
        } else {
            sketch.sketch.segs(index).iter().any(|original| exact_segment_contains(original, generated))
        };
        if matches {
            if found.is_some() {
                return None;
            }
            found = Some(curve.id.clone());
        }
    }
    found
}

fn exact_segment_contains(original: &Seg2, generated: &Seg2) -> bool {
    match (original, generated) {
        (Seg2::Line { a, b }, Seg2::Line { a: c, b: d }) => {
            let direction = *b - *a;
            let scale = direction.len().max((*d - *c).len()).max(1.);
            let interval = |point: solvecraft_geom::Vec2| {
                let t = (point - *a).dot(direction) / direction.len2();
                (-1e-10..=1. + 1e-10).contains(&t)
            };
            direction.len() > 1e-12
                && direction.cross(*c - *a).abs() <= 1e-10 * scale * scale
                && direction.cross(*d - *a).abs() <= 1e-10 * scale * scale
                && interval(*c)
                && interval(*d)
        }
        (Seg2::Arc { center: a, radius: ar, start: os, sweep: ow }, Seg2::Arc { center: b, radius: br, start: gs, sweep: gw }) => {
            let tau = std::f64::consts::TAU;
            let tolerance = 1e-10;
            let length = ow.abs();
            let generated_length = gw.abs();
            if a.dist(*b) > 1e-10 * ar.max(*br).max(1.)
                || (ar - br).abs() > 1e-10 * ar.max(*br).max(1.)
                || length > tau + tolerance
                || generated_length > length + tolerance
            {
                return false;
            }
            if length >= tau - tolerance {
                return true;
            }
            let original_low = if *ow >= 0. { *os } else { os + ow };
            let generated_low = if *gw >= 0. { *gs } else { gs + gw };
            let offset = (generated_low - original_low).rem_euclid(tau);
            let offset = if offset > tau - tolerance { 0. } else { offset };
            offset + generated_length <= length + tolerance
        }
        _ => false,
    }
}

impl Document {
    pub fn configuration_face_target(&self, body: &ModelBody, index: usize, point: solvecraft_geom::Vec3) -> Result<crate::config::ConfigFaceTarget> {
        let names = crate::naming::face_names(body);
        let mut name = names
            .get(index)
            .map(|name| crate::naming::strip_piece(name).to_string())
            .ok_or_else(|| DocError::Unknown("face source name missing".into()))?;
        let origin = match body.body.face_sources().and_then(|sources| sources.get(index)).and_then(Option::as_ref) {
            Some(solvecraft_kernel::FaceSource::Imported { producer: Some(producer), .. })
            | Some(solvecraft_kernel::FaceSource::CertifiedExtrudeSide { producer, .. }) => *producer,
            _ => name
                .strip_prefix('F')
                .and_then(|name| name.split_once(':'))
                .and_then(|(id, _)| id.parse().ok())
                .ok_or_else(|| DocError::Invalid("face source producer missing".into()))?,
        };
        let certificate = self.configuration_face_certificate(body, index)?;
        if let Some(FaceCertificate::ExtrudeSide { curve, .. }) = &certificate {
            name = format!("F{origin}:side:{curve}");
        } else if !name.starts_with(&format!("F{origin}:")) {
            name = format!("F{origin}:face:{index}");
        }
        let inverse = crate::mat_inverse(&body.source_transform).ok_or_else(|| DocError::Invalid("invalid definition placement".into()))?;
        Ok(crate::config::ConfigFaceTarget {
            body: body.definition_name().into(),
            origin_feature: origin,
            name,
            point: crate::apply_point(&inverse, point),
            certificate,
        })
    }
    pub fn configuration_face_certificate(&self, body: &ModelBody, index: usize) -> Result<Option<FaceCertificate>> {
        let Some(source) = body.body.face_sources().and_then(|sources| sources.get(index)).and_then(Option::as_ref) else {
            return Ok(None);
        };
        if let solvecraft_kernel::FaceSource::Imported { item, face, payload_sha256: Some(digest), producer: Some(producer) } = source {
            if !self.feature(*producer).is_some_and(|feature| matches!(feature.kind, FeatureKind::Import { .. })) {
                return Err(DocError::Invalid("import source producer missing".into()));
            }
            let certificate = FaceCertificate::Imported { payload_sha256: digest.clone(), item: *item, face: *face };
            certificate.validate()?;
            return Ok(Some(certificate));
        }
        let certificate = match source {
            solvecraft_kernel::FaceSource::CertifiedExtrudeSide { producer, sketch, curve } => {
                if !self.feature(*producer).is_some_and(|feature| matches!(feature.kind, FeatureKind::Extrude { .. })) {
                    return Err(DocError::Invalid("side source producer missing".into()));
                }
                FaceCertificate::ExtrudeSide { sketch: *sketch, curve: curve.clone() }
            }
            _ => return Err(DocError::Invalid("face source does not match producer".into())),
        };
        certificate.validate()?;
        Ok(Some(certificate))
    }
}

pub(crate) fn payload_digest(payload: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(payload.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn certificate_budgets_apply_before_saved_target_and_paint_decode() {
        for certificate in [
            json!({"kind":"extrude_side","sketch":1,"curve":"x".repeat(1_000_000)}),
            json!({"kind":"imported","payload_sha256":"0".repeat(64),"item":0,"face":1}),
            json!({"kind":"imported","payload_sha256":"g".repeat(64),"item":1,"face":1}),
            json!({"kind":"extrude_side","sketch":1,"curve":"c1","extra":[[[[[]]]]]}),
        ] {
            assert!(validate_certificate_json(&certificate).is_err());
            let target = json!({"configs":{"columns":[{"face_appearance":{"body":"B","origin_feature":1,"name":"F1:side:c1","point":[0,0,0],"certificate":certificate}}]}});
            assert!(crate::config::validate_styles_json(&target).is_err());
            let paint = json!({"appearances":{"faces":[{"certificate":certificate}]}});
            assert!(crate::config::validate_styles_json(&paint).is_err());
        }
        assert!(validate_certificate_json(&json!({"kind":"imported","payload_sha256":"a".repeat(64),"item":1,"face":2})).is_ok());
        assert!(validate_certificate_json(&json!({"kind":"extrude_side","sketch":1,"curve":"c1"})).is_ok());
    }
    #[test]
    fn analytic_arc_interval_refuses_unsampled_gaps_and_accepts_wrap_reversal() {
        let radians = |degrees: f64| degrees.to_radians();
        let arc = |start, sweep| Seg2::Arc { center: solvecraft_geom::Vec2::default(), radius: 10., start: radians(start), sweep: radians(sweep) };
        assert!(!exact_segment_contains(&arc(10., 350.), &arc(20., 360.)), "five point probes missed the missing ten-degree gap");
        assert!(!exact_segment_contains(&arc(10., 350.), &arc(20., -360.)));
        assert!(exact_segment_contains(&arc(350., 40.), &arc(355., 20.)));
        assert!(exact_segment_contains(&arc(350., 40.), &arc(15., -20.)));
        assert!(exact_segment_contains(&arc(30., -40.), &arc(15., -20.)));
        assert!(!exact_segment_contains(&arc(350., 40.), &arc(345., 20.)));
        assert!(exact_segment_contains(&arc(0., 360.), &arc(20., -360.)));
    }
}
