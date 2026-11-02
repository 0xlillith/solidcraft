//! The sheet scene (#4890 W10, #3890 W15): everything drawn on one sheet of a drawing as one paper
//! scene ([`Scene`]), for the sheet page and every export, so the screen and the files draw the
//! same thing. In paint order: the sheet's drawing border ([`SheetLayout::paper`]), then each view
//! on the sheet by id, its visible lines as [`paper_group`] makes them, then exact sheet sketches
//! by id. Text, the title block, tables, hidden lines and shaded views each join as one more kind
//! of group.
//!
//! [`build`] draws a sheet at once, each view the one way: its lines worked out centred on the view,
//! then moved to its centre, so the same lines on the same centre are the same bytes however they
//! got there. It reads the model as it is: no model is evaluated.
//!
//! The sheet page draws through the provider instead, which keeps view and sketch groups between
//! frames: [`extract`] returns the last scene and separate stale lists, and [`step`] brings one view
//! or sketch up to date, a short piece of work for the UI's tick. A view moved across its sheet is
//! moved, never projected again. The provider evaluates no model either. Its cache is the
//! session's view state, beside the drawing: never saved, never an undo step, and dropped when its
//! drawing closes.
//!
//! [`SheetLayout::paper`]: solidcraft_geom::drawing_sheet::SheetLayout::paper

use std::borrow::Borrow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use solidcraft_doc::drawing::view::{ResolvedView, ViewPlace};
use solidcraft_doc::drawing::{Drawing, Sheet, SheetSketch};
use solidcraft_doc::drawing_view::TangentEdges;
use solidcraft_doc::{DocError, Document, ModelState, expr::ParamDef, ids};
use solidcraft_geom::drawing_view::is_paper_coordinate;
use solidcraft_geom::paper::text::TextRun;
use solidcraft_geom::paper::{Group, Item, MAX_RECORDS, Picture, Scene};
use solidcraft_geom::{Aabb2, Loop2, Region2, Seg2, Vec2};

use crate::drawing_view::{CHORD_MM, LineRequest, MIN_MODEL_CHORD_MM, ShownSet, Stats, Unmeasured, VISIBLE_MM, paper_group};
use crate::params::bad;
use crate::{EngineError, Result, Session};

pub mod sketch;

/// The command the sheet scene answers for, in its errors.
pub const CMD: &str = "drawing.scene";

/// Numbers every scene the provider assembles, in every session, so a page never sees one number
/// for two scenes, not even across closing and opening its drawing.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// What became of one view of a sheet scene.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// Its visible lines are in the scene.
    Drawn,
    /// It has nothing to measure, so it draws nothing: the sheet page shows its placeholder.
    Placeholder(Unmeasured),
    /// It can't be drawn yet, or its lines are more than a paper scene holds: why. The sheet's
    /// other views are drawn all the same.
    Failed(String),
}

impl Status {
    /// Its name in replies.
    pub fn name(&self) -> &'static str {
        match self {
            Status::Drawn => "drawn",
            Status::Placeholder(_) => "placeholder",
            Status::Failed(_) => "failed",
        }
    }
}

/// One view of a sheet scene.
#[derive(Clone, Debug)]
pub struct ViewState {
    pub view: u64,
    pub status: Status,
    /// What working out its lines cost, when they were.
    pub stats: Option<Stats>,
}

/// What became of one authored sheet sketch. Writer support is checked separately.
#[derive(Clone, Debug, PartialEq)]
pub enum SketchStatus {
    Drawn,
    Failed(String),
}

impl SketchStatus {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Drawn => "drawn",
            Self::Failed(_) => "failed",
        }
    }
}

/// One sheet sketch, independently of projected-view status.
#[derive(Clone, Debug)]
pub struct SketchState {
    pub sketch: u64,
    pub status: SketchStatus,
}

/// One sheet drawn at once ([`build`]).
#[derive(Clone, Debug)]
pub struct SheetScene {
    pub scene: Scene,
    /// Each view on the sheet, in id order.
    pub views: Vec<ViewState>,
    /// Each sketch on the sheet, in id order.
    pub sketches: Vec<SketchState>,
}

