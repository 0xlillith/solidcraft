use std::f64::consts::{FRAC_PI_4, PI, SQRT_2};

use crate::hatch::budget::{Budget, Limits};
use crate::hatch::clip::{ClippedLine, Frame, PreparedRegion};
use crate::hatch::generate::generate_with_limits;
use crate::hatch::phase::{Dyadic, Param};
use crate::hatch::{self, Family, Invalid, Mark, Pattern};
use crate::{Loop2, MAX_BEZIER_CTRL, Region2, Seg2, Vec2};

const TOL: f64 = 1e-9;
type Segment = (Vec2, Vec2);

fn p(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

fn polygon(points: &[(f64, f64)]) -> Loop2 {
    let vertices: Vec<Vec2> = points.iter().map(|&(x, y)| p(x, y)).collect();
    let segs = vertices.iter().zip(vertices.iter().cycle().skip(1)).map(|(&a, &b)| Seg2::Line { a, b }).collect();
    Loop2 { segs }
}

fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Loop2 {
    polygon(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

fn plate() -> Region2 {
    Region2 { outer: rectangle(0.0, 0.0, 40.0, 20.0), holes: vec![] }
}

fn perforated_plate() -> Region2 {
    Region2 { outer: plate().outer, holes: vec![polygon(&[(10.0, 5.0), (10.0, 15.0), (20.0, 15.0), (20.0, 5.0)])] }
}

fn assert_point(actual: Vec2, expected: Vec2) {
    assert!(actual.dist(expected) <= TOL, "actual {actual:?}, expected {expected:?}");
}

fn assert_spans(line: &ClippedLine, expected: &[(f64, f64)]) {
    assert_eq!(line.spans().len(), expected.len());
    for (span, &(start, end)) in line.spans().iter().zip(expected) {
        assert!(span.start < span.end);
        assert!((span.start - start).abs() <= TOL, "actual start {}, expected {start}", span.start);
        assert!((span.end - end).abs() <= TOL, "actual end {}, expected {end}", span.end);
    }
}

fn horizontal_with_budget(region: &Region2, y: f64, budget: &mut Budget) -> Result<ClippedLine, Invalid> {
    let prepared = PreparedRegion::new(region, budget)?;
    let frame = Frame::new(Vec2::ZERO, Vec2::X)?;
    prepared.clip(&frame, y, budget)
}

fn horizontal(region: &Region2, y: f64) -> (ClippedLine, Budget) {
    let mut budget = Budget::default();
    let line = horizontal_with_budget(region, y, &mut budget).unwrap();
    (line, budget)
}

fn order(a: &Vec2, b: &Vec2) -> std::cmp::Ordering {
    a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y))
}

fn canonical(segments: &[Segment]) -> Vec<Segment> {
    let mut sorted: Vec<Segment> = segments.iter().map(|&(a, b)| if order(&a, &b).is_gt() { (b, a) } else { (a, b) }).collect();
    sorted.sort_by(|(a, b), (c, d)| order(a, c).then_with(|| order(b, d)));
    sorted
}

fn assert_segments(actual: &[Segment], expected: &[Segment], total_length: f64) {
    assert_eq!(actual.len(), expected.len());
    // Rounded points on x=0 need not sort in the same order as exact points.
    // Consume each matching segment once, preserving multiplicity without
    // requiring floating-point tie ordering to reproduce the analytic order.
    let mut unmatched = canonical(actual);
    for (c, d) in canonical(expected) {
        let found = unmatched.iter().position(|(a, b)| a.dist(c) <= TOL && b.dist(d) <= TOL);
        let Some(index) = found else { panic!("missing segment {c:?} -> {d:?}; remaining {unmatched:?}") };
        unmatched.swap_remove(index);
    }
    let length: f64 = actual.iter().map(|(a, b)| a.dist(*b)).sum();
    assert!((length - total_length).abs() <= TOL, "actual length {length}, expected {total_length}");
}

fn diagonal_segments(region: &Region2, origin: Vec2, spacing: f64, members: &[i32]) -> Vec<Segment> {
    let mut budget = Budget::default();
    let prepared = PreparedRegion::new(region, &mut budget).unwrap();
    let frame = Frame::new(origin, p(1.0, 1.0)).unwrap();
    let mut segments = Vec::new();
    for &k in members {
        let offset = f64::from(k) * spacing;
        let line = prepared.clip(&frame, offset, &mut budget).unwrap();
        for span in line.spans() {
            assert!(span.start < span.end);
            // Independent 45-degree coordinate map, not Frame::point under test.
            let point = |t: f64| origin + p((t - offset) / SQRT_2, (t + offset) / SQRT_2);
            segments.push((point(span.start), point(span.end)));
        }
    }
    segments
}

fn generated_diagonal_segments(region: &Region2) -> Vec<Segment> {
    let pattern = Pattern {
        name: "ORIGINAL_45".into(),
        description: "Independent rectangle and hole oracles".into(),
        families: vec![Family { angle: FRAC_PI_4, origin: Vec2::ZERO, delta_x: 0.0, delta_y: 5.0, dashes: vec![] }],
    };
    let before = pattern.clone();
    let marks = hatch::generate(&pattern, region).unwrap();
    assert_eq!(pattern, before);
    marks
        .into_iter()
        .map(|mark| match mark {
            Mark::Line { a, b } => (a, b),
            Mark::Dot { .. } => panic!("a continuous family must not produce dots"),
        })
        .collect()
}

fn diagonal_expected() -> Vec<Segment> {
    let r = SQRT_2;
    // y-x=5*sqrt(2)*k, intersected by hand with [0,40] x [0,20].
    vec![
        (p(25.0 * r, 0.0), p(40.0, 40.0 - 25.0 * r)),
        (p(20.0 * r, 0.0), p(40.0, 40.0 - 20.0 * r)),
        (p(15.0 * r, 0.0), p(40.0, 40.0 - 15.0 * r)),
        (p(10.0 * r, 0.0), p(20.0 + 10.0 * r, 20.0)),
        (p(5.0 * r, 0.0), p(20.0 + 5.0 * r, 20.0)),
        (p(0.0, 0.0), p(20.0, 20.0)),
        (p(0.0, 5.0 * r), p(20.0 - 5.0 * r, 20.0)),
        (p(0.0, 10.0 * r), p(20.0 - 10.0 * r, 20.0)),
    ]
}

fn perforated_expected() -> Vec<Segment> {
    let r = SQRT_2;
    // The hole [10,20] x [5,15] cuts only members -2, -1 and 0.
    vec![
        (p(25.0 * r, 0.0), p(40.0, 40.0 - 25.0 * r)),
        (p(20.0 * r, 0.0), p(40.0, 40.0 - 20.0 * r)),
        (p(15.0 * r, 0.0), p(40.0, 40.0 - 15.0 * r)),
        (p(10.0 * r, 0.0), p(5.0 + 10.0 * r, 5.0)),
        (p(20.0, 20.0 - 10.0 * r), p(20.0 + 10.0 * r, 20.0)),
        (p(5.0 * r, 0.0), p(5.0 + 5.0 * r, 5.0)),
        (p(20.0, 20.0 - 5.0 * r), p(20.0 + 5.0 * r, 20.0)),
        (p(0.0, 0.0), p(10.0, 10.0)),
        (p(15.0, 15.0), p(20.0, 20.0)),
        (p(0.0, 5.0 * r), p(20.0 - 5.0 * r, 20.0)),
        (p(0.0, 10.0 * r), p(20.0 - 10.0 * r, 20.0)),
    ]
}

fn permute(region: &Region2, reverse: bool, shift: usize, translation: Vec2) -> Region2 {
    let change = |boundary: &Loop2| {
        let mut vertices: Vec<Vec2> = boundary
            .segs
            .iter()
            .map(|seg| match seg {
                Seg2::Line { a, .. } => *a + translation,
                _ => panic!("this test fixture contains only lines"),
            })
            .collect();
        if reverse {
            vertices.reverse();
        }
        let len = vertices.len();
        vertices.rotate_left(shift % len);
        let coords: Vec<_> = vertices.iter().map(|v| (v.x, v.y)).collect();
        polygon(&coords)
    };
    Region2 { outer: change(&region.outer), holes: region.holes.iter().map(change).collect() }
}

#[test]
fn frame_uses_a_global_origin_and_normalizes_the_tangent_once() {
    let origin = p(11.0, -7.0);
    let frame = Frame::new(origin, p(6.0, 8.0)).unwrap();
    assert_point(frame.tangent(), p(0.6, 0.8));
    assert_point(frame.normal(), p(-0.8, 0.6));
    // O + 5*(-.8,.6) + 10*(.6,.8) = (13,4).
    assert_point(frame.point(5.0, 10.0).unwrap(), p(13.0, 4.0));
    assert_point(frame.point(-5.0, -10.0).unwrap(), p(9.0, -18.0));
}

#[test]
fn original_rectangle_at_45_degrees_has_eight_analytic_segments() {
    let region = plate();
    let before = region.clone();
    let actual = diagonal_segments(&region, Vec2::ZERO, 5.0, &[-5, -4, -3, -2, -1, 0, 1, 2]);
    assert_segments(&actual, &diagonal_expected(), 220.0 * SQRT_2 - 150.0);
    assert_eq!(region, before);
}

#[test]
fn hole_splits_three_diagonal_segments_and_preserves_exact_remaining_lengths() {
    let region = perforated_plate();
    let before = region.clone();
    let actual = diagonal_segments(&region, Vec2::ZERO, 5.0, &[-5, -4, -3, -2, -1, 0, 1, 2]);
    assert_segments(&actual, &perforated_expected(), 185.0 * SQRT_2 - 120.0);
    assert_eq!(region, before);
}

#[test]
fn public_generator_enumerates_all_eight_rectangle_segments_at_45_degrees() {
    let region = plate();
    let before = region.clone();
    // The public path chooses members and constructs its own sin/cos frame.
    let actual = generated_diagonal_segments(&region);
    assert_segments(&actual, &diagonal_expected(), 220.0 * SQRT_2 - 150.0);
    assert_eq!(region, before);
}

#[test]
fn public_generator_preserves_all_eleven_segments_around_the_hole() {
    let region = perforated_plate();
    let before = region.clone();
    let actual = generated_diagonal_segments(&region);
    assert_segments(&actual, &perforated_expected(), 185.0 * SQRT_2 - 120.0);
    assert_eq!(region, before);
}

#[test]
fn cyclic_starts_reverse_winding_translation_and_signed_member_spacing_preserve_geometry() {
    for reverse in [false, true] {
        for shift in [0, 1, 3] {
            for translation in [Vec2::ZERO, p(11.0, -7.0)] {
                let region = permute(&perforated_plate(), reverse, shift, translation);
                let expected: Vec<_> = perforated_expected().into_iter().map(|(a, b)| (a + translation, b + translation)).collect();
                for (spacing, members) in [(5.0, [-5, -4, -3, -2, -1, 0, 1, 2]), (-5.0, [-2, -1, 0, 1, 2, 3, 4, 5])] {
                    let actual = diagonal_segments(&region, translation, spacing, &members);
                    assert_segments(&actual, &expected, 185.0 * SQRT_2 - 120.0);
                }
            }
        }
    }
}

#[test]
fn conservative_normal_bounds_cover_the_original_vertices_without_rotating_the_origin() {
    let region = plate();
    let mut budget = Budget::default();
    let prepared = PreparedRegion::new(&region, &mut budget).unwrap();
    let frame = Frame::new(p(11.0, -7.0), p(1.0, 1.0)).unwrap();
    let (lo, hi) = prepared.normal_bounds(&frame, &mut budget).unwrap();
    assert!(lo.is_finite() && hi.is_finite() && lo <= hi);
    // (P-O).n = (y-x+18)/sqrt(2), for the four original corners.
    for projection in [18.0 / SQRT_2, -22.0 / SQRT_2, -2.0 / SQRT_2, 38.0 / SQRT_2] {
        assert!(lo <= projection + TOL && hi >= projection - TOL);
    }
    // In the axis-aligned frame the extrema are exactly 0-(-7) and 20-(-7).
    // An inward bound is wrong even if it lies within the comparison tolerance.
    let axis_frame = Frame::new(p(11.0, -7.0), Vec2::X).unwrap();
    let (lo, hi) = prepared.normal_bounds(&axis_frame, &mut budget).unwrap();
    assert!(lo.is_finite() && hi.is_finite() && lo <= 7.0 && hi >= 27.0);
}

#[test]
fn outer_boundary_runs_and_vertex_only_tangencies_have_no_ink_or_dots() {
    for y in [0.0, 20.0, -1.0, 21.0] {
        let (line, mut budget) = horizontal(&plate(), y);
        assert_spans(&line, &[]);
        for t in [0.0, 10.0, 40.0] {
            assert!(!line.contains_strict(t, &mut budget).unwrap());
        }
    }
    let (interior, mut budget) = horizontal(&plate(), 10.0);
    assert_spans(&interior, &[(0.0, 40.0)]);
    assert!(!interior.contains_strict(0.0, &mut budget).unwrap());
    assert!(interior.contains_strict(20.0, &mut budget).unwrap());
    assert!(!interior.contains_strict(40.0, &mut budget).unwrap());
    let triangle = Region2 { outer: polygon(&[(0.0, 0.0), (8.0, 0.0), (4.0, 4.0)]), holes: vec![] };
    let (tangent, mut budget) = horizontal(&triangle, 4.0);
    assert_spans(&tangent, &[]);
    assert!(!tangent.contains_strict(4.0, &mut budget).unwrap());
}

#[test]
fn concave_collinear_shelf_and_hole_edge_are_subtracted_from_interior_spans() {
    let region = Region2 {
        outer: polygon(&[(0.0, 0.0), (12.0, 0.0), (12.0, 8.0), (8.0, 8.0), (8.0, 4.0), (4.0, 4.0), (4.0, 8.0), (0.0, 8.0)]),
        holes: vec![rectangle(1.0, 2.0, 3.0, 4.0)],
    };
    for reverse in [false, true] {
        let input = permute(&region, reverse, 3, Vec2::ZERO);
        let (line, mut budget) = horizontal(&input, 4.0);
        assert_spans(&line, &[(0.0, 1.0), (3.0, 4.0), (8.0, 12.0)]);
        for t in [0.0, 1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0] {
            assert!(!line.contains_strict(t, &mut budget).unwrap(), "boundary or exterior at {t}");
        }
        for t in [0.5, 3.5, 10.0] {
            assert!(line.contains_strict(t, &mut budget).unwrap());
        }
        let length: f64 = line.spans().iter().map(|span| span.end - span.start).sum();
        assert_eq!(length, 6.0);
    }
}

#[test]
fn reflex_tangent_contact_splits_spans_without_a_parity_change() {
    let region = Region2 { outer: polygon(&[(0.0, 0.0), (2.0, 0.0), (4.0, 4.0), (6.0, 0.0), (8.0, 0.0), (8.0, 6.0), (0.0, 6.0)]), holes: vec![] };
    for reverse in [false, true] {
        let input = permute(&region, reverse, 2, Vec2::ZERO);
        let (line, mut budget) = horizontal(&input, 4.0);
        assert_spans(&line, &[(0.0, 4.0), (4.0, 8.0)]);
        assert!(line.contains_strict(3.0, &mut budget).unwrap());
        assert!(!line.contains_strict(4.0, &mut budget).unwrap());
        assert!(line.contains_strict(5.0, &mut budget).unwrap());
    }
}

#[test]
fn a_representable_skinny_hole_is_not_joined_by_a_geometric_epsilon() {
    // 2^-40 is exactly representable and far below geom::EPS. All edges are
    // axis-aligned, so this case does not need a tolerance-based repair.
    let width = 1.0 / 1_099_511_627_776.0;
    let region = Region2 { outer: rectangle(0.0, 0.0, 8.0, 6.0), holes: vec![rectangle(4.0, 2.0, 4.0 + width, 4.0)] };
    let (line, mut budget) = horizontal(&region, 3.0);
    let spans: Vec<_> = line.spans().iter().map(|span| (span.start, span.end)).collect();
    assert_eq!(spans, vec![(0.0, 4.0), (4.0 + width, 8.0)]);
    assert!(!line.contains_strict(4.0, &mut budget).unwrap());
    assert!(!line.contains_strict(4.0 + width / 2.0, &mut budget).unwrap());
    assert!(!line.contains_strict(4.0 + width, &mut budget).unwrap());
    assert!(line.contains_strict(4.0 - width, &mut budget).unwrap());
    assert!(line.contains_strict(4.0 + 2.0 * width, &mut budget).unwrap());
}

#[test]
fn underflowed_product_residual_is_not_an_exact_boundary_contact() {
    // u=2^-52, a=(2-u)*2^-485, b=(2-u)*2^-486, v=2^-969-2^-1021.
    let a = f64::from_bits(0x21af_ffff_ffff_ffff);
    let b = f64::from_bits(0x219f_ffff_ffff_ffff);
    let v = f64::from_bits(0x035f_ffff_ffff_fffe);
    // Exact real arithmetic gives a*b = v + 2^-1075. The product rounds to v
    // and its positive residual rounds to zero (half the smallest subnormal).
    // Thus the vertex (b,v) is below the line, not exactly on its boundary.
    assert_eq!(a, 2.0 * b);
    assert_eq!(a * b, v);
    assert_eq!(a.mul_add(b, -v), 0.0);
    let region = Region2 { outer: rectangle(b, v, a, 1.0), holes: vec![] };
    let before = region.clone();
    let mut budget = Budget::default();
    let prepared = PreparedRegion::new(&region, &mut budget).unwrap();
    let frame = Frame::new(Vec2::ZERO, p(1.0, a)).unwrap();
    assert_eq!(prepared.clip(&frame, 0.0, &mut budget).unwrap_err(), Invalid("unresolved clipping arithmetic"));

    // At this angle cos(a) rounds to 1 and sin(a) to a. The public generator
    // must also refuse the uncertainty, rather than return a false contact.
    let pattern = Pattern {
        name: "RESIDUAL_UNDERFLOW".into(),
        description: "Independent binary64 projection adversary".into(),
        families: vec![Family { angle: a, origin: Vec2::ZERO, delta_x: 0.0, delta_y: 2.0, dashes: vec![] }],
    };
    pattern.validate().unwrap();
    assert_eq!(hatch::generate(&pattern, &region).unwrap_err(), Invalid("unresolved clipping arithmetic"));
    assert_eq!(region, before);
}

#[test]
fn strict_membership_queries_charge_work_even_for_an_empty_clipped_line() {
    let region = Region2 { outer: rectangle(0.0, 0.0, 1.0, 1.0), holes: vec![] };
    let (line, _preparation_budget) = horizontal(&region, 2.0);
    assert_spans(&line, &[]);
    // Isolate the query's entry charge: there are no spans to visit.
    let mut query_budget = Budget::new(Limits { work: 0, ..Limits::default() });
    assert_eq!(line.contains_strict(0.0, &mut query_budget), Err(Invalid("geometry work budget exhausted")));
}

#[test]
fn non_line_boundaries_are_refused_instead_of_flattened() {
    let mut controls = [Vec2::ZERO; MAX_BEZIER_CTRL];
    controls[..5].copy_from_slice(&[p(0.0, 0.0), p(2.0, -1.0), p(4.0, -2.0), p(6.0, -1.0), p(8.0, 0.0)]);
    for curve in [
        Seg2::Arc { center: p(4.0, 0.0), radius: 4.0, start: PI, sweep: PI },
        Seg2::Cubic { p0: p(0.0, 0.0), p1: p(2.0, -1.0), p2: p(6.0, -1.0), p3: p(8.0, 0.0) },
        Seg2::Conic { a: p(0.0, 0.0), apex: p(4.0, -2.0), b: p(8.0, 0.0), w: 0.5 },
        Seg2::Bezier { n: 4, p: controls },
    ] {
        let mut region = Region2 { outer: rectangle(0.0, 0.0, 8.0, 6.0), holes: vec![] };
        region.outer.segs[0] = curve;
        let before = region.clone();
        assert!(PreparedRegion::new(&region, &mut Budget::default()).is_err());
        assert_eq!(region, before);
    }
    let region = Region2 { outer: plate().outer, holes: vec![Loop2::circle(p(20.0, 10.0), 2.0)] };
    assert!(PreparedRegion::new(&region, &mut Budget::default()).is_err());
}

#[test]
fn open_degenerate_self_crossing_and_self_touching_boundaries_are_refused() {
    let mut open = rectangle(0.0, 0.0, 8.0, 6.0);
    open.segs.pop();
    let mut disconnected = rectangle(0.0, 0.0, 8.0, 6.0);
    disconnected.segs[1] = Seg2::Line { a: p(8.0, 1.0), b: p(8.0, 6.0) };
    for outer in [
        Loop2 { segs: vec![] },
        polygon(&[(0.0, 0.0), (8.0, 0.0)]),
        polygon(&[(0.0, 0.0), (4.0, 0.0), (8.0, 0.0)]),
        polygon(&[(0.0, 0.0), (4.0, 0.0), (4.0, 0.0), (0.0, 4.0)]),
        polygon(&[(0.0, 0.0), (8.0, 6.0), (0.0, 6.0), (8.0, 0.0)]),
        polygon(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (2.0, 2.0), (0.0, 4.0), (2.0, 2.0)]),
        polygon(&[(0.0, 0.0), (4.0, 0.0), (2.0, 0.0), (4.0, 4.0), (0.0, 4.0)]),
        open,
        disconnected,
    ] {
        let region = Region2 { outer, holes: vec![] };
        let before = region.clone();
        assert!(PreparedRegion::new(&region, &mut Budget::default()).is_err());
        assert_eq!(region, before);
    }
}

