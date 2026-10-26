//! Original analytic hatch-output cases; no external pattern library or CAD output.
use std::f64::consts::FRAC_PI_2;

use super::budget::Limits;
use super::generate::generate_with_limits;
use super::{Family, Mark, Pattern, generate};
use crate::{Loop2, Region2, Seg2, Vec2};

// Comparison only, never an admission/snap tolerance. The rotated and decimal
// fixtures stay within 66 mm: this is under 128 binary64 epsilons at that scale.
// Axis-aligned dyadic fixtures below instead require exact coordinates.
const ROUNDOFF_MM: f64 = 1e-12;

fn family(origin: [f64; 2], dx: f64, dy: f64, dashes: &[f64]) -> Family {
    Family { angle: 0.0, origin: Vec2::new(origin[0], origin[1]), delta_x: dx, delta_y: dy, dashes: dashes.to_vec() }
}

fn pattern(families: Vec<Family>) -> Pattern {
    Pattern { name: "ORIGINAL_PHASE".into(), description: "Independent analytic output cases".into(), families }
}

fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Loop2 {
    Loop2::polygon(&[Vec2::new(x0, y0), Vec2::new(x1, y0), Vec2::new(x1, y1), Vec2::new(x0, y1)])
}

fn region(x0: f64, y0: f64, x1: f64, y1: f64) -> Region2 {
    Region2 { outer: rectangle(x0, y0, x1, y1), holes: vec![] }
}

fn near_point(actual: Vec2, expected: [f64; 2], tolerance: f64) -> bool {
    actual.is_finite() && (actual.x - expected[0]).abs() <= tolerance && (actual.y - expected[1]).abs() <= tolerance
}

fn assert_marks(marks: &[Mark], lines: &[[f64; 4]], dots: &[[f64; 2]], tolerance: f64) {
    // Match the complete multiset, allowing traversal and endpoint direction to
    // differ. Consuming each match separately preserves coincident dot instances.
    assert_eq!(marks.len(), lines.len() + dots.len(), "unexpected marks: {marks:?}");
    let mut unmatched = marks.to_vec();
    for &[ax, ay, bx, by] in lines {
        let found = unmatched.iter().position(|mark| match *mark {
            Mark::Line { a, b } => {
                assert!(a.is_finite() && b.is_finite() && a != b, "invalid stroke: {mark:?}");
                (near_point(a, [ax, ay], tolerance) && near_point(b, [bx, by], tolerance))
                    || (near_point(a, [bx, by], tolerance) && near_point(b, [ax, ay], tolerance))
            }
            Mark::Dot { .. } => false,
        });
        let Some(index) = found else { panic!("missing line {ax},{ay} -> {bx},{by}; remaining: {unmatched:?}") };
        unmatched.swap_remove(index);
    }
    for &at in dots {
        let found = unmatched.iter().position(|mark| match *mark {
            Mark::Dot { at: actual } => near_point(actual, at, tolerance),
            Mark::Line { .. } => false,
        });
        let Some(index) = found else { panic!("missing dot {at:?}; remaining: {unmatched:?}") };
        unmatched.swap_remove(index);
    }
    assert!(unmatched.is_empty(), "unexpected marks: {unmatched:?}");
}

#[test]
fn clipped_first_stroke_keeps_the_origin_and_negative_cycles() {
    // Strokes are [2 + 5*j, 5 + 5*j]. A reset at -5.5 also gives total
    // length 9, so only the individual endpoints distinguish the wrong phase.
    let p = pattern(vec![family([2.0, 0.0], 0.0, 10.0, &[3.0, -2.0])]);
    let r = region(-5.5, -0.5, 9.5, 0.5);
    let marks = generate(&p, &r).unwrap();
    assert_marks(&marks, &[[-5.5, 0.0, -5.0, 0.0], [-3.0, 0.0, 0.0, 0.0], [2.0, 0.0, 5.0, 0.0], [7.0, 0.0, 9.5, 0.0]], &[], 0.0);
}

