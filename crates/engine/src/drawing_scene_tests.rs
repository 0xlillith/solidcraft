use std::sync::Arc;

use serde_json::{Value, json};
use solidcraft_doc::MAX_PLACEMENT_DEPTH;
use solidcraft_doc::drawing::DEFAULT_DPI;
use solidcraft_doc::drawing::view::{View, ViewKind, ViewPlace};
use solidcraft_doc::drawing_view::{ViewDirection, ViewOverrides};
use solidcraft_geom::drawing_view::{NamedView, ProjectedSide};
use solidcraft_geom::paper::{NoPictures, ObjectKind};

use super::*;
use crate::schema::drawing_scene::SheetSceneResult;
use crate::schema::output_schema_for;
use crate::schema::tests::conforms;

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    s.execute(id, &p).unwrap()
}

/// A block 100 x 50 x 20 mm along X, Y and Z from the origin, and a drawing of it on A3 with the
/// ISO border: the front view at 1:2 centred at (150, 150), and the view above it, the top view in
/// third angle, lined up with it at y 220. The session, the sheet and the two views' ids.
pub(crate) fn front_and_top() -> (Session, u64, [u64; 2]) {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length": 100, "width": 50, "height": 20, "body_name": "Block"}));
    front_and_top_of(s)
}

/// [`front_and_top`]'s drawing of the design `s`.
fn front_and_top_of(mut s: Session) -> (Session, u64, [u64; 2]) {
    run(&mut s, "drawing.create", json!({}));
    let mut d = (**s.drawing("test").unwrap()).clone();
    let mut id_source = solidcraft_doc::ids::IdSource::seeded(4890);
    let (sheet, source) = (d.sheets[0].id, d.sources[0].id);
    let mut add = |d: &mut Drawing, kind: ViewKind, place: ViewPlace, settings: ViewOverrides| {
        let id = d.allocate_id(&mut id_source).unwrap();
        d.add_views([Arc::new(View { id, sheet, kind, place, settings, reference_shown_percent: 0.0 })]);
        id
    };
    let half = ViewOverrides { scale: Some("1:2".parse().unwrap()), ..ViewOverrides::default() };
    let front = add(&mut d, ViewKind::Base { source, view: ViewDirection::Named(NamedView::Front) }, ViewPlace::At([150.0, 150.0]), half);
    let top = add(&mut d, ViewKind::Projected { parent: front, side: ProjectedSide::Above }, ViewPlace::Aligned(220.0), ViewOverrides::default());
    d.validate().unwrap();
    s.doc_mut().put_drawing(Arc::new(d));
    (s, sheet, [front, top])
}

/// Component `L{level}`, placed inside `L{level - 1}` (the design itself at level 0).
pub(crate) fn nest(s: &mut Session, level: u64) {
    let (component, parent) = (1000 + level, if level == 0 { 0 } else { 1000 + level - 1 });
    let doc = s.doc_mut();
    doc.components.push(solidcraft_doc::Component { id: component, name: format!("L{level}"), parent, ..Default::default() });
    let name = format!("L{level}");
    // From JSON, so a field another change adds to occurrences takes its default here.
    let occurrence = json!({"id": 5000 + level, "component": component, "parent": parent, "name": name});
    doc.occurrences.push(serde_json::from_value(occurrence).unwrap());
}

/// Sheet `sheet` of the drawing open, built at once.
fn built(s: &Session, sheet: u64) -> SheetScene {
    let d = s.drawing("test").unwrap().clone();
    build(s, &d, sheet).unwrap()
}

/// The provider's scene of `sheet` once nothing is waiting.
fn settled(s: &mut Session, sheet: u64) -> SheetView {
    extract(s, sheet).unwrap();
    while step(s) {}
    extract(s, sheet).unwrap()
}

/// The scene as its JSON text: what byte for byte compares.
fn bytes(scene: &Scene) -> String {
    serde_json::to_string(scene).unwrap()
}

/// The drawing border's groups of sheet `sheet` of the drawing open, as its sheet layout draws them.
fn border(s: &Session, sheet: u64) -> Vec<Group> {
    let d = s.drawing("test").unwrap();
    let record = d.sheets.iter().find(|x| x.id == sheet).unwrap();
    record.border.as_ref().unwrap().layout(&record.sheet).unwrap().paper(sheet)
}

/// The box round the ends of `group`'s stroke pieces, and how long they are in all.
fn reach(group: &Group) -> (Vec2, Vec2, f64) {
    let (mut lo, mut hi, mut length) = (Vec2::new(f64::INFINITY, f64::INFINITY), Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY), 0.0);
    for item in &group.items {
        let Item::Stroke { path, .. } = item else { continue };
        for seg in path {
            for p in [seg.start(), seg.end()] {
                lo = Vec2::new(lo.x.min(p.x), lo.y.min(p.y));
                hi = Vec2::new(hi.x.max(p.x), hi.y.max(p.y));
            }
            length += seg.length();
        }
    }
    (lo, hi, length)
}

/// Whether `a` and `b` are the same strokes in the same order, their pieces' ends within `tol`.
fn same_within(a: &Group, b: &Group, tol: f64) -> bool {
    let near = |s: &Seg2, t: &Seg2| s.start().dist(t.start()) <= tol && s.end().dist(t.end()) <= tol;
    a.items.len() == b.items.len()
        && a.items.iter().zip(&b.items).all(|pair| match pair {
            (Item::Stroke { path: p, closed: c, width_mm: w }, Item::Stroke { path: q, closed: d, width_mm: v }) => {
                c == d && w == v && p.len() == q.len() && p.iter().zip(q).all(|(s, t)| near(s, t))
            }
            _ => false,
        })
}

/// A real sheet editor under inch display units; command coordinates still mean paper mm.
fn stroke_editor() -> (Session, u64, u64) {
    let mut s = Session::default();
    run(&mut s, "solid.box", json!({"length":40,"width":30,"height":5,"body_name":"Source plate"}));
    run(&mut s, "document.units", json!({"units":"in"}));
    run(&mut s, "parameters.add", json!({"name":"Inset","expression":"1 in"}));
    let drawing = run(&mut s, "drawing.create", json!({"name":"Strokes","units":"in"}));
    let made = run(&mut s, "drawing.sketch.create", json!({"owner":drawing["sheets"][0]["id"]}));
    let owner = ids::decode(made["drawing"].as_str().unwrap()).unwrap();
    let sketch = ids::decode(made["sketch"].as_str().unwrap()).unwrap();
    (s, owner, sketch)
}

fn stroke_path(item: &Item) -> (&[Seg2], bool, f64) {
    let Item::Stroke { path, closed, width_mm } = item else { panic!("not a stroke: {item:?}") };
    (path, *closed, *width_mm)
}

#[test]
fn sheet_sketch_strokes_keep_paper_coordinates_and_read_only_source() {
    let (mut s, owner, id) = stroke_editor();
    run(&mut s, "sketch.rectangle.two_point", json!({"p0":[15,10],"p1":[55,30]}));
    run(&mut s, "sketch.line", json!({"points":[["Inset + 5","40"],["Inset + 20","40"]],"infer":false}));
    let before = s.doc.clone();
    let model = s.model.state();
    let history = (s.undo.len(), s.redo.len(), s.evaluations);
    let solved = s.doc.solve_sheet_sketch(owner, id).unwrap();
    let group = sketch::paper_group(&solved, 0.35).unwrap();
    assert_eq!((group.object, group.kind, group.items.len()), (id, ObjectKind::Sketch, 5));
    assert_eq!(group.kind.layer(), "SKETCH");
    let mut rectangle = group.clone();
    rectangle.items.truncate(4);
    let (lo, hi, perimeter) = reach(&rectangle);
    assert!(lo.dist(Vec2::new(15.0, 10.0)) < 1e-9 && hi.dist(Vec2::new(55.0, 30.0)) < 1e-9);
    assert!((perimeter - 120.0).abs() < 1e-9);
    let (line, closed, width) = stroke_path(&group.items[4]);
    assert!(!closed && width == 0.35 && line.len() == 1);
    assert!(line[0].start().dist(Vec2::new(30.4, 40.0)) < 1e-9);
    assert!(line[0].end().dist(Vec2::new(45.4, 40.0)) < 1e-9);
    assert!(Arc::ptr_eq(&before, &s.doc) && Arc::ptr_eq(&model, &s.model.state()));
    assert_eq!((s.undo.len(), s.redo.len(), s.evaluations), history);
    assert_eq!(s.doc.drawing(owner).unwrap().sheet_sketch(id).unwrap().sketch, solved.sketch);

    // Reopening and solving through the same read-only boundary gives the same writer input.
    let reopened = solidcraft_doc::Document::from_json(&s.doc.to_json()).unwrap();
    let again = sketch::paper_group(&reopened.solve_sheet_sketch(owner, id).unwrap(), 0.35).unwrap();
    assert_eq!(again, group);
    let scene = Scene::new(Vec2::new(420.0, 297.0), vec![group]);
    let svg = || solidcraft_io::paper::paper_svg(&scene.prepare().unwrap(), &NoPictures, DEFAULT_DPI).unwrap().0;
    assert_eq!(svg(), svg());
    assert!(String::from_utf8(svg()).unwrap().contains("data-kind=\"sketch\""));
}