#[test]
fn outside_touching_crossing_nested_and_overlapping_holes_are_refused() {
    let outer = rectangle(0.0, 0.0, 10.0, 8.0);
    for holes in [
        vec![rectangle(11.0, 1.0, 12.0, 2.0)],
        vec![rectangle(0.0, 2.0, 2.0, 4.0)],
        vec![rectangle(-1.0, 2.0, 2.0, 4.0)],
        vec![polygon(&[(0.0, 0.0), (2.0, 1.0), (1.0, 2.0)])],
        vec![outer.clone()],
        vec![rectangle(-1.0, -1.0, 11.0, 9.0)],
        vec![rectangle(1.0, 1.0, 9.0, 7.0), rectangle(2.0, 2.0, 3.0, 3.0)],
        vec![rectangle(1.0, 1.0, 5.0, 5.0), rectangle(4.0, 2.0, 7.0, 6.0)],
        vec![rectangle(1.0, 1.0, 3.0, 3.0), rectangle(3.0, 1.0, 5.0, 3.0)],
        vec![rectangle(1.0, 1.0, 3.0, 3.0), rectangle(3.0, 3.0, 5.0, 5.0)],
    ] {
        let region = Region2 { outer: outer.clone(), holes };
        let before = region.clone();
        assert!(PreparedRegion::new(&region, &mut Budget::default()).is_err());
        assert_eq!(region, before);
    }
}