#[test]
fn tangential_delta_changes_phase_once_for_positive_and_negative_members() {
    // k=-1,0,1 have y=-2,0,2 and x origins -1.5,0,1.5.
    let p = pattern(vec![family([0.0, 0.0], 1.5, 2.0, &[2.0, -3.0])]);
    let r = region(0.25, -2.5, 8.25, 2.5);
    let marks = generate(&p, &r).unwrap();
    assert_marks(
        &marks,
        &[[0.25, -2.0, 0.5, -2.0], [3.5, -2.0, 5.5, -2.0], [0.25, 0.0, 2.0, 0.0], [5.0, 0.0, 7.0, 0.0], [1.5, 2.0, 3.5, 2.0], [6.5, 2.0, 8.25, 2.0]],
        &[],
        0.0,
    );
}

#[test]
fn rotated_negative_deltas_keep_global_origin_and_signed_member_phase() {
    // u=(0,1), n=(-1,0), hence B_k=(4+3*k,1-k), k=-1,0,1,2.
    let mut f = family([4.0, 1.0], -1.0, -3.0, &[1.0, -2.0]);
    f.angle = FRAC_PI_2;
    let marks = generate(&pattern(vec![f]), &region(0.5, -0.5, 10.5, 5.5)).unwrap();
    assert_marks(
        &marks,
        &[
            [1.0, -0.5, 1.0, 0.0],
            [1.0, 2.0, 1.0, 3.0],
            [1.0, 5.0, 1.0, 5.5],
            [4.0, 1.0, 4.0, 2.0],
            [4.0, 4.0, 4.0, 5.0],
            [7.0, 0.0, 7.0, 1.0],
            [7.0, 3.0, 7.0, 4.0],
            [10.0, -0.5, 10.0, 0.0],
            [10.0, 2.0, 10.0, 3.0],
            [10.0, 5.0, 10.0, 5.5],
        ],
        &[],
        ROUNDOFF_MM,
    );
}

#[test]
fn a_hole_splits_a_dash_without_pausing_or_restarting_its_cycle() {
    let p = pattern(vec![family([0.0, 0.0], 0.0, 10.0, &[3.0, -2.0])]);
    let mut r = region(-0.5, -0.75, 14.5, 0.75);
    r.holes.push(rectangle(2.5, -0.25, 8.5, 0.25).reversed());
    let marks = generate(&p, &r).unwrap();
    assert_marks(&marks, &[[0.0, 0.0, 2.5, 0.0], [10.0, 0.0, 13.0, 0.0]], &[], 0.0);
}

#[test]
fn explicit_dots_keep_negative_cycles_and_phase_across_a_hole() {
    // Every cycle has stroke [4*j,4*j+1] and dot 4*j+2.
    let p = pattern(vec![family([0.0, 0.0], 0.0, 10.0, &[1.0, -1.0, 0.0, -2.0])]);
    let mut r = region(-2.5, -0.75, 10.5, 0.75);
    r.holes.push(rectangle(1.5, -0.25, 5.5, 0.25).reversed());
    let marks = generate(&p, &r).unwrap();
    assert_marks(&marks, &[[0.0, 0.0, 1.0, 0.0], [8.0, 0.0, 9.0, 0.0]], &[[-2.0, 0.0], [6.0, 0.0], [10.0, 0.0]], 0.0);
}

#[test]
fn converted_paper_millimetres_and_common_translation_are_applied_once() {
    // Values are already converted by 12.7 mm per authored length. There is
    // no PAT reader or units argument in this generator-level output oracle.
    let p = pattern(vec![family([3.175, 1.5875], 6.35, 12.7, &[3.175, -3.175, 0.0, -6.35])]);
    let marks = generate(&p, &region(0.0, 0.0, 25.4, 3.175)).unwrap();
    assert_marks(&marks, &[[3.175, 1.5875, 6.35, 1.5875], [15.875, 1.5875, 19.05, 1.5875]], &[[9.525, 1.5875], [22.225, 1.5875]], ROUNDOFF_MM);

    let translated = pattern(vec![family([43.175, -5.4125], 6.35, 12.7, &[3.175, -3.175, 0.0, -6.35])]);
    let marks = generate(&translated, &region(40.0, -7.0, 65.4, -3.825)).unwrap();
    assert_marks(
        &marks,
        &[[43.175, -5.4125, 46.35, -5.4125], [55.875, -5.4125, 59.05, -5.4125]],
        &[[49.525, -5.4125], [62.225, -5.4125]],
        ROUNDOFF_MM,
    );
}