#[test]
fn sheet_sketch_strokes_preserve_analytic_curves_and_clockwise_travel() {
    let (mut s, owner, id) = stroke_editor();
    for (cmd, params) in [
        ("sketch.line", json!({"points":[[5,5],[15,5]],"infer":false})),
        ("sketch.circle.center", json!({"center":[30,40],"radius":5})),
        ("sketch.arc.center_point", json!({"center":[30,40],"start":[35,40],"sweep":-90})),
        ("sketch.ellipse", json!({"center":[80,50],"major":[100,50],"minor_radius":10})),
        ("sketch.conic", json!({"start":[40,80],"end":[90,80],"apex":[65,105],"rho":0.4,"connect":false})),
    ] {
        run(&mut s, cmd, params);
    }
    let solved = s.doc.solve_sheet_sketch(owner, id).unwrap();
    let group = sketch::paper_group(&solved, 0.5).unwrap();
    assert_eq!(group.items.len(), 5);
    assert!(matches!(stroke_path(&group.items[0]).0, [Seg2::Line { .. }]));
    let (circle, closed, _) = stroke_path(&group.items[1]);
    assert!(closed && circle.len() == 2);
    for seg in circle {
        let Seg2::Arc { center, radius, sweep, .. } = seg else { panic!("sampled circle: {seg:?}") };
        assert_eq!((*center, *radius), (Vec2::new(30.0, 40.0), 5.0));
        assert!((sweep - std::f64::consts::PI).abs() < 1e-12);
    }
    let (arc, closed, _) = stroke_path(&group.items[2]);
    let [Seg2::Arc { radius, sweep, .. }] = arc else { panic!("sampled arc: {arc:?}") };
    assert!(!closed && *radius == 5.0 && (*sweep + std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    assert!(arc[0].start().dist(Vec2::new(35.0, 40.0)) < 1e-9 && arc[0].end().dist(Vec2::new(30.0, 35.0)) < 1e-9);
    let (ellipse, closed, _) = stroke_path(&group.items[3]);
    assert!(closed && ellipse.len() == 4);
    for seg in ellipse {
        assert!(matches!(seg, Seg2::Conic { w, .. } if (*w - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-15));
        for t in [0.0, 0.2, 0.5, 0.8, 1.0] {
            let p = seg.point_at(t) - Vec2::new(80.0, 50.0);
            assert!((p.x * p.x / 400.0 + p.y * p.y / 100.0 - 1.0).abs() < 1e-12, "{p:?}");
        }
    }
    let (conic, closed, _) = stroke_path(&group.items[4]);
    let [Seg2::Conic { a, apex, b, w }] = conic else { panic!("sampled conic: {conic:?}") };
    assert!(!closed && (*w - 2.0 / 3.0).abs() < 1e-12);
    assert_eq!((*a, *apex, *b), (Vec2::new(40.0, 80.0), Vec2::new(65.0, 105.0), Vec2::new(90.0, 80.0)));
    assert!(conic[0].point_at(0.5).dist(Vec2::new(65.0, 90.0)) < 1e-9);
}

#[test]
fn sheet_sketch_strokes_keep_fit_periodic_and_quintic_splines_exact() {
    let (mut s, owner, id) = stroke_editor();
    run(&mut s, "sketch.spline.fit_point", json!({"points":[[10,10],[12,12],[14,10]],"connect":false}));
    run(&mut s, "sketch.spline.fit_point", json!({"points":[[80,30],[100,50],[120,30],[100,10]],"periodic":true,"connect":false}));
    let controls = [[140, 10], [150, 30], [160, 35], [170, 35], [180, 30], [190, 10]];
    run(&mut s, "sketch.spline.control_point_5", json!({"points":controls,"connect":false}));
    let group = sketch::paper_group(&s.doc.solve_sheet_sketch(owner, id).unwrap(), 0.5).unwrap();
    assert_eq!(group.items.len(), 3);
    let (fit, closed, _) = stroke_path(&group.items[0]);
    assert!(!closed && fit.len() == 2 && fit.iter().all(|seg| matches!(seg, Seg2::Cubic { .. })));
    let Seg2::Cubic { p0, p1, p2, p3 } = fit[0] else { panic!("lost cubic") };
    for (actual, expected) in [p0, p1, p2, p3].into_iter().zip([[10.0, 10.0], [10.0 + 2.0 / 3.0, 11.0], [10.0 + 4.0 / 3.0, 12.0], [12.0, 12.0]]) {
        assert!(actual.dist(Vec2::from(expected)) < 1e-9, "{actual:?} {expected:?}");
    }
    assert!(fit[0].point_at(0.5).dist(Vec2::new(11.0, 11.375)) < 1e-9);
    for (seg, a, b) in fit.iter().zip([[10.0, 10.0], [12.0, 12.0]]).zip([[12.0, 12.0], [14.0, 10.0]]).map(|((s, a), b)| (s, a, b)) {
        assert!(seg.start().dist(Vec2::from(a)) < 1e-9 && seg.end().dist(Vec2::from(b)) < 1e-9);
    }
    let (periodic, closed, _) = stroke_path(&group.items[1]);
    assert!(closed && periodic.len() == 4 && periodic.iter().all(|seg| matches!(seg, Seg2::Cubic { .. })));
    for i in 0..periodic.len() {
        let next = &periodic[(i + 1) % periodic.len()];
        assert!(periodic[i].end().dist(next.start()) < 1e-9);
        let end = periodic[i].derivatives(1.0, 2).unwrap();
        let start = next.derivatives(0.0, 2).unwrap();
        assert!(end[0].dist(start[0]) < 1e-8 && end[1].dist(start[1]) < 1e-8);
    }
    let (quintic, closed, _) = stroke_path(&group.items[2]);
    let [Seg2::Bezier { n: 5, p }] = quintic else { panic!("lost quintic: {quintic:?}") };
    assert!(!closed);
    for (actual, expected) in p.iter().zip(controls) {
        assert!(actual.dist(Vec2::new(expected[0] as f64, expected[1] as f64)) < 1e-9);
    }
}

#[test]
fn sheet_sketch_strokes_omit_construction_and_refuse_centerlines_by_name() {
    let (mut s, owner, id) = stroke_editor();
    run(&mut s, "sketch.line", json!({"points":[[10,10],[30,10]],"infer":false}));
    let mut solved = s.doc.solve_sheet_sketch(owner, id).unwrap();
    let curve = solved.sketch.curves[0].id.clone();
    let original = sketch::paper_group(&solved, 0.5).unwrap();
    solved.sketch.view.hide_grid = true;
    solved.sketch.view.no_snap = true;
    assert_eq!(sketch::paper_group(&solved, 0.5).unwrap(), original, "editor visibility is not print state");
    solved.sketch.curves[0].construction = true;
    solved.sketch.curves[0].infinite = true;
    assert!(sketch::paper_group(&solved, 0.5).unwrap().items.is_empty());
    for construction in [false, true] {
        solved.sketch.curves[0].infinite = false;
        solved.sketch.curves[0].construction = construction;
        solved.sketch.curves[0].centerline = true;
        let why = sketch::paper_group(&solved, 0.5).unwrap_err().to_string();
        assert!(
            why.contains(&ids::encode(owner)) && why.contains(&ids::encode(solved.sheet)) && why.contains(&ids::encode(id)) && why.contains(&curve),
            "{why}"
        );
        assert!(why.contains("centerline"), "{why}");
    }
}

#[test]
fn sheet_sketch_strokes_do_not_close_a_short_open_curve_by_tolerance() {
    let (mut s, owner, id) = stroke_editor();
    run(&mut s, "sketch.line", json!({"points":[[10,10],[30,10]],"infer":false}));
    let mut solved = s.doc.solve_sheet_sketch(owner, id).unwrap();
    solved.sketch.points[2].pos = solved.sketch.points[1].pos + Vec2::new(solidcraft_geom::EPS / 2.0, 0.0);
    let group = sketch::paper_group(&solved, 0.5).unwrap();
    let (path, closed, _) = stroke_path(&group.items[0]);
    assert!(!closed && path[0].start() != path[0].end());
    assert!(path[0].start().dist(path[0].end()) < solidcraft_geom::EPS);
}

#[test]
fn sheet_sketch_strokes_refuse_hostile_geometry_without_a_polyline_fallback() {
    use solidcraft_sketch::{CurveKind, SolveStatus};
    let (mut s, owner, id) = stroke_editor();
    run(&mut s, "sketch.line", json!({"points":[[10,10],[30,10]],"infer":false}));
    let original = s.doc.solve_sheet_sketch(owner, id).unwrap();
    for width in [0.0, -1.0, 10.0001, f64::NAN, f64::INFINITY] {
        assert!(sketch::paper_group(&original, width).is_err(), "width {width}");
    }
    let mut cases = Vec::new();
    let mut add = |change: fn(&mut solidcraft_doc::drawing::SolvedSheetSketch)| {
        let mut bad = original.clone();
        change(&mut bad);
        cases.push(bad);
    };
    add(|s| s.id = 0);
    add(|s| s.report.status = SolveStatus::Failed);
    add(|s| s.sketch.points[1].pos.x = f64::NAN);
    add(|s| s.sketch.points[1].pos.x = 100_001.0);
    add(|s| s.sketch.curves[0].kind = CurveKind::Line { a: 1, b: usize::MAX });
    add(|s| s.sketch.curves[0].kind = CurveKind::Spline { pts: vec![1, 2], control: true, degree: 10, periodic: false });
    add(|s| s.sketch.curves[0].kind = CurveKind::Ellipse { c: 1, m: 1, r: 5.0 });
    add(|s| s.sketch.curves[0].kind = CurveKind::Conic { a: 1, b: 2, apex: 0, rho: 1.0 - 1e-10 });
    add(|s| s.sketch.curves[0].kind = CurveKind::Arc { c: 0, a: 1, b: 2 });
    add(|s| s.sketch.curves[0].infinite = true);
    add(|s| s.sketch.view.three_d = true);
    add(|s| s.sketch.curves[0].link = Some("unresolved".into()));
    add(|s| {
        s.sketch.equations.push(solidcraft_sketch::EquationCurve {
            curve: s.sketch.curves[0].id.clone(),
            x: None,
            y: "x".into(),
            start: "0".into(),
            end: "1".into(),
            tolerance: 0.01,
        })
    });
    for bad in cases {
        let why = sketch::paper_group(&bad, 0.5).unwrap_err().to_string();
        assert!(why.contains("sheet sketch"), "{why}");
    }
    assert_eq!(sketch::paper_group(&original, 0.5).unwrap().items.len(), 1, "no partial output or mutation after errors");
}

#[test]
fn sheet_sketch_strokes_respect_the_exact_paper_record_limit() {
    use solidcraft_sketch::{CurveKind, Sketch};
    let (s, owner, id) = stroke_editor();
    let mut solved = s.doc.solve_sheet_sketch(owner, id).unwrap();
    let mut geometry = Sketch::new();
    let centre = geometry.add_point(Vec2::new(100.0, 100.0), None).unwrap();
    let major = geometry.add_point(Vec2::new(110.0, 100.0), None).unwrap();
    // 19,998 ellipse strokes at 5 records each, plus the group itself: 99,991.
    let ellipse = geometry.add_curve(CurveKind::Ellipse { c: centre, m: major, r: 5.0 }, None).unwrap();
    let template = geometry.curves[ellipse].clone();
    geometry.curves = (0..19_998).map(|i| solidcraft_sketch::Curve { id: format!("ellipse{i}"), ..template.clone() }).collect();
    let pts = (0..9).map(|i| geometry.add_point(Vec2::new(20.0 + i as f64, 20.0 + (i % 2) as f64), None).unwrap()).collect();
    geometry.add_curve(CurveKind::Spline { pts, control: false, degree: 3, periodic: false }, None).unwrap();
    solved.sketch = geometry;
    let full = sketch::paper_group(&solved, 0.5).unwrap();
    assert_eq!(full.records(), MAX_RECORDS, "eight cubic pieces plus their stroke consume the remaining nine records");
    solved.sketch.add_curve(CurveKind::Line { a: centre, b: major }, None).unwrap();
    let why = sketch::paper_group(&solved, 0.5).unwrap_err().to_string();
    assert!(why.contains(&MAX_RECORDS.to_string()), "{why}");
    assert_eq!(full.records(), MAX_RECORDS);
}

/// The scene is the sheet's three border groups as its layout draws them, byte for byte, then the
/// two views in id order: the front view round its 50 x 10 mm outline centred at (150, 150), 120 mm
/// of line in all, the top view round 50 x 25 mm centred at (150, 220), 150 mm. The command answers
/// the same scene, matching its schema, and the SVG writer takes it with the border's three groups
/// before the views'.
#[test]
fn a_sheet_is_its_border_then_its_views_by_id() {
    let (mut s, sheet, [front, top]) = front_and_top();
    let SheetScene { scene, views, .. } = built(&s, sheet);
    let border = border(&s, sheet);
    assert_eq!(border.len(), 3);
    assert_eq!(serde_json::to_string(&scene.groups[..3]).unwrap(), serde_json::to_string(&border).unwrap());
    let order = if front < top { [front, top] } else { [top, front] };
    let tail: Vec<(u64, ObjectKind)> = scene.groups[3..].iter().map(|g| (g.object, g.kind)).collect();
    assert_eq!(tail, order.map(|id| (id, ObjectKind::View)));
    for (id, lo, hi, length) in [(front, [125.0, 145.0], [175.0, 155.0], 120.0), (top, [125.0, 207.5], [175.0, 232.5], 150.0)] {
        let (min, max, drawn) = reach(scene.groups.iter().find(|g| g.object == id).unwrap());
        assert!(min.dist(Vec2::from(lo)) < 1e-6 && max.dist(Vec2::from(hi)) < 1e-6 && (drawn - length).abs() < 1e-6, "{id}: {min:?} {max:?} {drawn}");
    }
    let statuses: Vec<(u64, Status)> = views.iter().map(|v| (v.view, v.status.clone())).collect();
    assert_eq!(statuses, order.map(|id| (id, Status::Drawn)));

    let reply = run(&mut s, CMD, json!({"sheet": "Sheet 1"}));
    let schema = output_schema_for::<SheetSceneResult>();
    assert!(conforms(schema.as_value(), schema.as_value(), &reply), "{reply}");
    assert_eq!(reply["scene"], serde_json::to_value(&scene).unwrap());
    let replied: Vec<(Value, Value)> = reply["views"].as_array().unwrap().iter().map(|v| (v["view"].clone(), v["status"].clone())).collect();
    assert_eq!(replied, order.map(|id| (json!(ids::encode(id)), json!("drawn"))));

    let (svg, _) = solidcraft_io::paper::paper_svg(&scene.prepare().unwrap(), &NoPictures, DEFAULT_DPI).unwrap();
    let svg = String::from_utf8(svg).unwrap();
    let borders: Vec<usize> = svg.match_indices("data-kind=\"drawing_border\"").map(|(at, _)| at).collect();
    let first_view = svg.find("data-kind=\"view\"").unwrap();
    assert!(borders.len() == 3 && borders.iter().all(|at| *at < first_view), "{svg}");
}

/// Each view's group is the snapshot's for the same view, scale and centre: the same strokes in the
/// same order, their ends within rounding. The scene works a view's lines out centred on it and
/// then moves them; the snapshot maps them to the centre in one step.
#[test]
fn each_view_draws_as_its_snapshot_within_rounding() {
    let (mut s, sheet, [front, top]) = front_and_top();
    let scene = built(&s, sheet).scene;
    // The snapshot works on the model, so with no drawing open.
    run(&mut s, "drawing.close", json!({}));
    for (id, named, at) in [(front, "front", [150.0, 150.0]), (top, "top", [150.0, 220.0])] {
        let v = run(&mut s, "drawing.view.snapshot", json!({"view": named, "scale": "1:2", "position_mm": at, "scene": true}));
        let snapped: Scene = serde_json::from_value(v["scene"].clone()).unwrap();
        let ours = scene.groups.iter().find(|g| g.object == id).unwrap();
        assert!(same_within(ours, &snapped.groups[0], 1e-9), "{id}: {ours:?} {:?}", snapped.groups[0]);
    }
}

/// A view of many lines is drawn whole, as its snapshot is: the block drilled with eight holes and
/// seen from above is its outline and the eight rims, some 30 records.
#[test]
fn a_view_of_many_lines_draws_as_its_snapshot() {
    let (mut s, sheet, [_, top]) = front_and_top();
    let drawing = ids::encode(s.open_drawing.unwrap());
    run(&mut s, "drawing.close", json!({}));
    for i in 0..8 {
        let base = [12.5 + 25.0 * (i % 4) as f64, 12.5 + 25.0 * (i / 4) as f64, -1.0];
        run(&mut s, "solid.cylinder", json!({"diameter": 8, "height": 22, "base": base, "operation": "cut", "targets": ["Block"]}));
    }
    let v = run(&mut s, "drawing.view.snapshot", json!({"view": "top", "scale": "1:2", "position_mm": [150.0, 220.0], "scene": true}));
    let snapped: Scene = serde_json::from_value(v["scene"].clone()).unwrap();
    run(&mut s, "drawing.open", json!({"drawing": drawing}));
    let scene = built(&s, sheet);
    let state = scene.views.iter().find(|v| v.view == top).unwrap();
    assert_eq!(state.status, Status::Drawn, "{state:?}");
    let ours = scene.scene.groups.iter().find(|g| g.object == top).unwrap();
    assert!(ours.records() > 20 && same_within(ours, &snapped.groups[0], 1e-9), "{ours:?} {:?}", snapped.groups[0]);
}

/// A sheet with no views is its border alone.
#[test]
fn a_sheet_with_no_views_is_its_border() {
    let (mut s, _, _) = front_and_top();
    let second = run(&mut s, "drawing.sheet.add", json!({}))["sheet"].as_str().and_then(ids::decode).unwrap();
    let SheetScene { scene, views, .. } = built(&s, second);
    assert_eq!((serde_json::to_string(&scene.groups).unwrap(), views.len()), (serde_json::to_string(&border(&s, second)).unwrap(), 0));
}

/// A view in a style the paper scene can't draw yet fails with the snapshot's reason, and a view
/// with nothing to measure is a placeholder; each draws nothing, and the sheet's other views and
/// its border are drawn all the same. The command says why for each.
#[test]
fn a_view_that_cant_be_drawn_leaves_out_only_itself() {
    let (mut s, sheet, [front, top]) = front_and_top();
    let status = |s: &Session, id: u64| built(s, sheet).views.into_iter().find(|v| v.view == id).map(|v| v.status);
    for (style, why) in [
        ("shaded", "not supported yet: shaded views need the drawing renderer (#5624)"),
        ("visible_hidden", "not supported yet: hidden lines in the paper scene need dashed lines (#4890 A5)"),
    ] {
        run(&mut s, "drawing.view.edit", json!({"view": ids::encode(top), "set": {"display_style": style}}));
        assert_eq!((status(&s, front), status(&s, top)), (Some(Status::Drawn), Some(Status::Failed(why.into()))), "{style}");
        let objects: Vec<u64> = built(&s, sheet).scene.groups.iter().map(|g| g.object).collect();
        assert_eq!(objects, [sheet, sheet, sheet, front], "{style}");
    }
    let reply = run(&mut s, CMD, json!({"sheet": ids::encode(sheet)}));
    let schema = output_schema_for::<SheetSceneResult>();
    assert!(conforms(schema.as_value(), schema.as_value(), &reply), "{reply}");
    let failed = reply["views"].as_array().unwrap().iter().find(|v| v["view"] == ids::encode(top)).unwrap().clone();
    assert_eq!((&failed["status"], failed["reason"].as_str().map(|r| r.contains("dashed lines"))), (&json!("failed"), Some(true)), "{reply}");
    // Past the walk limit the front view has nothing to measure.
    for level in 0..=MAX_PLACEMENT_DEPTH as u64 + 1 {
        nest(&mut s, level);
    }
    assert_eq!(status(&s, front), Some(Status::Placeholder(Unmeasured::PastWalkLimit)));
    assert_eq!(built(&s, sheet).scene.groups.len(), 3);
}

/// Neither the command nor the provider evaluates the model.
#[test]
fn drawing_a_sheet_evaluates_no_model() {
    let (mut s, sheet, _) = front_and_top();
    let before = s.evaluations;
    run(&mut s, CMD, json!({"sheet": ids::encode(sheet)}));
    settled(&mut s, sheet);
    assert_eq!(s.evaluations, before);
}

/// An unknown sheet and another drawing's sheet are refused, and so is a sheet with no drawing open.
#[test]
fn unknown_and_other_drawings_sheets_are_refused() {
    let (mut s, sheet, _) = front_and_top();
    let refused = |s: &mut Session, named: &str| s.execute(CMD, &json!({"sheet": named})).unwrap_err().to_string();
    assert!(refused(&mut s, "Sheet 9").contains("no sheet `Sheet 9`"));
    run(&mut s, "drawing.create", json!({"name": "Second"}));
    let first = ids::encode(sheet);
    assert!(refused(&mut s, &first).contains(&format!("no sheet `{first}`")));
    assert!(extract(&mut s, sheet).unwrap_err().to_string().contains("the drawing has no sheet"));
    run(&mut s, "drawing.close", json!({}));
    assert!(refused(&mut s, &first).contains("open a drawing first"));
    assert!(extract(&mut s, sheet).is_err() && !step(&mut s));
}

/// A new cache gives the border with both views out of date and draws nothing; each step draws one
/// view, in id order; once nothing is waiting the scene is the built one, byte for byte. The
/// generation goes up each time the scene changes, and only then.
#[test]
fn the_provider_draws_a_view_a_step_until_it_is_the_built_scene() {
    let (mut s, sheet, [front, top]) = front_and_top();
    let order = if front < top { [front, top] } else { [top, front] };
    let whole = built(&s, sheet).scene;
    let first = extract(&mut s, sheet).unwrap();
    assert_eq!(
        (bytes(&first.scene), first.stale.as_slice(), first.views.len()),
        (bytes(&Scene::new(whole.size_mm, whole.groups[..3].to_vec())), &order[..], 0)
    );
    let again = extract(&mut s, sheet).unwrap();
    assert!(Arc::ptr_eq(&again.scene, &first.scene) && again.generation == first.generation);
    assert_eq!(s.drawing_scenes.projections, 0);

    assert!(step(&mut s));
    let one = extract(&mut s, sheet).unwrap();
    let objects: Vec<u64> = one.scene.groups.iter().map(|g| g.object).collect();
    assert_eq!((objects, one.stale.as_slice()), (vec![sheet, sheet, sheet, order[0]], &order[1..]));
    assert!(one.generation > first.generation);
    assert!(!step(&mut s));
    let both = extract(&mut s, sheet).unwrap();
    assert_eq!((bytes(&both.scene), both.stale.len()), (bytes(&whole), 0));
    assert!(both.generation > one.generation);
    assert!(both.views.iter().all(|v| v.status == Status::Drawn) && both.views.len() == 2);
    assert!(!step(&mut s));
    assert_eq!((s.drawing_scenes.projections, extract(&mut s, sheet).unwrap().generation), (2, both.generation));
}

/// Moving a view moves its lines and projects nothing: the top view is lined up with the front
/// one, so both move 50 mm right, and the scene is the built one. Undoing the move projects nothing
/// either and gives back the scene before it, byte for byte.
#[test]
fn a_move_and_its_undo_project_nothing() {
    let (mut s, sheet, [front, _]) = front_and_top();
    let before = settled(&mut s, sheet);
    let projections = s.drawing_scenes.projections;
    run(&mut s, "drawing.view.edit", json!({"view": ids::encode(front), "set": {"place": {"at": [200.0, 150.0]}}}));
    let after = extract(&mut s, sheet).unwrap();
    assert_eq!((after.stale.len(), s.drawing_scenes.projections), (0, projections));
    assert!(after.generation > before.generation);
    assert_eq!(bytes(&after.scene), bytes(&built(&s, sheet).scene));
    for (was, now) in before.scene.groups.iter().zip(&after.scene.groups).skip(3) {
        assert!(same_within(now, &moved(was, Vec2::new(50.0, 0.0)), 1e-9), "{now:?}");
    }
    run(&mut s, "edit.undo", json!({}));
    let undone = extract(&mut s, sheet).unwrap();
    assert_eq!((undone.stale.len(), s.drawing_scenes.projections), (0, projections));
    assert_eq!(bytes(&undone.scene), bytes(&before.scene));
}

/// A view's picture moves with its lines: its rectangle by exactly the offset, its pixels and
/// opacity as they were.
#[test]
fn a_moved_group_moves_its_picture() {
    let picture = |min: Vec2, max: Vec2| Item::Picture(Picture { source: 3, rect: Aabb2 { min, max }, opacity: 0.5 });
    let group = Group { object: 7, kind: ObjectKind::View, items: vec![picture(Vec2::new(1.0, 2.0), Vec2::new(11.0, 7.0))] };
    assert_eq!(moved(&group, Vec2::new(50.0, -2.0)).items, [picture(Vec2::new(51.0, 0.0), Vec2::new(61.0, 5.0))]);
}

/// After a model change (going back to a checkpoint taken before a tower was added) both views are
/// out of date and the last scene still shows. Each step draws one view from the model as it is
/// now, and a view listed as up to date always draws as the built scene does.
#[test]
fn after_a_model_change_the_last_scene_shows_until_its_views_are_drawn_again() {
    let (mut s, sheet, _) = front_and_top();
    let drawing = ids::encode(s.open_drawing.unwrap());
    let checkpoint = run(&mut s, "edit.checkpoint", json!({}))["checkpoint"].clone();
    run(&mut s, "drawing.close", json!({}));
    run(&mut s, "solid.box", json!({"length": 20, "width": 20, "height": 40, "corner": [40, 60, 0], "body_name": "Tower", "operation": "new"}));
    run(&mut s, "drawing.open", json!({"drawing": drawing}));
    let with_tower = settled(&mut s, sheet);
    run(&mut s, "edit.restore_checkpoint", json!({"checkpoint": checkpoint}));
    let projections = s.drawing_scenes.projections;
    let old = extract(&mut s, sheet).unwrap();
    assert_eq!((old.stale.len(), bytes(&old.scene), old.generation), (2, bytes(&with_tower.scene), with_tower.generation));
    let now = built(&s, sheet).scene;
    let group = |scene: &Scene, id: u64| {
        scene.groups.iter().find(|g| g.object == id && g.kind == ObjectKind::View).map(|g| serde_json::to_string(g).unwrap())
    };
    loop {
        let more = step(&mut s);
        let mid = extract(&mut s, sheet).unwrap();
        for v in mid.views.iter().filter(|v| !mid.stale.contains(&v.view)) {
            assert_eq!(group(&mid.scene, v.view), group(&now, v.view), "{}", v.view);
        }
        if !more {
            break;
        }
    }
    let last = extract(&mut s, sheet).unwrap();
    assert_eq!((last.stale.len(), s.drawing_scenes.projections, bytes(&last.scene)), (0, projections + 2, bytes(&now)));
}

/// Closing the drawing drops its scenes and the world states they hold; opened again, every view
/// is out of date.
#[test]
fn closing_the_drawing_drops_its_scenes() {
    let (mut s, sheet, _) = front_and_top();
    let drawing = ids::encode(s.open_drawing.unwrap());
    let before = settled(&mut s, sheet);
    assert_eq!(s.drawing_scenes.views.len(), 2);
    run(&mut s, "drawing.close", json!({}));
    let cache = &s.drawing_scenes;
    assert!(cache.views.is_empty() && cache.last.is_none() && cache.shown.is_none() && cache.drawing.is_none());
    assert!(!step(&mut s));
    run(&mut s, "drawing.open", json!({"drawing": drawing}));
    let reopened = extract(&mut s, sheet).unwrap();
    // A new scene, so a new number: the page can't take it for the one it showed before.
    assert!(reopened.stale.len() == 2 && reopened.generation > before.generation);
}

/// A view whose lines would take the sheet past a paper scene's records fails and is left out;
/// a view after it that fits is still drawn, up to the limit exactly.
#[test]
fn a_view_past_the_sheets_records_leaves_out_only_itself() {
    let (s, sheet, _) = front_and_top();
    let d = s.drawing("test").unwrap().clone();
    let record = d.sheets.iter().find(|x| x.id == sheet).unwrap();
    // One stroke of `n` pieces: n + 2 records with its group.
    let lines = |n: usize| {
        let path = (0..n).map(|i| Seg2::Line { a: Vec2::new(i as f64 * 1e-3, 0.0), b: Vec2::new((i + 1) as f64 * 1e-3, 0.0) }).collect();
        let items = vec![Item::Stroke { path, closed: false, width_mm: 0.5 }];
        Drawn { group: Some(Group { object: n as u64, kind: ObjectKind::View, items }), status: Status::Drawn, stats: None }
    };
    let left = MAX_RECORDS - border(&s, sheet).iter().map(Group::records).sum::<usize>();
    // The first leaves 20 records; the second needs 32; the third takes the last 20.
    let views = [lines(left - 22), lines(30), lines(18)];
    let centre = Vec2::new(100.0, 100.0);
    let (scene, states) = assemble(record, views.iter().enumerate().map(|(i, v)| (i as u64, centre, v))).unwrap();
    let statuses: Vec<&'static str> = states.iter().map(|v| v.status.name()).collect();
    assert_eq!(statuses, ["drawn", "failed", "drawn"]);
    assert_eq!((scene.groups.len(), scene.groups.iter().map(Group::records).sum::<usize>()), (5, MAX_RECORDS));
    assert!(matches!(&states[1].status, Status::Failed(why) if why.contains("put it on another sheet")), "{:?}", states[1]);
}

/// A view at 200:1 asks the mesh for chords no finer than its floor, so it is drawn rather than
/// refused for a chord it has no setting for.
#[test]
fn a_view_past_100_to_1_is_drawn_with_a_coarser_chord() {
    let (mut s, sheet, [front, _]) = front_and_top();
    run(&mut s, "drawing.view.edit", json!({"view": ids::encode(front), "set": {"scale": "200:1"}}));
    let scene = built(&s, sheet);
    let front = scene.views.iter().find(|v| v.view == front).unwrap();
    assert_eq!(front.status, Status::Drawn, "{front:?}");
}

/// Seeded mixes of moves, setting changes, undos, redos, extracts and steps: once nothing is
/// waiting, the provider's scene is always the built one, byte for byte.
#[test]
fn seeded_edits_and_steps_always_end_at_the_built_scene() {
    let pick = |seed: u64, salt: u64, n: u64| ((seed ^ salt).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 33) % n;
    for seed in 1..=24u64 {
        let (mut s, sheet, [front, top]) = front_and_top();
        let edit =
            |s: &mut Session, view: u64, set: Value| s.execute("drawing.view.edit", &json!({"view": ids::encode(view), "set": set})).map(|_| ());
        for i in 0..12u64 {
            let along = 100.0 + 10.0 * pick(seed, 100 + i, 10) as f64;
            let scale = ["1:1", "1:2", "1:5"][pick(seed, 200 + i, 3) as usize];
            let tangent_edges = ["hide", "show"][pick(seed, 300 + i, 2) as usize];
            // Refused edits (nothing to undo, say) change nothing, which the end still checks.
            let _ = match pick(seed, i, 8) {
                0 => edit(&mut s, front, json!({"place": {"at": [along, 150.0]}})),
                1 => edit(&mut s, top, json!({"place": {"aligned": along + 100.0}})),
                2 => edit(&mut s, front, json!({"scale": scale})),
                3 => edit(&mut s, top, json!({"tangent_edges": tangent_edges})),
                4 => s.execute("edit.undo", &json!({})).map(|_| ()),
                5 => s.execute("edit.redo", &json!({})).map(|_| ()),
                6 => {
                    step(&mut s);
                    Ok(())
                }
                _ => extract(&mut s, sheet).map(drop),
            };
        }
        let end = settled(&mut s, sheet);
        assert!(end.stale.is_empty(), "seed {seed}");
        assert_eq!(bytes(&end.scene), bytes(&built(&s, sheet).scene), "seed {seed}");
    }
}

/// Setting a part as reference evaluates nothing, yet the views of it are out of date: its lines
/// join the reference parts', so no longer hide what is under them or stop at its own extents.
#[test]
fn a_change_of_reference_parts_redraws_the_views() {
    let mut design = Session::default();
    let mut part = |name: &str, body: &str| {
        let occurrence = run(&mut design, "component.create", json!({"name": name}))["occurrence"].as_u64().unwrap();
        run(&mut design, "solid.box", json!({"length": 20, "width": 20, "height": 10, "body_name": body, "operation": "new"}));
        run(&mut design, "component.activate", json!({"component": 0}));
        occurrence
    };
    part("Under", "BodyUnder");
    let over = part("Over", "BodyOver");
    run(&mut design, "occurrence.move", json!({"occurrence": over, "translate": [10, 0, 10]}));
    let (mut s, sheet, [front, top]) = front_and_top_of(design);
    let before = settled(&mut s, sheet);
    let drawing = json!({"drawing": ids::encode(s.drawing("test").unwrap().id)});
    let in_the_model = |s: &mut Session, id: &str, p: Value| {
        run(s, "drawing.close", json!({}));
        run(s, id, p);
        run(s, "drawing.open", drawing.clone());
    };
    in_the_model(&mut s, "occurrence.bom_type", json!({"occurrence": over, "bom_type": "reference"}));
    let stale = extract(&mut s, sheet).unwrap().stale;
    assert!(stale.len() == 2 && stale.contains(&front) && stale.contains(&top), "{stale:?}");
    let after = settled(&mut s, sheet);
    assert_ne!(bytes(&before.scene), bytes(&after.scene));
    let thin = |scene: &Scene| {
        let items = scene.groups.iter().filter(|g| g.kind == ObjectKind::View).flat_map(|g| &g.items);
        items.filter(|i| matches!(i, Item::Stroke { width_mm, .. } if *width_mm == crate::drawing_view::REFERENCE_MM)).count()
    };
    assert_eq!(thin(&before.scene), 0);
    assert!(thin(&after.scene) > 0);
    let scene = built(&s, sheet);
    assert_eq!(bytes(&scene.scene), bytes(&after.scene), "the provider and a build draw one thing");
    // Undone, the views draw again, as they were; redone, as they were then.
    in_the_model(&mut s, "edit.undo", json!({}));
    assert_eq!(extract(&mut s, sheet).unwrap().stale.len(), 2);
    assert_eq!(bytes(&settled(&mut s, sheet).scene), bytes(&before.scene));
    in_the_model(&mut s, "edit.redo", json!({}));
    assert_eq!(extract(&mut s, sheet).unwrap().stale.len(), 2);
    assert_eq!(bytes(&settled(&mut s, sheet).scene), bytes(&after.scene));
    // Set on its component, it draws the same.
    in_the_model(&mut s, "occurrence.bom_type", json!({"occurrence": over, "bom_type": null}));
    assert_eq!(bytes(&settled(&mut s, sheet).scene), bytes(&before.scene));
    in_the_model(&mut s, "component.metadata.set", json!({"component": "Over", "bom_type": "reference"}));
    assert_eq!(extract(&mut s, sheet).unwrap().stale.len(), 2);
    assert_eq!(bytes(&settled(&mut s, sheet).scene), bytes(&after.scene));
    // A saved view takes the setting: the reference part now covers what is under it.
    run(&mut s, "drawing.view.edit", json!({"view": ids::encode(top), "set": {"reference_hides_model": true}}));
    assert_ne!(bytes(&settled(&mut s, sheet).scene), bytes(&after.scene));
}

/// A run of text moves with its group: its start moves, and nothing else changes.
#[test]
fn text_moves_with_its_group() {
    let run = TextRun {
        font: [0; 32],
        scale: 0.005,
        origin: Vec2::new(1.0, 2.0),
        angle: 0.5,
        slant: 0.2,
        glyphs: vec![],
        text: String::new(),
        clusters: vec![],
        rtl: false,
    };
    let group = Group { object: 1, kind: ObjectKind::View, items: vec![Item::Text(run.clone())] };
    let Item::Text(placed) = &moved(&group, Vec2::new(10.0, 20.0)).items[0] else { panic!("a run") };
    assert_eq!(placed, &TextRun { origin: Vec2::new(11.0, 22.0), ..run });
}

/// A saved view's reference_shown_percent redraws that view alone, grows its boundary on the reference
/// part's side, leaves the model's strokes where they were, and goes back with an undo.
#[test]
fn a_saved_views_reference_shown_percent_grows_only_what_it_should() {
    let mut design = Session::default();
    let mut part = |name: &str, body: &str| {
        let occurrence = run(&mut design, "component.create", json!({"name": name}))["occurrence"].as_u64().unwrap();
        run(&mut design, "solid.box", json!({"length": 20, "width": 20, "height": 10, "body_name": body, "operation": "new"}));
        run(&mut design, "component.activate", json!({"component": 0}));
        occurrence
    };
    part("Under", "BodyUnder");
    let over = part("Over", "BodyOver");
    run(&mut design, "occurrence.move", json!({"occurrence": over, "translate": [10, 0, 10]}));
    run(&mut design, "occurrence.bom_type", json!({"occurrence": over, "bom_type": "reference"}));
    let (mut s, sheet, [_, top]) = front_and_top_of(design);
    let strokes = |scene: &Scene, reference: bool| {
        let items = scene.groups.iter().filter(|g| g.kind == ObjectKind::View).flat_map(|g| &g.items);
        let thin = |i: &&Item| matches!(i, Item::Stroke { width_mm, .. } if *width_mm == crate::drawing_view::REFERENCE_MM);
        let mut drawn: Vec<String> = items.filter(|i| matches!(i, Item::Stroke { .. }) && thin(i) == reference).map(|i| format!("{i:?}")).collect();
        drawn.sort();
        drawn
    };
    let before = settled(&mut s, sheet);
    let boundary = |s: &mut Session| run(s, "drawing.view.inspect", json!({"view": ids::encode(top)}))["boundary_mm"].clone();
    let (min, max) = {
        let b = boundary(&mut s);
        (b["min"].clone(), b["max"].clone())
    };
    run(&mut s, "drawing.view.edit", json!({"view": ids::encode(top), "set": {"reference_shown_percent": 100}}));
    assert_eq!(extract(&mut s, sheet).unwrap().stale, [top]);
    let after = settled(&mut s, sheet);
    assert_eq!(strokes(&before.scene, false), strokes(&after.scene, false), "the model stays where it is");
    assert_ne!(strokes(&before.scene, true), strokes(&after.scene, true));
    let grown = boundary(&mut s);
    assert_eq!(grown["min"], min, "the side it doesn't pass stays");
    assert_ne!(grown["max"], max);
    assert_eq!(bytes(&built(&s, sheet).scene), bytes(&after.scene), "the provider and a build draw one thing");
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(bytes(&settled(&mut s, sheet).scene), bytes(&before.scene));
}

fn cached_sketch_group(scene: &Scene, id: u64) -> &Group {
    scene.groups.iter().find(|group| group.kind == ObjectKind::Sketch && group.object == id).unwrap()
}

#[test]
fn sheet_sketch_provider_extract_is_read_only_and_steps_one_sketch_at_a_time() {
    let mut s = Session::default();
    run(&mut s, "sketch.create", json!({"plane":"XY"}));
    let model_sketch = s.active_sketch.unwrap();
    let line = run(&mut s, "sketch.line", json!({"points":[[0,0],[40,0]]}))["curves"][0].as_str().unwrap().to_string();
    run(&mut s, "sketch.finish", json!({}));
    run(&mut s, "solid.box", json!({"length":5,"width":5,"height":5}));
    run(&mut s, "sketch.edit", json!({"sketch":model_sketch}));
    run(&mut s, "sketch.drag", json!({"entity":format!("{line}.end"),"from":[40,0],"to":[45,3],"live":true}));
    assert!(s.model.is_stale(), "the source model has an unfinished live gesture");
    let drawing = run(&mut s, "drawing.create", json!({}));
    let first = run(&mut s, "drawing.sketch.create", json!({"owner":drawing["sheets"][0]["id"]}));
    let first = ids::decode(first["sketch"].as_str().unwrap()).unwrap();
    let sheet = s.drawing("test").unwrap().sheets[0].id;
    run(&mut s, "sketch.line", json!({"points":[[15,10],[55,10]],"infer":false}));
    run(&mut s, "sketch.finish", json!({}));
    let second = run(&mut s, "drawing.sketch.create", json!({"owner":ids::encode(sheet)}));
    let second = ids::decode(second["sketch"].as_str().unwrap()).unwrap();
    run(&mut s, "sketch.line", json!({"points":[[20,30],[60,30]],"infer":false}));
    run(&mut s, "sketch.finish", json!({}));
    let doc = s.doc.clone();
    let model = s.model.state();
    let history = (s.undo.len(), s.redo.len(), s.evaluations, s.revision);
    let mut order = vec![first, second];
    order.sort_unstable();
    let initial = extract(&mut s, sheet).unwrap();
    assert_eq!(initial.stale_sketches, order);
    assert!(initial.sketches.is_empty() && initial.scene.groups.iter().all(|group| group.kind != ObjectKind::Sketch));
    assert_eq!((s.drawing_scenes.sketch_solves, s.drawing_scenes.projections), (0, 0));
    assert!(step(&mut s));
    let one = extract(&mut s, sheet).unwrap();
    assert_eq!(one.stale_sketches, [order[1]]);
    assert_eq!(one.sketches.iter().map(|state| state.sketch).collect::<Vec<_>>(), [order[0]]);
    assert_eq!(s.drawing_scenes.sketch_solves, 1);
    assert!(!step(&mut s));
    let complete = extract(&mut s, sheet).unwrap();
    assert!(complete.stale_sketches.is_empty() && complete.stale.is_empty());
    assert_eq!(
        complete.sketches.iter().map(|state| (state.sketch, state.status.name())).collect::<Vec<_>>(),
        order.iter().map(|id| (*id, "drawn")).collect::<Vec<_>>()
    );
    assert_eq!(bytes(&complete.scene), bytes(&built(&s, sheet).scene));
    let again = extract(&mut s, sheet).unwrap();
    assert!(Arc::ptr_eq(&again.scene, &complete.scene) && again.generation == complete.generation);
    assert!(!step(&mut s));
    assert_eq!((s.drawing_scenes.sketch_solves, s.drawing_scenes.projections), (2, 0));
    assert!(Arc::ptr_eq(&doc, &s.doc) && Arc::ptr_eq(&model, &s.model.state()));
    assert_eq!((s.undo.len(), s.redo.len(), s.evaluations, s.revision), history);
    assert!(s.model.is_stale() && s.active_sketch == Some(model_sketch));
}

#[test]
fn sheet_sketch_provider_source_key_reads_parameters_feature_inputs_and_units_without_model_evaluation() {
    let (mut s, drawing, id) = stroke_editor();
    let sheet = s.drawing("test").unwrap().sheets[0].id;
    // This unitless ratio depends on document length defaults while its ParamDef stays the same:
    // (2 inches + 1 mm) / 1 mm = 51.8; in a millimetre design it is 3.
    s.doc_mut().set_param("Ratio", "(2 + 1 mm) / 1 mm", Some(""), None).unwrap();
    let feature = s.doc.features.iter().find(|feature| !feature.param_names.is_empty()).unwrap();
    let feature_id = feature.id;
    let feature_param = feature.param_names[0].clone();
    assert!(s.doc.params.iter().all(|param| param.name != feature_param));
    run(&mut s, "sketch.line", json!({"points":[["Inset + 5","10"],["Inset + 20","10"]],"infer":false}));
    run(&mut s, "sketch.line", json!({"points":[[format!("{feature_param} + 5"),"20"],[format!("{feature_param} + 20"),"20"]],"infer":false}));
    run(&mut s, "sketch.line", json!({"points":[["Ratio * 1 mm","30"],["Ratio * 1 mm + 10 mm","30"]],"infer":false}));
    run(&mut s, "sketch.finish", json!({}));
    let record = s.drawing("test").unwrap().sheet_sketch(id).unwrap().clone();
    let owner = s.drawing("test").unwrap().clone();
    let model = s.model.state();
    let history = (s.undo.len(), s.redo.len(), s.evaluations, s.revision);
    let was_stale = s.model.is_stale();
    let positions = |scene: &Scene| cached_sketch_group(scene, id).items.iter().map(|item| stroke_path(item).0[0].start().x).collect::<Vec<_>>();
    let mut previous = settled(&mut s, sheet);
    assert_eq!(positions(&previous.scene), [30.4, 45.0, 51.8]);
    for stage in 0..3 {
        match stage {
            0 => s.doc_mut().set_param("Inset", "2 in", Some("in"), None).unwrap(),
            1 => {
                let params = s.doc.params.clone();
                let feature = s.doc_mut().features.iter_mut().find(|feature| feature.id == feature_id).unwrap();
                let (_, expression, _) = feature.kind.inputs_mut().into_iter().next().unwrap();
                *expression = "60".into();
                assert_eq!(s.doc.params, params, "named feature inputs are not stored in doc.params");
            }
            _ => {
                let defs = s.doc.param_defs();
                s.doc_mut().units = "mm".into();
                assert_eq!(s.doc.param_defs(), defs, "this change needs the separate document-unit key");
            }
        }
        assert!(Arc::ptr_eq(s.doc.drawing(drawing).unwrap(), &owner));
        assert!(Arc::ptr_eq(s.doc.drawing(drawing).unwrap().sheet_sketch(id).unwrap(), &record));
        let doc = s.doc.clone();
        let count = s.drawing_scenes.sketch_solves;
        let pending = extract(&mut s, sheet).unwrap();
        assert_eq!(pending.stale_sketches, [id]);
        assert!(Arc::ptr_eq(&pending.scene, &previous.scene) && pending.generation == previous.generation);
        assert_eq!(s.drawing_scenes.sketch_solves, count, "extract only records stale work");
        assert!(!step(&mut s));
        previous = extract(&mut s, sheet).unwrap();
        assert!(previous.stale_sketches.is_empty());
        assert_eq!(s.drawing_scenes.sketch_solves, count + 1);
        assert_eq!(bytes(&previous.scene), bytes(&built(&s, sheet).scene));
        let expected = match stage {
            0 => [55.8, 45.0, 51.8],
            1 => [55.8, 65.0, 51.8],
            _ => [55.8, 65.0, 3.0],
        };
        for (actual, expected) in positions(&previous.scene).iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-9, "stage {stage}: {actual} != {expected}");
        }
        assert!(Arc::ptr_eq(&doc, &s.doc) && Arc::ptr_eq(&model, &s.model.state()));
        assert_eq!((s.undo.len(), s.redo.len(), s.evaluations, s.revision), history);
        assert_eq!(s.drawing_scenes.projections, 0);
        assert_eq!(s.model.is_stale(), was_stale, "paper resolution preserves the source model's evaluation state");
    }
}

#[test]
fn sheet_sketch_provider_keeps_mixed_paper_order_and_reuses_view_moves_and_editor_state() {
    let mut design = Session::default();
    run(&mut design, "solid.box", json!({"length":100,"width":50,"height":20,"body_name":"Block"}));
    run(&mut design, "document.units", json!({"units":"in"}));
    let (mut s, sheet, [front, top]) = front_and_top_of(design);
    run(&mut s, "drawing.view.edit", json!({"view":ids::encode(top),"set":{"scale":"1:4"}}));
    let made = run(&mut s, "drawing.sketch.create", json!({"owner":ids::encode(sheet)}));
    let id = ids::decode(made["sketch"].as_str().unwrap()).unwrap();
    run(&mut s, "sketch.rectangle.two_point", json!({"p0":[15,10],"p1":[55,30]}));
    run(&mut s, "sketch.finish", json!({}));
    let before = settled(&mut s, sheet);
    let expected = built(&s, sheet);
    assert_eq!(bytes(&before.scene), bytes(&expected.scene));
    let mut views = [front, top];
    views.sort_unstable();
    assert_eq!(
        before.scene.groups.iter().skip(3).map(|group| (group.object, group.kind)).collect::<Vec<_>>(),
        [(views[0], ObjectKind::View), (views[1], ObjectKind::View), (id, ObjectKind::Sketch)]
    );
    let (lo, hi, perimeter) = reach(cached_sketch_group(&before.scene, id));
    assert!(lo.dist(Vec2::new(15.0, 10.0)) < 1e-9 && hi.dist(Vec2::new(55.0, 30.0)) < 1e-9 && (perimeter - 120.0).abs() < 1e-9);
    let counts = (s.drawing_scenes.projections, s.drawing_scenes.sketch_solves);
    let sketch_bytes = serde_json::to_string(cached_sketch_group(&before.scene, id)).unwrap();
    run(&mut s, "drawing.view.edit", json!({"view":ids::encode(front),"set":{"place":{"at":[200,150]}}}));
    let moved = extract(&mut s, sheet).unwrap();
    assert!(moved.stale.is_empty() && moved.stale_sketches.is_empty());
    assert_eq!((s.drawing_scenes.projections, s.drawing_scenes.sketch_solves), counts);
    assert_eq!(serde_json::to_string(cached_sketch_group(&moved.scene, id)).unwrap(), sketch_bytes);
    assert_eq!(bytes(&moved.scene), bytes(&built(&s, sheet).scene));
    run(&mut s, "drawing.sketch.edit", json!({"sketch":ids::encode(id)}));
    run(&mut s, "sketch.finish", json!({}));
    let editing_ended = extract(&mut s, sheet).unwrap();
    assert!(editing_ended.stale_sketches.is_empty() && Arc::ptr_eq(&editing_ended.scene, &moved.scene));
    assert_eq!((s.drawing_scenes.projections, s.drawing_scenes.sketch_solves), counts);
}

#[test]
fn sheet_sketch_provider_reuses_sheets_and_retires_deleted_undone_and_closed_owners() {
    let (mut s, drawing, first) = stroke_editor();
    let first_sheet = s.drawing("test").unwrap().sheets[0].id;
    run(&mut s, "sketch.line", json!({"points":[[10,10],[30,10]],"infer":false}));
    run(&mut s, "sketch.finish", json!({}));
    let second_sheet = run(&mut s, "drawing.sheet.add", json!({"name":"Second"}));
    let second_sheet = ids::decode(second_sheet["sheet"].as_str().unwrap()).unwrap();
    let second = run(&mut s, "drawing.sketch.create", json!({"owner":ids::encode(second_sheet)}));
    let second = ids::decode(second["sketch"].as_str().unwrap()).unwrap();
    run(&mut s, "sketch.line", json!({"points":[[60,60],[90,60]],"infer":false}));
    run(&mut s, "sketch.finish", json!({}));
    let first_scene = settled(&mut s, first_sheet);
    let second_scene = settled(&mut s, second_sheet);
    assert_eq!(s.drawing_scenes.sketch_solves, 2);
    let again = extract(&mut s, first_sheet).unwrap();
    assert!(again.stale_sketches.is_empty());
    assert_eq!(bytes(&again.scene), bytes(&first_scene.scene));
    assert_eq!(s.drawing_scenes.sketch_solves, 2, "switching sheets retains settled drawing entries");
    assert!(again.scene.groups.iter().all(|group| group.kind != ObjectKind::Sketch || group.object == first));
    extract(&mut s, second_sheet).unwrap();
    run(&mut s, "drawing.sheet.delete", json!({"sheet":ids::encode(second_sheet),"contents":"delete"}));
    assert!(!step(&mut s), "a deleted extracted sheet supplies no work");
    assert!(extract(&mut s, second_sheet).is_err());
    let remaining = settled(&mut s, first_sheet);
    assert!(remaining.scene.groups.iter().all(|group| group.object != second));
    assert!(!s.drawing_scenes.sketches.contains_key(&second));
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(extract(&mut s, second_sheet).unwrap().stale_sketches, [second]);
    assert_eq!(bytes(&settled(&mut s, second_sheet).scene), bytes(&second_scene.scene));
    run(&mut s, "drawing.close", json!({}));
    assert!(s.drawing_scenes.sketches.is_empty() && s.drawing_scenes.source.is_none() && s.drawing_scenes.context.is_none());
    assert!(!step(&mut s));
    run(&mut s, "drawing.open", json!({"drawing":ids::encode(drawing)}));
    assert_eq!(extract(&mut s, first_sheet).unwrap().stale_sketches, [first]);
    assert_eq!(bytes(&settled(&mut s, first_sheet).scene), bytes(&first_scene.scene));
    let document = solidcraft_doc::Document::from_json(&s.doc.to_json()).unwrap();
    let mut reopened = Session::new(document);
    run(&mut reopened, "drawing.open", json!({"drawing":ids::encode(drawing)}));
    assert_eq!(bytes(&settled(&mut reopened, second_sheet).scene), bytes(&second_scene.scene));
}

#[test]
fn sheet_sketch_provider_caches_named_source_failures_and_recovers_without_global_error_blocking() {
    let (mut s, drawing, id) = stroke_editor();
    let sheet = s.drawing("test").unwrap().sheets[0].id;
    run(&mut s, "sketch.line", json!({"points":[["Inset + 5","10"],["Inset + 20","10"]],"infer":false}));
    run(&mut s, "sketch.finish", json!({}));
    s.doc_mut().params.iter_mut().find(|param| param.name == "Inset").unwrap().expr = "Missing + 1 mm".into();
    let doc = s.doc.clone();
    let model = s.model.state();
    let history = (s.evaluations, s.undo.len(), s.redo.len());
    let pending = extract(&mut s, sheet).unwrap();
    assert_eq!(pending.stale_sketches, [id]);
    assert!(pending.sketches.is_empty() && s.drawing_scenes.sketch_solves == 0);
    assert!(!step(&mut s));
    let failed = extract(&mut s, sheet).unwrap();
    assert!(failed.stale_sketches.is_empty());
    assert!(matches!(&failed.sketches[0].status, SketchStatus::Failed(why) if why.contains(&ids::encode(id)) && why.contains("Inset")));
    assert!(failed.scene.groups.iter().all(|group| group.kind != ObjectKind::Sketch));
    for _ in 0..3 {
        assert!(!step(&mut s));
        let cached = extract(&mut s, sheet).unwrap();
        assert!(Arc::ptr_eq(&cached.scene, &failed.scene) && cached.generation == failed.generation);
    }
    assert_eq!(s.drawing_scenes.sketch_solves, 1);
    assert!(Arc::ptr_eq(&doc, &s.doc) && Arc::ptr_eq(&model, &s.model.state()));
    assert_eq!((s.evaluations, s.undo.len(), s.redo.len()), history);
    s.doc_mut().set_param("Inset", "1 in", Some("in"), None).unwrap();
    let restored = settled(&mut s, sheet);
    assert_eq!(restored.sketches[0].status, SketchStatus::Drawn);
    s.doc_mut().params.push(solidcraft_doc::Param {
        name: "Unrelated".into(),
        expr: "NoSuchParameter".into(),
        unit: "mm".into(),
        comment: String::new(),
        model: false,
    });
    assert!(s.doc.param_values().1.contains_key("Unrelated"));
    assert_eq!(settled(&mut s, sheet).sketches[0].status, SketchStatus::Drawn);
    let mut no_source = (**s.drawing("test").unwrap()).clone();
    let sources = std::mem::take(&mut no_source.sources);
    no_source.validate().unwrap();
    s.doc_mut().put_drawing(Arc::new(no_source));
    let failed = settled(&mut s, sheet);
    assert!(
        matches!(&failed.sketches[0].status, SketchStatus::Failed(why) if why.contains("declared design source") && why.contains(&ids::encode(id)))
    );
    let count = s.drawing_scenes.sketch_solves;
    assert!(!step(&mut s));
    assert_eq!(s.drawing_scenes.sketch_solves, count);
    let mut repaired = (**s.doc.drawing(drawing).unwrap()).clone();
    repaired.sources = sources;
    s.doc_mut().put_drawing(Arc::new(repaired));
    assert_eq!(settled(&mut s, sheet).sketches[0].status, SketchStatus::Drawn);
}

#[test]
fn sheet_sketch_provider_invalid_header_failures_follow_repairs_with_the_same_sketch_arc() {
    let (mut s, _, id) = stroke_editor();
    let sheet = s.drawing("test").unwrap().sheets[0].id;
    run(&mut s, "sketch.line", json!({"points":[[10,10],[30,10]],"infer":false}));
    run(&mut s, "sketch.finish", json!({}));
    let record = s.drawing("test").unwrap().sheet_sketch(id).unwrap().clone();
    let before = settled(&mut s, sheet);
    let mut invalid = (**s.drawing("test").unwrap()).clone();
    let name = std::mem::take(&mut invalid.name);
    s.doc_mut().put_drawing(Arc::new(invalid));
    let failed = settled(&mut s, sheet);
    assert!(matches!(&failed.sketches[0].status, SketchStatus::Failed(why) if why.contains(&ids::encode(id))));
    assert!(failed.scene.groups.iter().all(|group| group.kind != ObjectKind::Sketch));
    let count = s.drawing_scenes.sketch_solves;
    assert!(!step(&mut s));
    assert_eq!(s.drawing_scenes.sketch_solves, count);
    let mut repaired = (**s.drawing("test").unwrap()).clone();
    repaired.name = name;
    assert!(Arc::ptr_eq(repaired.sheet_sketch(id).unwrap(), &record));
    s.doc_mut().put_drawing(Arc::new(repaired));
    let after = settled(&mut s, sheet);
    assert_eq!(after.sketches[0].status, SketchStatus::Drawn);
    assert_eq!(bytes(&after.scene), bytes(&before.scene));
    assert_eq!(s.drawing_scenes.sketch_solves, count + 1);
}

#[test]
fn sheet_sketch_assembly_budget_names_only_the_excluded_sketch_and_can_recover() {
    let (s, sheet, _) = front_and_top();
    let d = s.drawing("test").unwrap();
    let record = d.sheets.iter().find(|record| record.id == sheet).unwrap();
    let group = |id, kind, n: usize| Group {
        object: id,
        kind,
        items: vec![Item::Stroke {
            path: (0..n).map(|i| Seg2::Line { a: Vec2::new(i as f64 * 1e-3, 0.0), b: Vec2::new((i + 1) as f64 * 1e-3, 0.0) }).collect(),
            closed: false,
            width_mm: 0.5,
        }],
    };
    let left = MAX_RECORDS - border(&s, sheet).iter().map(Group::records).sum::<usize>();
    let view = Drawn { group: Some(group(10, ObjectKind::View, left - 12)), status: Status::Drawn, stats: None };
    let sketches = [
        DrawnSketch { group: Some(group(101, ObjectKind::Sketch, 10)), status: SketchStatus::Drawn },
        DrawnSketch { group: Some(group(102, ObjectKind::Sketch, 8)), status: SketchStatus::Drawn },
    ];
    let drawn = || sketches.iter().enumerate().map(|(i, drawn)| (101 + i as u64, drawn));
    let (scene, views, states) = assemble_with_sketches(record, std::iter::once((10, Vec2::ZERO, &view)), drawn()).unwrap();
    assert_eq!(views[0].status, Status::Drawn);
    assert!(matches!(&states[0].status, SketchStatus::Failed(why) if why.contains(&ids::encode(101)) && why.contains(&MAX_RECORDS.to_string())));
    assert_eq!(states[1].status, SketchStatus::Drawn);
    assert_eq!(scene.groups.iter().map(Group::records).sum::<usize>(), MAX_RECORDS);
    assert_eq!(scene.groups.iter().skip(3).map(|group| group.object).collect::<Vec<_>>(), [10, 102]);
    let (_, _, recovered) = assemble_with_sketches(record, std::iter::empty::<(u64, Vec2, Drawn)>(), drawn()).unwrap();
    assert!(recovered.iter().all(|state| state.status == SketchStatus::Drawn), "composition budget failures are not permanent geometry failures");
}

#[test]
fn sheet_sketch_strokes_refuse_tiny_arcs_that_shared_pieces_widen() {
    use solidcraft_sketch::CurveKind;
    let (mut s, owner, id) = stroke_editor();
    run(&mut s, "sketch.line", json!({"points":[[10,10],[30,10]],"infer":false}));
    let mut solved = s.doc.solve_sheet_sketch(owner, id).unwrap();
    solved.sketch.curves[0].kind = CurveKind::Arc { c: 0, a: 1, b: 2 };
    for (a, b) in [(Vec2::new(1000.0, 0.0), Vec2::new(1000.0, 5e-10)), (Vec2::new(-1000.0, 2.5e-10), Vec2::new(-1000.0, -2.5e-10))] {
        solved.sketch.points[1].pos = a;
        solved.sketch.points[2].pos = b;
        let why = sketch::paper_group(&solved, 0.5).unwrap_err().to_string();
        assert!(why.contains("sweep") && why.contains(&solved.sketch.curves[0].id), "{why}");
        // The opposite travel is a real almost-full turn, which must remain printable.
        solved.sketch.points[1].pos = b;
        solved.sketch.points[2].pos = a;
        let group = sketch::paper_group(&solved, 0.5).unwrap();
        let (path, closed, _) = stroke_path(&group.items[0]);
        assert!(!closed);
        assert!(matches!(path,[Seg2::Arc{sweep,..}] if *sweep > std::f64::consts::TAU-1e-10 && *sweep <=std::f64::consts::TAU));
    }
    solved.sketch.points[1].pos = Vec2::new(1000.0, 0.0);
    solved.sketch.points[2].pos = solved.sketch.points[1].pos;
    let group = sketch::paper_group(&solved, 0.5).unwrap();
    let (path, closed, _) = stroke_path(&group.items[0]);
    assert!(closed && matches!(path,[Seg2::Arc{sweep,..}] if *sweep == std::f64::consts::TAU));
}

#[test]
fn sheet_sketch_strokes_name_an_arc_past_paper_tolerance_even_when_solved() {
    use solidcraft_sketch::CurveKind;
    let (mut s, owner, id) = stroke_editor();
    run(&mut s, "sketch.arc.center_point", json!({"center":[30,40],"start":[40,40],"sweep":90}));
    let mut drawing = (**s.doc.drawing(owner).unwrap()).clone();
    let record = Arc::make_mut(&mut drawing.sketches[0]);
    let CurveKind::Arc { b, .. } = record.sketch.curves[0].kind else { panic!("not an arc") };
    record.sketch.points[b].pos.y += 5e-8;
    for point in &mut record.sketch.points {
        point.fixed = true;
    }
    record.sketch.curves[0].fixed = true;
    drawing.validate().unwrap();
    s.doc_mut().put_drawing(Arc::new(drawing));
    let before = s.doc.clone();
    let solved = s.doc.solve_sheet_sketch(owner, id).unwrap();
    assert!(solved.report.keeps_orientation());
    assert!(solved.report.max_residual > solidcraft_geom::EPS && solved.report.max_residual < 1e-6, "{:?}", solved.report);
    assert_eq!(solved.sketch.points, before.drawing(owner).unwrap().sheet_sketch(id).unwrap().sketch.points);
    let why = sketch::paper_group(&solved, 0.5).unwrap_err().to_string();
    assert!(why.contains("paper endpoint tolerance") && why.contains(&solved.sketch.curves[0].id), "{why}");
    assert!(Arc::ptr_eq(&before, &s.doc));
}