/// One sheet as the provider last drew it ([`extract`]).
#[derive(Clone, Debug)]
pub struct SheetView {
    pub scene: Arc<Scene>,
    /// Goes up each time `scene` changes, so the page redraws only then.
    pub generation: u64,
    /// Each view on the sheet drawn so far, in id order, as it was last drawn.
    pub views: Vec<ViewState>,
    /// The views out of date, in id order: not drawn yet, or drawn from another model or with other
    /// settings than they have now. Their last lines stay in the scene until [`step`] draws them.
    pub stale: Vec<u64>,
    /// Each sketch drawn so far, in id order, as it was last solved and converted.
    pub sketches: Vec<SketchState>,
    /// Sketches not solved from their current stored records and source definitions yet.
    pub stale_sketches: Vec<u64>,
}

/// The id of the sheet `named` names in `drawing` (its id or its one name), refused as `cmd`: the
/// sheet a command asks to draw.
pub fn sheet_id(drawing: &Drawing, named: &str, cmd: &str) -> Result<u64> {
    let at = drawing.sheet_index(named).map_err(|e| bad(cmd, e.to_string()))?;
    drawing.sheets.get(at).map(|sheet| sheet.id).ok_or_else(|| bad(cmd, format!("no sheet `{named}`")))
}

/// Sheet `sheet` of `drawing`, one of `s`'s design's drawings, drawn at once: its border, then each
/// of its views, then each sketch, by id. An object that can't be drawn is left out and says why;
/// the sheet is refused only when it can't be drawn at all.
pub fn build(s: &Session, drawing: &Drawing, sheet: u64) -> Result<SheetScene> {
    let record = sheet_of(drawing, sheet)?;
    let on = views_on(drawing, sheet)?;
    // Worked out once for the sheet: every view draws from the same one. Each view's lines are
    // placed as they are drawn, so at most one view's are held beside the scene.
    let shown = crate::drawing_view::shown(s);
    let drawn = on.iter().map(|(id, view, centre)| (*id, *centre, draw(s, &shown, *id, view)));
    let sketches = sketches_on(drawing, sheet);
    let sketched = sketches.iter().map(|record| (record.id, draw_sketch(s, drawing, record)));
    let (scene, views, sketches) = assemble_with_sketches(record, drawn, sketched)?;
    Ok(SheetScene { scene, views, sketches })
}

#[derive(Clone, Debug)]
struct DrawnSketch {
    group: Option<Group>,
    status: SketchStatus,
}

/// Solve a read-only copy, then convert exact authored geometry in physical paper mm.
fn draw_sketch(s: &Session, drawing: &Drawing, record: &SheetSketch) -> DrawnSketch {
    let id = record.id;
    let result =
        drawing.solve_sheet_sketch(&s.doc, id).map_err(|e| bad(CMD, e.to_string())).and_then(|solved| sketch::paper_group(&solved, VISIBLE_MM));
    match result {
        Ok(group) => DrawnSketch { group: Some(group), status: SketchStatus::Drawn },
        Err(e) => DrawnSketch {
            group: None,
            status: SketchStatus::Failed(format!(
                "drawing {} sheet {} sketch {}: {}",
                ids::encode(drawing.id),
                ids::encode(record.sheet),
                ids::encode(id),
                reason(e)
            )),
        },
    }
}

/// A view's lines for a sheet scene, centred on the view, or why it has none.
#[derive(Clone, Debug)]
struct Drawn {
    group: Option<Group>,
    status: Status,
    stats: Option<Stats>,
}