#[test]
fn reversing_signed_member_parameterization_preserves_the_same_geometry() {
    let r = region(0.25, -2.5, 8.25, 2.5);
    let p = pattern(vec![family([0.0, 0.0], -1.5, -2.0, &[2.0, -3.0])]);
    let marks = generate(&p, &r).unwrap();
    assert_marks(
        &marks,
        &[[0.25, -2.0, 0.5, -2.0], [3.5, -2.0, 5.5, -2.0], [0.25, 0.0, 2.0, 0.0], [5.0, 0.0, 7.0, 0.0], [1.5, 2.0, 3.5, 2.0], [6.5, 2.0, 8.25, 2.0]],
        &[],
        0.0,
    );
}

#[test]
fn changing_loop_start_and_orientation_does_not_restart_the_pattern() {
    let p = pattern(vec![family([0.0, 0.0], 0.0, 10.0, &[3.0, -2.0])]);
    let outer = rectangle(-0.5, -0.75, 14.5, 0.75);
    let hole = rectangle(2.5, -0.25, 8.5, 0.25);
    for reverse_outer in [false, true] {
        for reverse_hole in [false, true] {
            for start in 0..4 {
                let mut r = Region2 {
                    outer: if reverse_outer { outer.reversed() } else { outer.clone() },
                    holes: vec![if reverse_hole { hole.reversed() } else { hole.clone() }],
                };
                r.outer.segs.rotate_left(start);
                r.holes[0].segs.rotate_left((start + 1) % 4);
                let marks = generate(&p, &r).unwrap();
                assert_marks(&marks, &[[0.0, 0.0, 2.5, 0.0], [10.0, 0.0, 13.0, 0.0]], &[], 0.0);
            }
        }
    }
}

#[test]
fn continuous_lines_ignore_tangential_phase_and_keep_holes() {
    let p = pattern(vec![family([7.0, 0.0], 3.5, 2.0, &[])]);
    let mut r = region(-1.5, -2.5, 4.5, 2.5);
    r.holes.push(rectangle(0.5, -0.25, 1.5, 0.25).reversed());
    let marks = generate(&p, &r).unwrap();
    assert_marks(&marks, &[[-1.5, -2.0, 4.5, -2.0], [-1.5, 0.0, 0.5, 0.0], [1.5, 0.0, 4.5, 0.0], [-1.5, 2.0, 4.5, 2.0]], &[], 0.0);
}

#[test]
fn repeated_zero_tokens_are_distinct_even_at_neighboring_cycle_boundaries() {
    let r = region(-0.25, -0.5, 2.25, 0.5);
    let expected = [[0.0, 0.0], [0.0, 0.0], [1.0, 0.0], [1.0, 0.0], [2.0, 0.0], [2.0, 0.0]];
    for dashes in [&[0.0, 0.0, -1.0][..], &[0.0, -1.0, 0.0][..]] {
        let p = pattern(vec![family([0.0, 0.0], 0.0, 10.0, dashes)]);
        let marks = generate_with_limits(&p, &r, Limits { marks: 6, ..Limits::default() }).unwrap();
        assert_marks(&marks, &[], &expected, 0.0);
        assert!(generate_with_limits(&p, &r, Limits { marks: 5, ..Limits::default() }).is_err());
    }
}

#[test]
fn dots_on_outer_and_hole_boundaries_are_omitted_but_interior_dots_remain() {
    let p = pattern(vec![family([0.0, 0.0], 0.0, 10.0, &[0.0, -1.0])]);
    let mut r = region(0.0, -0.75, 6.0, 0.75);
    r.holes.push(rectangle(2.0, -0.25, 4.0, 0.25).reversed());
    let marks = generate(&p, &r).unwrap();
    assert_marks(&marks, &[], &[[1.0, 0.0], [5.0, 0.0]], 0.0);
}

#[test]
fn a_stroke_retains_boundary_endpoints_without_becoming_a_dot() {
    let p = pattern(vec![family([0.0, 0.0], 0.0, 10.0, &[1.0, -1.0])]);
    let marks = generate(&p, &region(0.0, -0.5, 1.0, 0.5)).unwrap();
    assert_marks(&marks, &[[0.0, 0.0, 1.0, 0.0]], &[], 0.0);
    // At x=1 and x=2 the adjacent positive strokes have only point contact.
    let marks = generate(&p, &region(1.0, -0.5, 2.0, 0.5)).unwrap();
    assert_marks(&marks, &[], &[], 0.0);
}