#[test]
fn hostile_coordinates_and_frames_fail_before_any_clipped_result() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX, -f64::MAX] {
        for corner in [(value, 0.0), (0.0, value)] {
            let region = Region2 { outer: polygon(&[corner, (8.0, 0.0), (8.0, 6.0), (0.0, 6.0)]), holes: vec![] };
            assert!(PreparedRegion::new(&region, &mut Budget::default()).is_err());
            assert!(Frame::new(p(corner.0, corner.1), Vec2::X).is_err());
        }
    }
    for tangent in [Vec2::ZERO, p(f64::NAN, 1.0), p(1.0, f64::INFINITY), p(f64::NEG_INFINITY, 1.0)] {
        assert!(Frame::new(Vec2::ZERO, tangent).is_err());
    }
    let frame = Frame::new(Vec2::ZERO, Vec2::X).unwrap();
    let region = plate();
    let mut budget = Budget::default();
    let prepared = PreparedRegion::new(&region, &mut budget).unwrap();
    let line = prepared.clip(&frame, 10.0, &mut budget).unwrap();
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(frame.point(value, 1.0).is_err());
        assert!(frame.point(1.0, value).is_err());
        assert!(prepared.clip(&frame, value, &mut budget).is_err());
        assert!(line.contains_strict(value, &mut budget).is_err());
    }
}