/// View `id`, drawing as `view` does, of the bodies `shown` holds: its visible lines, sampled at
/// [`CHORD_MM`] on paper, or coarser past 100:1, where that would ask the mesh for chords finer than
/// [`MIN_MODEL_CHORD_MM`]; centred on the view ([`assemble`] moves them to its centre). Lines past
/// the paper scene's [`MAX_RECORDS`] are refused here, so no view holds more than a scene takes.
fn draw(s: &Session, shown: &std::result::Result<ShownSet, Unmeasured>, id: u64, view: &ResolvedView) -> Drawn {
    let none = |status| Drawn { group: None, status, stats: None };
    // A style that draws hidden lines is refused here too, so the lines below are visible ones.
    if let Some(why) = crate::drawing_view::not_drawable(view.display_style.value, view.reference_style.value, true) {
        return none(Status::Failed(why.into()));
    }
    // Every body the design places, until saved views have `source` and `hide` (V4) to keep a part.
    let measured = shown.as_ref().map_err(|why| *why).and_then(|set| crate::drawing_view::measure(set, &view.basis).map(|e| (set, e)));
    // Its reference parts draw too, around what the extents measure.
    let (set, extents) = match measured {
        Ok(measured) => measured,
        Err(why) => return none(Status::Placeholder(why)),
    };
    let around = crate::drawing_view::shows(set, &view.basis, &extents, view.reference_shown_percent);
    let tangent_edges = view.tangent_edges.value == TangentEdges::Show;
    // The margin keeps the mesh's chord at or above its floor once divided back by the factor.
    let chord_mm = CHORD_MM.max(MIN_MODEL_CHORD_MM * view.scale.value.factor() * (1.0 + 1e-9));
    let reference_hides_model = view.reference_hides_model.value;
    let request = LineRequest { shows: around, hidden: false, reference_hidden: false, reference_hides_model, tangent_edges, chord_mm };
    let lines = crate::drawing_view::frame(view.basis, view.scale.value, &extents.model, Vec2::ZERO, CMD)
        .and_then(|frame| crate::drawing_view::lines(s, set, &frame, &request, CMD));
    match lines {
        Ok(lines) => {
            let group = paper_group(&lines, id);
            if group.records() > MAX_RECORDS {
                return none(Status::Failed(format!("its lines are more than a paper scene's {MAX_RECORDS} records: choose a smaller scale")));
            }
            Drawn { group: Some(group), status: Status::Drawn, stats: Some(lines.stats) }
        }
        Err(e) => none(Status::Failed(reason(e))),
    }
}

/// The scene of sheet `record`: its border, then each of `views` (its id, centre and lines, in id
/// order) moved to its centre. A view that would take the scene past a paper scene's limits is left
/// out and says why, and the views after it are still drawn when they fit.
#[cfg(test)]
fn assemble<D: Borrow<Drawn>>(record: &Sheet, views: impl Iterator<Item = (u64, Vec2, D)>) -> Result<(Scene, Vec<ViewState>)> {
    let (scene, views, _) = assemble_with_sketches(record, views, std::iter::empty::<(u64, DrawnSketch)>())?;
    Ok((scene, views))
}