#[test]
fn independent_families_keep_their_own_origins_and_phase() {
    let p = pattern(vec![family([0.0, -0.25], 0.0, 10.0, &[1.0, -1.0]), family([0.5, 0.25], 0.0, 10.0, &[0.0, -2.0])]);
    let marks = generate(&p, &region(-0.25, -0.5, 3.25, 0.5)).unwrap();
    assert_marks(&marks, &[[0.0, -0.25, 1.0, -0.25], [2.0, -0.25, 3.0, -0.25]], &[[0.5, 0.25], [2.5, 0.25]], 0.0);
}

#[test]
fn all_negative_cycles_paint_nothing_instead_of_continuous_lines() {
    let p = pattern(vec![family([0.0, 0.0], 1.5, -2.0, &[-1.0, -2.0])]);
    let marks = generate(&p, &region(-4.5, -2.5, 4.5, 2.5)).unwrap();
    assert_marks(&marks, &[], &[], 0.0);
}

#[test]
fn painting_nothing_does_not_bypass_pattern_or_region_validation() {
    let gap = family([0.0, 0.0], 0.0, 10.0, &[-1.0]);
    let p = pattern(vec![gap.clone()]);
    let curved = Region2 { outer: Loop2::circle(Vec2::ZERO, 1.0), holes: vec![] };
    assert!(generate(&p, &curved).is_err());
    assert!(generate(&p, &Region2::default()).is_err());
    let mut open = region(-1.0, -0.5, 1.0, 0.5);
    open.outer.segs.truncate(3);
    assert!(generate(&p, &open).is_err());

    let r = region(-1.0, -0.5, 1.0, 0.5);
    let invalid = Family { delta_y: 0.0, ..gap.clone() };
    assert!(generate(&pattern(vec![gap, invalid]), &r).is_err());
    let mut invalid_name = p;
    invalid_name.name.clear();
    assert!(generate(&invalid_name, &r).is_err());
}

#[test]
fn a_later_invalid_family_refuses_the_entire_result() {
    let good = family([0.0, 0.0], 0.0, 10.0, &[]);
    let r = region(-1.0, -0.5, 1.0, 0.5);
    let mut bad = good.clone();
    bad.angle = f64::NAN;
    assert!(generate(&pattern(vec![good, bad]), &r).is_err());
}

#[test]
fn supported_line_input_does_not_silently_accept_an_unsupported_hole() {
    let p = pattern(vec![family([0.0, 0.0], 0.0, 10.0, &[])]);
    let mut r = region(-2.0, -2.0, 2.0, 2.0);
    r.holes.push(Loop2::circle(Vec2::ZERO, 0.5).reversed());
    assert!(generate(&p, &r).is_err());
}

#[test]
fn boundary_runs_are_omitted_without_shifting_the_interior_members() {
    let p = pattern(vec![family([0.0, 0.0], 0.0, 1.0, &[])]);
    let marks = generate(&p, &region(-1.0, 0.0, 1.0, 2.0)).unwrap();
    assert_marks(&marks, &[[-1.0, 1.0, 1.0, 1.0]], &[], 0.0);
}

#[test]
fn each_work_budget_can_refuse_a_small_otherwise_valid_pattern() {
    let p = pattern(vec![family([0.0, 0.0], 0.0, 10.0, &[1.0, -1.0])]);
    let r = region(-0.25, -0.5, 1.25, 0.5);
    assert_marks(&generate(&p, &r).unwrap(), &[[0.0, 0.0, 1.0, 0.0]], &[], 0.0);
    for limits in [
        Limits { input_pieces: 0, ..Limits::default() },
        Limits { candidate_lines: 0, ..Limits::default() },
        Limits { work: 0, ..Limits::default() },
        Limits { events: 0, ..Limits::default() },
        Limits { dash_steps: 0, ..Limits::default() },
        Limits { marks: 0, ..Limits::default() },
    ] {
        assert!(generate_with_limits(&p, &r, limits).is_err());
    }
}

