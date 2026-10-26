//! Bounded expansion of stored paper-millimetre patterns over prepared line regions.

use crate::{Region2, Vec2};

use super::budget::{Budget, Limits};
use super::clip::{ClippedLine, Frame, PreparedRegion, Span};
use super::{Invalid, Pattern};

type Result<T> = std::result::Result<T, Invalid>;

// Leave representable integer neighbours for conservative range padding and cycle ends.
const MAX_INDEX: f64 = 4_503_599_627_370_496.0;

/// One generated mark in paper millimetres. Dots remain points, not degenerate lines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mark {
    Line { a: Vec2, b: Vec2 },
    Dot { at: Vec2 },
}

/// Expand a validated pattern over a supported simple region with straight boundaries.
/// The result is complete or an error; inputs are never changed. Stored lengths already
/// use paper millimetres, so this function applies no unit conversion or view scale.
///
/// Output follows family order, ascending signed line index and ascending dash cycles
/// within each clipped span. A hole or boundary contact does not restart dash phase.
/// Strokes retain their clipped endpoints; dots require strict interior membership.
/// Every explicit zero retains its own dot, including coincident zeros across cycles.
///
/// Preparation, candidate lines, clipping, dash visits and marks share cumulative limits.
/// Unsupported boundaries, exhausted limits, nonfinite arithmetic, unsafe indices or
/// lost numeric progress return an error rather than truncated or repaired geometry.
/// Line/phase products and dash-prefix sums must be exactly representable; some valid
/// stored patterns therefore refuse expansion. Final coordinates remain rounded f64 values.
pub fn generate(pattern: &Pattern, region: &Region2) -> Result<Vec<Mark>> {
    generate_with_limits(pattern, region, Limits::default())
}

pub(super) fn generate_with_limits(pattern: &Pattern, region: &Region2, limits: Limits) -> Result<Vec<Mark>> {
    pattern.validate()?;
    let mut budget = Budget::new(limits);
    let region = PreparedRegion::new(region, &mut budget)?;
    let mut marks = Vec::new();
    for family in &pattern.families {
        budget.work(1)?;
        let frame = Frame::new(family.origin, Vec2::from_angle(family.angle))?;
        let (low, high) = region.normal_bounds(&frame, &mut budget)?;
        let a = finite(low / family.delta_y)?;
        let b = finite(high / family.delta_y)?;
        // Include neighbouring indices when projection/division rounds onto an integer.
        // These are candidates, not output promises; exterior candidates also cost budget.
        let lines = IndexRange::new(a.min(b).floor() - 1.0, a.max(b).ceil() + 1.0)?;
        budget.candidate_lines(lines.count()?)?;
        let cycle = Cycle::new(&family.dashes, &mut budget)?;
        let mut previous_offset = None;
        for k in lines.start..=lines.end {
            budget.work(1)?;
            let offset = certified_index_product(k, family.delta_y, "uncertified hatch spacing product")?;
            if let Some(previous) = previous_offset {
                let advances = if family.delta_y > 0.0 { offset > previous } else { offset < previous };
                if !advances {
                    return Err(Invalid("line spacing loses numeric progress"));
                }
            }
            previous_offset = Some(offset);
            let clipped = region.clip(&frame, offset, &mut budget)?;
            if clipped.spans().is_empty() {
                continue;
            }
            let phase = if cycle.entries.is_empty() { 0.0 } else { certified_index_product(k, family.delta_x, "uncertified hatch phase product")? };
            let member = Member { clipped: &clipped, frame: &frame, offset, phase };
            for span in clipped.spans() {
                budget.work(1)?;
                if !span.start.is_finite() || !span.end.is_finite() || span.start >= span.end {
                    return Err(Invalid("invalid clipped span"));
                }
                if cycle.entries.is_empty() {
                    push_line(&frame, offset, span.start, span.end, &mut budget, &mut marks)?;
                } else {
                    cycle.expand(&member, span, &mut budget, &mut marks)?;
                }
            }
        }
    }
    Ok(marks)
}

struct IndexRange {
    start: i64,
    end: i64,
}

impl IndexRange {
    fn new(start: f64, end: f64) -> Result<Self> {
        if !start.is_finite() || !end.is_finite() || start.abs() > MAX_INDEX || end.abs() > MAX_INDEX || start > end {
            return Err(Invalid("hatch line or cycle index is not safely representable"));
        }
        // Callers supply floor/ceil values, and the checked range lies inside exact i64/f64 integers.
        Ok(Self { start: start as i64, end: end as i64 })
    }

    fn count(&self) -> Result<usize> {
        let count = self.end.checked_sub(self.start).and_then(|n| n.checked_add(1)).ok_or(Invalid("hatch index count overflow"))?;
        usize::try_from(count).map_err(|_| Invalid("hatch index count exceeds the platform limit"))
    }
}

struct Entry {
    start: f64,
    end: f64,
    value: f64,
}

struct Cycle {
    entries: Vec<Entry>,
    period: f64,
}

struct Member<'a> {
    clipped: &'a ClippedLine,
    frame: &'a Frame,
    offset: f64,
    phase: f64,
}