fn assemble_with_sketches<D: Borrow<Drawn>, K: Borrow<DrawnSketch>>(
    record: &Sheet,
    views: impl Iterator<Item = (u64, Vec2, D)>,
    sketches: impl Iterator<Item = (u64, K)>,
) -> Result<(Scene, Vec<ViewState>, Vec<SketchState>)> {
    let size_mm = record.sheet.size_mm().map_err(|e| bad(CMD, e.to_string()))?;
    let border = match &record.border {
        Some(border) => border.layout(&record.sheet).map_err(|e| bad(CMD, e.to_string()))?.paper(record.id),
        None => Vec::new(),
    };
    let mut records = border.iter().map(Group::records).fold(0usize, usize::saturating_add);
    let mut scene = Scene::new(size_mm, border);
    // Each view's group is counted onto the scene and checked alone on the sheet first, so one past
    // the limits leaves out only itself, and checking costs one pass over each group.
    let mut alone = Scene::new(size_mm, Vec::with_capacity(1));
    let mut states = Vec::new();
    for (view, centre, drawn) in views {
        let drawn: &Drawn = drawn.borrow();
        let mut status = drawn.status.clone();
        if let Some(group) = &drawn.group {
            let more = group.records();
            if records.saturating_add(more) > MAX_RECORDS {
                status = Status::Failed(format!(
                    "with its lines the sheet is more than a paper scene's {MAX_RECORDS} records: put it on another sheet, or choose a smaller scale"
                ));
            } else {
                alone.groups.push(moved(group, centre));
                match alone.validate() {
                    Ok(()) => {
                        records += more;
                        scene.groups.append(&mut alone.groups);
                    }
                    Err(e) => {
                        alone.groups.clear();
                        status = Status::Failed(format!("{e}: choose a smaller scale"));
                    }
                }
            }
        }
        states.push(ViewState { view, status, stats: drawn.stats });
    }
    let mut sketch_states = Vec::new();
    for (sketch, drawn) in sketches {
        let drawn = drawn.borrow();
        let mut status = drawn.status.clone();
        if let Some(group) = &drawn.group {
            let more = group.records();
            if records.saturating_add(more) > MAX_RECORDS {
                status = SketchStatus::Failed(format!(
                    "sketch {}: with its strokes the sheet exceeds a paper scene's {MAX_RECORDS} records",
                    ids::encode(sketch)
                ));
            } else {
                // Sketches already use sheet coordinates: no view scale or translation.
                alone.groups.push(group.clone());
                match alone.validate() {
                    Ok(()) => {
                        records += more;
                        scene.groups.append(&mut alone.groups);
                    }
                    Err(e) => {
                        alone.groups.clear();
                        status = SketchStatus::Failed(format!("sketch {}: {e}", ids::encode(sketch)));
                    }
                }
            }
        }
        sketch_states.push(SketchState { sketch, status });
    }
    scene.validate().map_err(|e| bad(CMD, e.to_string()))?;
    Ok((scene, states, sketch_states))
}

/// `group` moved `by` on paper: each point plus `by`, exactly, so the same lines moved to the same
/// centre are the same bytes however they got there.
fn moved(group: &Group, by: Vec2) -> Group {
    let segs = |path: &[Seg2]| -> Vec<Seg2> {
        let shift = |seg: &Seg2| match *seg {
            // An arc's centre moves; its radius and its angles stay.
            Seg2::Arc { center, radius, start, sweep } => Seg2::Arc { center: center + by, radius, start, sweep },
            _ => seg.map_controls(|p| p + by),
        };
        path.iter().map(shift).collect()
    };
    let ring = |l: &Loop2| Loop2 { segs: segs(&l.segs) };
    let items = group.items.iter().map(|item| match item {
        Item::Stroke { path, closed, width_mm } => Item::Stroke { path: segs(path), closed: *closed, width_mm: *width_mm },
        Item::Fill { region } => Item::Fill { region: Region2 { outer: ring(&region.outer), holes: region.holes.iter().map(ring).collect() } },
        Item::Text(run) => Item::Text(TextRun { origin: run.origin + by, ..run.clone() }),
        Item::Picture(picture) => Item::Picture(Picture { rect: Aabb2 { min: picture.rect.min + by, max: picture.rect.max + by }, ..*picture }),
    });
    Group { object: group.object, kind: group.kind, items: items.collect() }
}

/// Sheet `sheet` of `d`.
fn sheet_of(d: &Drawing, sheet: u64) -> Result<&Sheet> {
    d.sheets.iter().map(|s| &**s).find(|s| s.id == sheet).ok_or_else(|| bad(CMD, format!("the drawing has no sheet {}", ids::encode(sheet))))
}

/// The views on sheet `sheet` of `d`, in id order, each as it draws and with its centre.
fn views_on(d: &Drawing, sheet: u64) -> Result<Vec<(u64, ResolvedView, Vec2)>> {
    let fail = |e: DocError| bad(CMD, e.to_string());
    let mut on: Vec<u64> = d.views.iter().filter(|v| v.sheet == sheet).map(|v| v.id).collect();
    on.sort_unstable();
    on.into_iter()
        .map(|id| -> Result<(u64, ResolvedView, Vec2)> { Ok((id, d.resolve_view(id).map_err(fail)?, d.view_centre(id).map_err(fail)?)) })
        .collect()
}