#[test]
fn marks_are_counted_across_families_and_split_strokes_without_partial_success() {
    let first = family([0.0, -0.25], 0.0, 10.0, &[]);
    let second = family([0.0, 0.25], 0.0, 10.0, &[]);
    let mut r = region(-1.5, -0.5, 2.5, 0.5);
    r.holes.push(rectangle(0.0, -0.375, 1.0, 0.375).reversed());
    let p = pattern(vec![first, second]);
    let marks = generate_with_limits(&p, &r, Limits { marks: 4, ..Limits::default() }).unwrap();
    assert_marks(&marks, &[[-1.5, -0.25, 0.0, -0.25], [1.0, -0.25, 2.5, -0.25], [-1.5, 0.25, 0.0, 0.25], [1.0, 0.25, 2.5, 0.25]], &[], 0.0);
    assert!(generate_with_limits(&p, &r, Limits { marks: 3, ..Limits::default() }).is_err());
}

#[test]
fn candidate_lines_are_counted_even_without_marks() {
    let r = region(-0.25, -0.5, 0.25, 0.5);
    let f = family([0.0, 0.0], 0.0, 10.0, &[-1.0]);
    let p = pattern(vec![f.clone(), f]);
    assert!(generate(&p, &r).unwrap().is_empty());
    // Each family has member k=0 in the region; a one-line budget cannot
    // serve both. Conservative exterior candidates also consume this budget.
    assert!(generate_with_limits(&p, &r, Limits { candidate_lines: 1, ..Limits::default() }).is_err());
}

#[test]
fn candidate_lines_and_dash_steps_are_charged_for_a_visible_gap() {
    let p = pattern(vec![family([0.0, 0.0], 0.0, 10.0, &[1.0, -3.0])]);
    let r = region(1.25, -0.5, 3.75, 0.5);
    assert!(generate(&p, &r).unwrap().is_empty());
    assert!(generate_with_limits(&p, &r, Limits { candidate_lines: 0, ..Limits::default() }).is_err());
    assert!(generate_with_limits(&p, &r, Limits { dash_steps: 0, ..Limits::default() }).is_err());
}

#[test]
fn subnormal_spacing_is_valid_data_but_cannot_expand_without_a_bound() {
    let p = pattern(vec![family([0.0, 0.0], 0.0, f64::from_bits(1), &[])]);
    assert!(p.validate().is_ok());
    assert!(generate(&p, &region(-0.5, -0.5, 0.5, 0.5)).is_err());
}

#[test]
fn a_nonzero_dash_step_that_cannot_advance_is_refused() {
    let p = pattern(vec![family([0.0, 0.0], 0.0, 10.0, &[1.0, -f64::from_bits(1)])]);
    assert!(p.validate().is_ok());
    assert!(generate(&p, &region(-0.25, -0.5, 2.25, 0.5)).is_err());
}

#[test]
fn unresolvable_far_phase_indices_are_not_saturated_into_a_success() {
    let p = pattern(vec![family([100_000.0, 0.0], 0.0, 10.0, &[2.0_f64.powi(-40), -2.0_f64.powi(-40)])]);
    let r = region(-2.0_f64.powi(-50), -0.5, 2.0_f64.powi(-50), 0.5);
    assert!(p.validate().is_ok());
    assert!(generate(&p, &r).is_err());
}

#[test]
fn collapsed_or_nonfinite_line_boundaries_return_an_error() {
    let p = pattern(vec![family([0.0, 0.0], 0.0, 10.0, &[])]);
    for point in [Vec2::ZERO, Vec2::new(f64::NAN, 0.0), Vec2::new(f64::INFINITY, 0.0)] {
        let r = Region2 { outer: Loop2 { segs: vec![Seg2::Line { a: point, b: point }] }, holes: vec![] };
        assert!(generate(&p, &r).is_err());
    }
}

#[test]
fn a_large_member_phase_cancels_before_its_single_output_rounding() {
    // Independent integer oracle: k*dx = 99_999_000_000_099_999.
    // With period 100 and cycle -999_990_000_001_000, the visible stroke
    // is exactly [-1,0]. Rounding k*dx first instead shifts it to [0,1].
    let k: i64 = 1_000_000_000_001;
    let dx: i64 = 99_999;
    let cycle: i128 = -999_990_000_001_000;
    let exact_start = i128::from(k) * i128::from(dx) + cycle * 100;
    assert_eq!(exact_start, -1);
    assert_eq!(exact_start + 1, 0);

    // These dyadic bounds contain only member k and preserve their width
    // exactly. Stored origin, deltas and dashes all satisfy model admission.
    let dy = 2.0_f64.powi(-40);
    let p = pattern(vec![family([0.0, 0.0], dx as f64, dy, &[1.0, -99.0])]);
    let r = region(-2.0, (k as f64 - 0.25) * dy, 2.0, (k as f64 + 0.25) * dy);
    assert!(p.validate().is_ok());
    let y = k as f64 * dy;
    assert_marks(&generate(&p, &r).unwrap(), &[[-1.0, y, 0.0, y]], &[], 0.0);
}