#[test]
fn exhausted_input_work_and_event_budgets_refuse_instead_of_returning_a_prefix() {
    let region = plate();
    let mut input_budget = Budget::new(Limits { input_pieces: 3, ..Limits::default() });
    assert!(PreparedRegion::new(&region, &mut input_budget).is_err());
    let mut no_work = Budget::new(Limits { work: 0, ..Limits::default() });
    assert!(PreparedRegion::new(&region, &mut no_work).is_err());

    // Spent counters cannot become reusable scratch capacity on a later call.
    let mut work = Budget::new(Limits { work: 1, ..Limits::default() });
    work.work(1).unwrap();
    assert!(horizontal_with_budget(&region, 10.0, &mut work).is_err());
    let mut events = Budget::new(Limits { events: 1, ..Limits::default() });
    events.events(1).unwrap();
    assert!(horizontal_with_budget(&region, 10.0, &mut events).is_err());
    // A fresh request still returns the complete span; refusal never mutates input.
    let (line, _budget) = horizontal(&region, 10.0);
    assert_spans(&line, &[(0.0, 40.0)]);
    assert_eq!(region, plate());
}

fn phase_parameter(coefficient: i128, exponent: i32, budget: &mut Budget) -> Param {
    Dyadic::from_parts(coefficient, exponent, budget).unwrap().param(budget).unwrap()
}

