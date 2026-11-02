//! Contracts and result types for `drawing.scene` (`cmd/drawing_scene.rs`, #4890).

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use solidcraft_doc::ids;
use solidcraft_geom::paper::ObjectKind;
use solidcraft_geom::{MAX_BEZIER_CTRL, MAX_BEZIER_DEGREE, Seg2};

use super::drawing_sheet::sheet_ref;
use super::drawing_view::{LinesStatsResult, PathSegResult, paper_scene_schema};
use crate::drawing_scene::{SketchState, SketchStatus, Status, ViewState};

/// Draw one sheet of the open drawing. Read-only.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct SheetSceneParams {
    /// The sheet, by id or name.
    #[serde(deserialize_with = "sheet_ref")]
    pub sheet: String,
}

/// A sheet's paper scene: its size and its groups in paint order, in paper millimetres from the
/// sheet's lower-left corner: the drawing border's (object: the sheet), then each view's strokes
/// (object: the view), then sheet sketches (object: the sketch).
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct SheetPaperScene(pub solidcraft_geom::paper::Scene);

impl JsonSchema for SheetPaperScene {
    fn schema_name() -> Cow<'static, str> {
        "SheetPaperScene".into()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        paper_scene_schema::<SheetPathSegResult>(generator, &[ObjectKind::DrawingBorder, ObjectKind::View, ObjectKind::Sketch], true)
    }
}

/// A sheet's exact paper geometry, including freeforms independently of writer support.
/// Projected-view paths retain their separate line/arc-only `PathSegResult` contract.
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct SheetPathSegResult(pub Seg2);

impl JsonSchema for SheetPathSegResult {
    fn schema_name() -> Cow<'static, str> {
        "SheetPaperSegment".into()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        let projected = generator.subschema_for::<PathSegResult>();
        let point = json!({"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2});
        json_schema!({
            "anyOf": [
                projected,
                {"type": "object", "properties": {"type": {"const": "cubic"}, "p0": point, "p1": point, "p2": point, "p3": point}, "required": ["type", "p0", "p1", "p2", "p3"], "additionalProperties": false},
                {"type": "object", "properties": {"type": {"const": "conic"}, "a": point, "apex": point, "b": point, "w": {"type": "number", "exclusiveMinimum": 0}}, "required": ["type", "a", "apex", "b", "w"], "additionalProperties": false},
                {"type": "object", "properties": {"type": {"const": "bezier"}, "n": {"type": "integer", "minimum": 4, "maximum": MAX_BEZIER_DEGREE}, "p": {"type": "array", "items": point, "minItems": MAX_BEZIER_CTRL, "maxItems": MAX_BEZIER_CTRL}}, "required": ["type", "n", "p"], "additionalProperties": false}
            ]
        })
    }
}

/// One view on the sheet: `drawn`, `placeholder` (nothing to measure, so no lines) or `failed`,
/// with why.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SheetSceneView {
    pub view: String,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// What working out its lines cost, when they were.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<LinesStatsResult>,
}

impl From<ViewState> for SheetSceneView {
    fn from(v: ViewState) -> Self {
        let status = v.status.name();
        let reason = match v.status {
            Status::Drawn => None,
            Status::Placeholder(why) => Some(why.to_string()),
            Status::Failed(why) => Some(why),
        };
        SheetSceneView { view: ids::encode(v.view), status, reason, stats: v.stats.map(Into::into) }
    }
}

/// One sheet sketch, in id order. `drawn` admits exact scene geometry; a selected writer can
/// still refuse that geometry. A failed sketch draws nothing and reports why.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct SheetSceneSketch {
    pub sketch: String,
    #[schemars(schema_with = "sketch_status_schema")]
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

fn sketch_status_schema(_: &mut SchemaGenerator) -> Schema {
    json_schema!({"type": "string", "enum": ["drawn", "failed"]})
}

impl From<SketchState> for SheetSceneSketch {
    fn from(sketch: SketchState) -> Self {
        let status = sketch.status.name();
        let reason = match sketch.status {
            SketchStatus::Drawn => None,
            SketchStatus::Failed(why) => Some(why),
        };
        Self { sketch: ids::encode(sketch.sketch), status, reason }
    }
}

/// What `drawing.scene` returns.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SheetSceneResult {
    pub sheet: String,
    pub scene: SheetPaperScene,
    /// Each view on the sheet, in id order.
    pub views: Vec<SheetSceneView>,
    /// Each sheet sketch on the sheet, in id order.
    pub sketches: Vec<SheetSceneSketch>,
    /// In the warn parameter-check mode: what the call's parameters got wrong.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub param_warnings: Vec<String>,
}