#[test]
fn decimal_dash_cycles_round_complete_endpoints_once() {
    // The exact sum of the stored binary64 values 0.1 and 0.2 is
    // 10_808_639_105_689_191 / 2^55: it lies between representable f64 values.
    // With n=3_602_879_701_896_397, the exact endpoints for j=-2..2 are
    // [-6n,-5n], [-3n,-2n], [0,n], [3n,4n], [6n,7n], all divided by 2^55.
    // In particular -5n/2^55 rounds to -0.5; adding a rounded cycle base
    // to the first dash would instead round that endpoint twice.
    let p = pattern(vec![family([0.0, 0.0], 0.0, 10.0, &[0.1, -0.2])]);
    let r = region(-0.75, -0.5, 0.75, 0.5);
    assert!(p.validate().is_ok());
    assert_marks(
        &generate(&p, &r).unwrap(),
        &[
            [-0.6000000000000001, 0.0, -0.5, 0.0],
            [-0.30000000000000004, 0.0, -0.2, 0.0],
            [0.0, 0.0, 0.1, 0.0],
            [0.30000000000000004, 0.0, 0.4, 0.0],
            [0.6000000000000001, 0.0, 0.7000000000000001, 0.0],
        ],
        &[],
        0.0,
    );
}

#[test]
fn a_large_decimal_cycle_preserves_the_stored_period_residual() {
    // Let n=3_602_879_701_896_397, D=2^55 and K=J=2^50.
    // The stored dx is -(3n+1)/D, whereas the exact dash period is 3n/D.
    // K*dx + J*period = -1/32; adding the first dash gives
    // (n-2^50)/D = 2_476_979_795_053_773/D, exactly the stored 0.06875.
    // Repeating the rounded period would wrongly yield [0,0.1].
    let dy = 2.0_f64.powi(-50);
    let p = pattern(vec![family([0.0, 0.0], -0.30000000000000004, dy, &[0.1, -0.2])]);
    let half_height = 2.0_f64.powi(-52);
    let r = region(-0.0625, 1.0 - half_height, 0.1875, 1.0 + half_height);
    assert!(p.validate().is_ok());
    assert_marks(&generate(&p, &r).unwrap(), &[[-0.03125, 1.0, 0.06875, 1.0]], &[], 0.0);
}

#[test]
fn a_nonrepresentable_dot_phase_rounds_only_after_interior_classification() {
    // Stored dx=3_002_399_751_580_331/2^53; member k=3 gives the exact
    // phase 1+2^-53, enclosed by [1,1+2^-52] and rounded nearest-even to 1.
    // That enclosure is strictly inside (0,2), so its single dot is retained.
    let p = pattern(vec![family([0.0, 0.0], 0.33333333333333337, 1.0, &[0.0, -4.0])]);
    let r = region(0.0, 2.5, 2.0, 3.5);
    assert_marks(&generate(&p, &r).unwrap(), &[], &[[1.0, 3.0]], 0.0);
}

#[test]
fn a_dot_enclosure_overlapping_a_boundary_refuses_the_entire_result() {
    // The same exact phase lies above 1, but its f64 enclosure touches the
    // boundary at 1. Neither dropping its rounded point nor accepting its
    // midpoint certifies membership. A preceding family must not leak output.
    let first = family([0.0, 3.0], 0.0, 10.0, &[]);
    let ambiguous = family([0.0, 0.0], 0.33333333333333337, 1.0, &[0.0, -4.0]);
    let r = region(1.0, 2.5, 2.0, 3.5);
    assert_marks(&generate(&pattern(vec![first.clone()]), &r).unwrap(), &[[1.0, 3.0, 2.0, 3.0]], &[], 0.0);
    assert!(generate(&pattern(vec![ambiguous.clone()]), &r).is_err());
    assert!(generate(&pattern(vec![first, ambiguous]), &r).is_err());
}