fn interval_contact_region() -> Region2 {
    Region2 { outer: rectangle(0.0, -1.0, 12.0, 1.0), holes: vec![rectangle(4.0, -0.5, 6.0, 0.5)] }
}

#[test]
fn tight_dyadic_dot_enclosures_have_one_owner_only_when_wholly_inside() {
    let (line, mut budget) = horizontal(&interval_contact_region(), 0.0);
    assert_spans(&line, &[(0.0, 4.0), (6.0, 12.0)]);
    // Each value is halfway between adjacent floats; no arbitrary interval
    // constructor is needed. Enclosures and tie-to-even results are hand-derived.
    for (coefficient, exponent, lo, hi, nearest, owner) in [
        ((1_i128 << 53) + 1, -53, 1.0, 1.0_f64.next_up(), 1.0, Some(0)),
        ((7_i128 << 51) + 1, -51, 7.0, 7.0_f64.next_up(), 7.0, Some(1)),
        ((5_i128 << 51) + 1, -51, 5.0, 5.0_f64.next_up(), 5.0, None),
        (-((1_i128 << 53) + 1), -53, (-1.0_f64).next_down(), -1.0, -1.0, None),
    ] {
        let t = phase_parameter(coefficient, exponent, &mut budget);
        assert_eq!((t.lo(), t.hi(), t.nearest()), (lo, hi, nearest));
        for i in 0..line.spans().len() {
            let expected = if owner == Some(i) { Some(nearest) } else { None };
            assert_eq!(line.dot_in_span(i, t, &mut budget).unwrap(), expected);
        }
    }
}