fn sketches_on(d: &Drawing, sheet: u64) -> Vec<&Arc<SheetSketch>> {
    let mut on: Vec<_> = d.sketches.iter().filter(|record| record.sheet == sheet).collect();
    on.sort_unstable_by_key(|record| record.id);
    on
}

/// `e`'s message without the command's name: a view's reason in a sheet scene.
fn reason(e: EngineError) -> String {
    match e {
        EngineError::BadParams { msg, .. } => msg,
        e => e.to_string(),
    }
}

/// The provider's cache: per view or sketch, its last drawing. The session holds one, for
/// the drawing open.
#[derive(Default)]
pub struct Cache {
    /// The drawing it is for.
    drawing: Option<u64>,
    /// The sheet last extracted: the one [`step`] brings up to date.
    sheet: Option<u64>,
    /// Each view's last drawing, by id.
    views: BTreeMap<u64, Entry>,
    sketches: BTreeMap<u64, SketchEntry>,
    /// Metadata only: refresh once per document Arc, without evaluating expressions.
    source: Option<(Arc<Document>, Arc<SourceInputs>)>,
    context: Option<(Arc<Drawing>, Arc<SketchContext>)>,
    /// The shown set of the world state views were last drawn from, worked out once for it.
    shown: Option<(Arc<ModelState>, std::result::Result<ShownSet, Unmeasured>)>,
    last: Option<Assembled>,
    /// Numbers each work result, so a scene is assembled again only when its contents change.
    serial: u64,
    /// Views drawn, for tests: a move draws none.
    #[cfg(test)]
    projections: usize,
    #[cfg(test)]
    sketch_solves: usize,
}

#[derive(PartialEq)]
struct SourceInputs {
    units: String,
    defs: Vec<ParamDef>,
}

/// All sketch records participate in the document's cumulative solve-admission budgets.
/// Share this context across entries; view moves do not change it. A record edit conservatively
/// invalidates the sheet's solves, including cached failures caused by another record's budget.
struct SketchContext {
    drawing: Arc<Drawing>,
}

/// Solving admits the complete drawing's structure and cumulative sketch budgets. Every such
/// input participates, even when it belongs to another sheet. A valid numeric move of an otherwise
/// unchanged view cannot affect admission: keep its context and avoid solving sketches on a move.
fn same_sketch_context(was: &Drawing, now: &Drawing) -> bool {
    if was.sketches.len() != now.sketches.len()
        || !was.sketches.iter().zip(&now.sketches).all(|(a, b)| Arc::ptr_eq(a, b))
        || was.views.len() != now.views.len()
    {
        return false;
    }
    let valid = |place| match place {
        ViewPlace::At([x, y]) => is_paper_coordinate(x) && is_paper_coordinate(y),
        ViewPlace::Aligned(along) => is_paper_coordinate(along),
    };
    for (a, b) in was.views.iter().zip(&now.views) {
        if Arc::ptr_eq(a, b) {
            continue;
        }
        if std::mem::discriminant(&a.place) != std::mem::discriminant(&b.place) || !valid(a.place) || !valid(b.place) {
            return false;
        }
        let mut unmoved = (**b).clone();
        unmoved.place = a.place;
        if **a != unmoved {
            return false;
        }
    }
    let mut unmoved = now.clone();
    unmoved.views = was.views.clone();
    *was == unmoved
}

struct SketchEntry {
    source: Arc<SourceInputs>,
    context: Arc<SketchContext>,
    drawn: DrawnSketch,
    serial: u64,
}

/// A view's last drawing, and what it was drawn from: the world state (held, so its address can't
/// be reused while it is compared) and the view as it drew.
struct Entry {
    state: Arc<ModelState>,
    view: ResolvedView,
    drawn: Drawn,
    serial: u64,
}