impl Cycle {
    fn new(dashes: &[f64], budget: &mut Budget) -> Result<Self> {
        let mut entries = Vec::new();
        entries.try_reserve_exact(dashes.len()).map_err(|_| Invalid("cannot allocate hatch dash entries"))?;
        let mut period = 0.0;
        // Pattern::validate has already bounded this list to six entries.
        for &value in dashes {
            budget.work(1)?;
            let end = certified_cycle_sum(period, value.abs())?;
            if value != 0.0 && end <= period {
                return Err(Invalid("dash length loses numeric progress within its cycle"));
            }
            entries.push(Entry { start: period, end, value });
            period = end;
        }
        if !entries.is_empty() && period <= 0.0 {
            return Err(Invalid("dash cycle must have a positive period"));
        }
        Ok(Self { entries, period })
    }

    fn expand(&self, member: &Member<'_>, span: &Span, budget: &mut Budget, marks: &mut Vec<Mark>) -> Result<()> {
        let first = finite(finite(span.start - member.phase)? / self.period)?.floor() - 1.0;
        let last = finite(finite(span.end - member.phase)? / self.period)?.floor() + 1.0;
        let cycles = IndexRange::new(first, last)?;
        let visits = cycles.count()?.checked_mul(self.entries.len()).ok_or(Invalid("dash visit count overflow"))?;
        // Reserve all visits before iteration, including gaps and cycles outside the span.
        // The previous cycle also owns any trailing zero coincident with this cycle's start.
        budget.dash_steps(visits)?;
        for j in cycles.start..=cycles.end {
            budget.work(1)?;
            let base = finite((j as f64).mul_add(self.period, member.phase))?;
            let next_j = j.checked_add(1).ok_or(Invalid("dash cycle index overflow"))?;
            let next_base = finite((next_j as f64).mul_add(self.period, member.phase))?;
            if next_base <= base {
                return Err(Invalid("dash cycle loses numeric progress"));
            }
            for entry in &self.entries {
                budget.work(1)?;
                let start = self.position(entry.start, base, next_base)?;
                let end = self.position(entry.end, base, next_base)?;
                if entry.value == 0.0 {
                    // Query before the local-span filter: a rounded endpoint must not
                    // silently discard a dot inside an uncertain contact enclosure.
                    let inside = member.clipped.contains_strict(start, budget)?;
                    if inside && start > span.start && start < span.end {
                        let at = member.frame.point(member.offset, start)?;
                        if !at.is_finite() {
                            return Err(Invalid("dot position is not finite"));
                        }
                        budget.marks(1)?;
                        marks.try_reserve(1).map_err(|_| Invalid("cannot allocate hatch output marks"))?;
                        marks.push(Mark::Dot { at });
                    }
                } else {
                    if end <= start {
                        return Err(Invalid("dash or gap loses numeric progress on its line"));
                    }
                    if entry.value > 0.0 {
                        let a = start.max(span.start);
                        let b = end.min(span.end);
                        if a < b {
                            push_line(member.frame, member.offset, a, b, budget, marks)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn position(&self, prefix: f64, base: f64, next_base: f64) -> Result<f64> {
        // Canonical cycle endpoints keep final and initial zeros exactly coincident.
        let t = if prefix == self.period { next_base } else { finite(base + prefix)? };
        if t < base || t > next_base {
            return Err(Invalid("dash phase is not representable within its cycle"));
        }
        Ok(t)
    }
}

fn push_line(frame: &Frame, offset: f64, start: f64, end: f64, budget: &mut Budget, marks: &mut Vec<Mark>) -> Result<()> {
    let a = frame.point(offset, start)?;
    let b = frame.point(offset, end)?;
    let progress = (b - a).dot(frame.tangent());
    if !a.is_finite() || !b.is_finite() || a == b || !progress.is_finite() || progress <= 0.0 {
        return Err(Invalid("positive hatch span loses finite directional progress"));
    }
    budget.marks(1)?;
    marks.try_reserve(1).map_err(|_| Invalid("cannot allocate hatch output marks"))?;
    marks.push(Mark::Line { a, b });
    Ok(())
}

fn finite(value: f64) -> Result<f64> {
    if value.is_finite() { Ok(value) } else { Err(Invalid("nonfinite hatch arithmetic")) }
}

fn certified_index_product(index: i64, factor: f64, reason: &'static str) -> Result<f64> {
    let k = index as f64;
    let product = finite(k * factor)?;
    // k is an exactly represented integer from IndexRange. Its exact product
    // with a finite f64 is a multiple of the minimum subnormal, so a nonzero
    // residual cannot disappear below that value. This is not a certificate
    // for arbitrary f64 multiplication. A later FMA cannot recover a lost phase.
    if k.mul_add(factor, -product) != 0.0 {
        return Err(Invalid(reason));
    }
    Ok(product)
}

fn certified_cycle_sum(prefix: f64, length: f64) -> Result<f64> {
    let value = finite(prefix + length)?;
    // TwoSum on finite bounded positive inputs, with gradual underflow. Reject
    // loss before repeating the period: many cycles could amplify a tiny residue.
    let part = value - prefix;
    let residual = (prefix - (value - part)) + (length - part);
    if residual != 0.0 {
        return Err(Invalid("uncertified hatch dash cycle sum"));
    }
    Ok(value)
}