#[test]
fn singleton_dyadic_dots_on_outer_and_hole_contacts_are_excluded() {
    let (line, mut budget) = horizontal(&interval_contact_region(), 0.0);
    for coordinate in [0, 4, 6, 12] {
        let t = phase_parameter(coordinate, 0, &mut budget);
        assert_eq!(t.lo(), t.hi());
        for i in 0..line.spans().len() {
            assert_eq!(line.dot_in_span(i, t, &mut budget).unwrap(), None);
        }
    }
}

#[test]
fn tight_dot_enclosures_touching_material_and_a_contact_refuse_membership() {
    let region = Region2 { outer: rectangle(1.0, -1.0, 12.0, 1.0), holes: vec![rectangle(4.0, -0.5, 6.0, 0.5)] };
    let (line, mut budget) = horizontal(&region, 0.0);
    assert_spans(&line, &[(1.0, 4.0), (6.0, 12.0)]);
    // Exact points lie just inside material, but their tight enclosures reach
    // a contact. Their nearest values equal that contact: neither silently
    // dropping the dot nor classifying only the rounded value is permitted.
    for (coefficient, exponent, i, lo, hi, nearest) in [
        ((1_i128 << 53) + 1, -53, 0, 1.0, 1.0_f64.next_up(), 1.0),
        ((4_i128 << 52) - 1, -52, 0, 4.0_f64.next_down(), 4.0, 4.0),
        ((6_i128 << 51) + 1, -51, 1, 6.0, 6.0_f64.next_up(), 6.0),
        ((12_i128 << 50) - 1, -50, 1, 12.0_f64.next_down(), 12.0, 12.0),
    ] {
        let t = phase_parameter(coefficient, exponent, &mut budget);
        assert_eq!((t.lo(), t.hi(), t.nearest()), (lo, hi, nearest));
        assert!(line.dot_in_span(i, t, &mut budget).is_err());
        assert_eq!(line.dot_in_span(1 - i, t, &mut budget).unwrap(), None);
    }
}