/// What a scene is assembled from: the sheet's record, and each view on it with its entry's serial
/// (0: not drawn yet) and its centre.
#[derive(PartialEq)]
struct Inputs {
    sheet: Sheet,
    views: Vec<(u64, u64, Vec2)>,
    sketches: Vec<(u64, u64)>,
}

/// The last answer of [`extract`], including its source document identity, so source-only edits
/// cannot bypass stale-sketch detection.
struct Assembled {
    sheet: u64,
    drawing: Arc<Drawing>,
    state: Arc<ModelState>,
    doc: Arc<Document>,
    serial: u64,
    inputs: Inputs,
    answer: SheetView,
}

impl Cache {
    /// Whether view `id`, drawing as `view` now, was last drawn so, from `state`.
    fn fresh(&self, id: u64, state: &Arc<ModelState>, view: &ResolvedView) -> bool {
        self.views.get(&id).is_some_and(|e| Arc::ptr_eq(&e.state, state) && e.view == *view)
    }

    fn fresh_sketch(&self, id: u64, source: &Arc<SourceInputs>, context: &Arc<SketchContext>) -> bool {
        self.sketches.get(&id).is_some_and(|e| Arc::ptr_eq(&e.source, source) && Arc::ptr_eq(&e.context, context))
    }

    fn sketch_inputs(&mut self, doc: &Arc<Document>, drawing: &Arc<Drawing>) -> (Arc<SourceInputs>, Arc<SketchContext>) {
        let source = match &self.source {
            Some((was, source)) if Arc::ptr_eq(was, doc) => source.clone(),
            _ => {
                let next = SourceInputs { units: doc.units.clone(), defs: doc.param_defs() };
                match &self.source {
                    Some((_, source)) if **source == next => source.clone(),
                    _ => Arc::new(next),
                }
            }
        };
        self.source = Some((doc.clone(), source.clone()));
        let context = match &self.context {
            Some((was, context)) if Arc::ptr_eq(was, drawing) || same_sketch_context(&context.drawing, drawing) => context.clone(),
            _ => Arc::new(SketchContext { drawing: drawing.clone() }),
        };
        self.context = Some((drawing.clone(), context.clone()));
        (source, context)
    }
}

/// Sheet `sheet` of the drawing open, as the provider last drew it: no solving or projection here.
/// A sheet with no cached entries starts with its border and every view and sketch out of date.
pub fn extract(s: &mut Session, sheet: u64) -> Result<SheetView> {
    forget_closed(s);
    let d = s.drawing(CMD)?.clone();
    let state = s.world_state();
    let cache = &mut s.drawing_scenes;
    if let Some(last) = &cache.last
        && last.sheet == sheet
        && last.serial == cache.serial
        && Arc::ptr_eq(&last.drawing, &d)
        && Arc::ptr_eq(&last.state, &state)
        && Arc::ptr_eq(&last.doc, &s.doc)
    {
        return Ok(last.answer.clone());
    }
    let record = sheet_of(&d, sheet)?;
    let on = views_on(&d, sheet)?;
    let sketches = sketches_on(&d, sheet);
    let (source, context) = cache.sketch_inputs(&s.doc, &d);
    cache.drawing = Some(d.id);
    cache.sheet = Some(sheet);
    let stale = on.iter().filter(|(id, view, _)| !cache.fresh(*id, &state, view)).map(|(id, ..)| *id).collect();
    let stale_sketches = sketches.iter().filter(|record| !cache.fresh_sketch(record.id, &source, &context)).map(|record| record.id).collect();
    let serial = |id: &u64| cache.views.get(id).map_or(0, |e| e.serial);
    let inputs = Inputs {
        sheet: record.clone(),
        views: on.iter().map(|(id, _, centre)| (*id, serial(id), *centre)).collect(),
        sketches: sketches.iter().map(|record| (record.id, cache.sketches.get(&record.id).map_or(0, |e| e.serial))).collect(),
    };
    let kept: BTreeSet<u64> = d.sketches.iter().map(|record| record.id).collect();
    cache.sketches.retain(|id, _| kept.contains(id));
    let (scene, views, sketches, generation) = match cache.last.take() {
        Some(last) if last.inputs == inputs => (last.answer.scene, last.answer.views, last.answer.sketches, last.answer.generation),
        _ => {
            // Views no longer in the drawing are forgotten: each holds a world state.
            let kept: BTreeSet<u64> = d.views.iter().map(|v| v.id).collect();
            cache.views.retain(|id, _| kept.contains(id));
            let drawn = on.iter().filter_map(|(id, _, centre)| cache.views.get(id).map(|e| (*id, *centre, &e.drawn)));
            let sketched = sketches.iter().filter_map(|record| cache.sketches.get(&record.id).map(|e| (record.id, &e.drawn)));
            let (scene, views, sketches) = assemble_with_sketches(record, drawn, sketched)?;
            (Arc::new(scene), views, sketches, GENERATION.fetch_add(1, Ordering::Relaxed).wrapping_add(1))
        }
    };
    let answer = SheetView { scene, generation, views, stale, sketches, stale_sketches };
    cache.last = Some(Assembled { sheet, drawing: d, state, doc: s.doc.clone(), serial: cache.serial, inputs, answer: answer.clone() });
    Ok(answer)
}