#[test]
fn fractional_contacts_keep_their_enclosure_for_dot_dash_and_candidate_queries() {
    let region = Region2 { outer: polygon(&[(0.0, 0.0), (2.0, 0.0), (2.0, 3.0), (1.0, 3.0)]), holes: vec![] };
    let (line, mut budget) = horizontal(&region, 1.0);
    assert_eq!(line.spans().len(), 1);
    // The left edge is y=3*x, so its contact at y=1 is exactly 1/3.
    let below_third = f64::from_bits(0x3fd5_5555_5555_5555);
    let window = line.span_window(0, &mut budget).unwrap();
    assert!(window.0 <= below_third && window.1 >= 2.0);
    // This midpoint rounds up to the even neighbour. Its tight enclosure
    // straddles 1/3, regardless of the rounded geometric endpoint.
    let contact = phase_parameter(12_009_599_006_321_323, -55, &mut budget);
    assert_eq!(contact.lo(), below_third);
    assert_eq!(contact.hi(), below_third.next_up());
    assert_eq!(contact.nearest(), below_third.next_up());
    assert!(line.dot_in_span(0, contact, &mut budget).is_err());
    let inside = phase_parameter(1, 0, &mut budget);
    assert_eq!(line.dot_in_span(0, inside, &mut budget).unwrap(), Some(1.0));
    assert!(line.dash_in_span(0, contact, inside, &mut budget).is_err());

    // A covering dash can dominate both contacts without treating their
    // rounded output coordinates as exact membership certificates.
    let a = phase_parameter(-1, 0, &mut budget);
    let b = phase_parameter(3, 0, &mut budget);
    let clipped = line.dash_in_span(0, a, b, &mut budget).unwrap().unwrap();
    assert_eq!(clipped, line.spans()[0]);
}

#[test]
fn positive_dashes_touch_boundaries_without_zero_length_marks_or_hole_bridges() {
    let (line, mut budget) = horizontal(&interval_contact_region(), 0.0);
    for (start, end, expected) in
        [(-2, 0, vec![]), (4, 6, vec![]), (12, 14, vec![]), (2, 4, vec![(2.0, 4.0)]), (6, 8, vec![(6.0, 8.0)]), (3, 7, vec![(3.0, 4.0), (6.0, 7.0)])]
    {
        let a = phase_parameter(start, 0, &mut budget);
        let b = phase_parameter(end, 0, &mut budget);
        let mut actual = Vec::new();
        for i in 0..line.spans().len() {
            if let Some(span) = line.dash_in_span(i, a, b, &mut budget).unwrap() {
                assert!(span.start < span.end);
                actual.push((span.start, span.end));
            }
        }
        assert_eq!(actual, expected);
    }
}

#[test]
fn a_positive_exact_dash_with_overlapping_endpoint_enclosures_is_refused() {
    let (line, mut budget) = horizontal(&interval_contact_region(), 0.0);
    let a = phase_parameter(1, 0, &mut budget);
    let b = phase_parameter((1_i128 << 53) + 1, -53, &mut budget);
    // Exact length is 2^-53, but a.hi == b.lo. The adapter's stated
    // conservative admission requires separated endpoint enclosures.
    assert_eq!(a.hi(), b.lo());
    assert!(line.dash_in_span(0, a, b, &mut budget).is_err());
}