/// Draws the first stale view, then the first stale sketch, on the sheet last extracted, and
/// says whether more are waiting. One call does one projection or one solve/conversion.
pub fn step(s: &mut Session) -> bool {
    forget_closed(s);
    let (Ok(d), Some(sheet)) = (s.drawing(CMD).cloned(), s.drawing_scenes.sheet) else { return false };
    if sheet_of(&d, sheet).is_err() {
        s.drawing_scenes.sheet = None;
        return false;
    }
    let Ok(on) = views_on(&d, sheet) else { return false };
    let state = s.world_state();
    let (source, context) = s.drawing_scenes.sketch_inputs(&s.doc, &d);
    let waiting: Vec<(u64, ResolvedView)> =
        on.into_iter().filter(|(id, view, _)| !s.drawing_scenes.fresh(*id, &state, view)).map(|(id, view, _)| (id, view)).collect();
    let sketch_waiting: Vec<_> =
        sketches_on(&d, sheet).into_iter().filter(|record| !s.drawing_scenes.fresh_sketch(record.id, &source, &context)).collect();
    let more = waiting.len().saturating_add(sketch_waiting.len()) > 1;
    let Some(&(id, view)) = waiting.first() else {
        let Some(record) = sketch_waiting.first() else { return false };
        let drawn = draw_sketch(s, &d, record);
        let cache = &mut s.drawing_scenes;
        cache.serial = cache.serial.wrapping_add(1);
        cache.sketches.insert(record.id, SketchEntry { source, context, drawn, serial: cache.serial });
        #[cfg(test)]
        {
            cache.sketch_solves += 1;
        }
        return more;
    };
    let shown = match s.drawing_scenes.shown.take() {
        Some((was, shown)) if Arc::ptr_eq(&was, &state) => shown,
        _ => crate::drawing_view::shown(s),
    };
    let drawn = draw(s, &shown, id, &view);
    let cache = &mut s.drawing_scenes;
    cache.shown = Some((state.clone(), shown));
    cache.serial = cache.serial.wrapping_add(1);
    cache.views.insert(id, Entry { state, view, drawn, serial: cache.serial });
    #[cfg(test)]
    {
        cache.projections += 1;
    }
    more
}

/// Drops the cache once its drawing isn't the one open: closing a drawing (or opening another) ends
/// its scenes, and the world states they hold. [`Session::execute`] calls it after every command.
pub(crate) fn forget_closed(s: &mut Session) {
    if s.drawing_scenes.drawing.is_some_and(|d| s.open_drawing != Some(d)) {
        s.drawing_scenes = Cache::default();
    }
}

#[cfg(test)]
#[path = "drawing_scene_tests.rs"]
pub(crate) mod tests;