#[test]
fn interval_adapter_preserves_reflex_contact_splits_and_unique_dot_ownership() {
    let region = Region2 { outer: polygon(&[(0.0, 0.0), (2.0, 0.0), (4.0, 4.0), (6.0, 0.0), (8.0, 0.0), (8.0, 6.0), (0.0, 6.0)]), holes: vec![] };
    let (line, mut budget) = horizontal(&region, 4.0);
    assert_spans(&line, &[(0.0, 4.0), (4.0, 8.0)]);
    let a = phase_parameter(3, 0, &mut budget);
    let b = phase_parameter(5, 0, &mut budget);
    let contact = phase_parameter(4, 0, &mut budget);
    for (i, expected) in [(0, (3.0, 4.0)), (1, (4.0, 5.0))] {
        let span = line.dash_in_span(i, a, b, &mut budget).unwrap().unwrap();
        assert_eq!((span.start, span.end), expected);
        assert_eq!(line.dot_in_span(i, contact, &mut budget).unwrap(), None);
        assert_eq!(line.dot_in_span(i, a, &mut budget).unwrap(), if i == 0 { Some(3.0) } else { None });
        assert_eq!(line.dot_in_span(i, b, &mut budget).unwrap(), if i == 1 { Some(5.0) } else { None });
    }
}

#[test]
fn public_dash_crossing_a_hole_keeps_the_member_phase_after_the_gap() {
    let region = interval_contact_region();
    let pattern = Pattern {
        name: "HOLE_PHASE".into(),
        description: "Original global-phase split across a hole".into(),
        families: vec![Family { angle: 0.0, origin: p(-1.0, -4.0), delta_x: 2.0, delta_y: 4.0, dashes: vec![8.0, -4.0] }],
    };
    let before = (region.clone(), pattern.clone());
    // Only k=1 meets the region: its stroke is x=1..9, cut by the hole 4..6.
    assert_eq!(
        hatch::generate(&pattern, &region).unwrap(),
        vec![Mark::Line { a: p(1.0, 0.0), b: p(4.0, 0.0) }, Mark::Line { a: p(6.0, 0.0), b: p(9.0, 0.0) }]
    );
    assert_eq!((region, pattern), before);
}

#[test]
fn distinct_zero_tokens_survive_hole_traversal_and_every_repeated_visit_is_charged() {
    let region = Region2 { outer: rectangle(-1.0, -1.0, 13.0, 1.0), holes: vec![rectangle(3.0, -0.5, 5.0, 0.5)] };
    let pattern = Pattern {
        name: "ZERO_IDENTITIES".into(),
        description: "Distinct zeros across cycles and separated material spans".into(),
        families: vec![Family { angle: 0.0, origin: Vec2::ZERO, delta_x: 0.0, delta_y: 10.0, dashes: vec![0.0, 0.0, -4.0, 0.0] }],
    };
    let before = (region.clone(), pattern.clone());
    // At each x, two leading tokens and the preceding cycle's trailing token
    // survive. x=4 lies in the hole. A repeated span visit adds no extra dot.
    let expected: Vec<_> = [0.0, 0.0, 0.0, 8.0, 8.0, 8.0, 12.0, 12.0, 12.0].into_iter().map(|x| Mark::Dot { at: p(x, 0.0) }).collect();
    // Windows (-1,3) and (5,13), period 4: j=-2..1 and j=0..4.
    // Nine cycle visits times four tokens = 36; repeated candidates still cost.
    let admitted = Limits { dash_steps: 36, ..Limits::default() };
    assert_eq!(generate_with_limits(&pattern, &region, admitted).unwrap(), expected);
    let exhausted = Limits { dash_steps: 35, ..Limits::default() };
    assert_eq!(generate_with_limits(&pattern, &region, exhausted), Err(Invalid("dash step budget exhausted")));
    assert_eq!((region, pattern), before);
}

#[test]
fn interval_adapter_queries_charge_entry_before_invalid_or_empty_span_access() {
    let (line, mut budget) = horizontal(&interval_contact_region(), 0.0);
    let (empty, _empty_budget) = horizontal(&interval_contact_region(), 2.0);
    let a = phase_parameter(1, 0, &mut budget);
    let b = phase_parameter(2, 0, &mut budget);
    for clipped in [&line, &empty] {
        for i in [0, usize::MAX] {
            let mut exhausted = Budget::new(Limits { work: 0, ..Limits::default() });
            let expected = Invalid("geometry work budget exhausted");
            assert_eq!(clipped.span_window(i, &mut exhausted), Err(expected.clone()));
            assert_eq!(clipped.dot_in_span(i, a, &mut exhausted), Err(expected.clone()));
            assert_eq!(clipped.dash_in_span(i, a, b, &mut exhausted), Err(expected));
        }
    }
}
