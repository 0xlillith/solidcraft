# Hatch patterns, PAT import and polygonal generation (W38d D1–D3)

D1 supplies a neutral hatch-pattern model and a bounded PAT reader for
[W38d (#7062)][issue], following [the drawing-sketch handoff][handoff].
D2 adds bounded line-and-dot generation inside polygonal regions with holes.
D3 extends its phase arithmetic and clipping queries while retaining the D1
model, public generation API, polygon-only scope and existing resource caps.
These are library foundations: they do not attach a fill to a sketch, save
drawing records, change the paper scene or register a command. They do not
complete W38d or implement section-derived hatching (W34).

## Public API

The L0 [`solidcraft_geom::hatch` module][model] contains:

- `Pattern { name, description, families }`.
- `Family { angle, origin, delta_x, delta_y, dashes }`.
- `Pattern::validate()` and `Family::validate()`, returning
  `Result<(), hatch::Invalid>`.

`Pattern::lines_45()` supplies original continuous 45-degree lines;
`Pattern::cross_hatch()` supplies 45- and 135-degree families. Each family has
1 mm nominal spacing, zero origin and tangential delta, and no dashes. Solid
paint is a later separate alternative, not a pattern with no families.

The records support serde and reject unknown fields. Deserialization is not
validation: call the validator before using a deserialized or edited value.
These checks also do not bound allocations during decoding. An encoded-input
adapter must enforce its own byte and collection limits.

L3 [`solidcraft_io::read_pat`][reader] accepts an in-memory byte slice:

```rust
read_pat(bytes: &[u8], options: PatImportOptions)
    -> solidcraft_io::Result<Vec<solidcraft_geom::hatch::Pattern>>
```

`PatImportOptions` has `units: PatUnits` and `scale: f64`.
`PatUnits` is `Millimetres` or `Inches`; the default is millimetres and scale
`1.0`. The reader returns validated definitions in input order. Duplicate names
remain separate entries with their spelling preserved; this function does not
register, rename, merge or overwrite patterns.

The reader does not open files or modify a document. It returns either the
complete list or an error, with no partial list.

D2 adds [`hatch::generate`][generator], using the existing `geom::Region2`:

```rust
generate(pattern: &Pattern, region: &Region2) -> Result<Vec<Mark>, hatch::Invalid>
```

`Mark` is either `Line { a: Vec2, b: Vec2 }` or `Dot { at: Vec2 }`.
These are transient geometric marks in the supplied region's paper XY frame,
not paper-scene objects or saved fill records. A dot remains a point; it is not
a zero-length line. Each explicit zero in a dash sequence produces its own
eligible dot, even if another zero coincides with it. The call validates the
pattern and supported region, leaves both inputs unchanged, and returns the complete result or an error. No partial
output is returned after a failure.

## Coordinates, units and dash phase

All lengths in the neutral model are **paper millimetres at pattern scale 1**;
angles are radians, counterclockwise. `origin` is in the pattern's XY frame.
For a family with angle `a`, define its unit tangent `u = (cos(a), sin(a))`
and normal `n = (-sin(a), cos(a))`. Its member with signed integer index `k`
has the mathematical parameterization:

```text
p(k, t) = origin + k * (delta_x * u + delta_y * n) + t * u
```

`delta_x` shifts the phase along the line; `delta_y` is signed perpendicular
spacing. The origin is not pre-rotated by the family angle. This formula defines
the data's meaning. D2 enumerates a bounded set of members that can meet the region.

PAT angles arrive in degrees and become radians. The length multiplier is
`scale` for millimetres or `25.4 * scale` for inches. It applies once to both
origin coordinates, both deltas and every dash length, and never to the angle.
The scale and multiplier must be finite and positive. Overflow, or conversion
of a nonzero input into zero, is an error; values are not clamped. Model bounds
are checked **after** conversion.

A family whose source spacing is `2`, imported with inches and scale `0.5`,
therefore has `25.4 mm` perpendicular spacing. A future import command can
show these converted values before saving anything. PAT supplies no units;
the reader does not guess a unit system from the filename or pattern name.

An empty dash list means a continuous line. In a nonempty list, positive lengths
draw, negative lengths leave gaps, and zero denotes a dot. The cycle length is
the sum of absolute lengths. All-zero cycles are invalid; negative-only cycles
are valid definitions that paint nothing. D2 keeps phase anchored at `p(k, 0)`
across clipped fragments and holes; clipping does not restart the dash sequence.

The eventual paper adapter must keep, for example, a 3 mm hatch spacing at 3 mm
on paper when a view scale changes. It must transform the region to paper
coordinates without applying that view scale to the pattern a second time.
D1–D3 have no view or placement API. A multi-family pattern also has no single
intrinsic spacing: a future spacing control must define its relationship to
pattern scale explicitly.

## Model limits

These are SolidCraft admission limits, not physical precision guarantees or
claims about every PAT implementation.

| Value | Accepted domain |
| --- | --- |
| Pattern name | 1–31 ASCII characters; first a letter or digit, then letters, digits, `_`, `-` or `$` |
| Description | At most 80 Unicode scalar values; no control characters |
| Families per pattern | 1–64 |
| Dash entries per family | 0–6 |
| Origin components, deltas and dash lengths | Finite, absolute value at most `100000 mm` |
| Angle | Finite, absolute value at most `1000000 rad` |
| Perpendicular delta | Nonzero, including rejection of either signed zero |
| Nonempty dash cycle | Finite, strictly positive sum of absolute lengths |

There is no minimum positive spacing epsilon. A definition can pass D1 yet be
too dense for the bounded generator. Validation does not authorize unlimited
line, intersection, dash or point expansion.

## PAT text policy and errors

The reader follows the public [pattern-definition syntax][pat-reference] and
[dash/offset semantics][dash-reference]. Its encoding policy is UTF-8 with an
optional single initial UTF-8 BOM; it does not guess legacy code pages.
Additional BOMs and physical control characters other than tabs are rejected,
including in comments.
Names and numeric tokens use ASCII syntax. Descriptions and comments may contain
Unicode. Limits are:

| Boundary | Limit |
| --- | --- |
| Input, checked before decoding | 1 MiB (1,048,576 bytes), including any BOM |
| Patterns in one import | 1,000 |
| Physical line | 80 Unicode scalar values, excluding LF or CRLF and the initial BOM, before removing comments |

LF and CRLF endings are accepted; a bare CR is refused. A header starts with
`*`, followed by the name and an optional comma-separated description.
Only the first header comma separates the name; subsequent commas belong to the
description. A descriptor contains five required comma-separated numbers:
angle, origin X, origin Y, tangential delta and perpendicular delta, followed
by zero to six dash entries. Decimal exponent notation is accepted. Empty
numeric fields, a trailing comma and nonfinite values are errors. A nonzero
decimal that underflows to zero while parsing is also refused.

Blank lines and text after `;` do not contribute to syntax. Horizontal tabs
are permitted as lexical whitespace, but control characters are not valid in a
name or description. Empty/comment-only input, a descriptor without a header,
and a pattern with no family are refused. Collection limits are checked before
growing their vectors.

**The final blank line is required.** After the last descriptor, there must be
a physical line containing only spaces/tabs or nothing. A single terminating
newline is insufficient, and a comment-only line does not substitute for the
blank line. A final spaces/tabs-only line may end at EOF without LF. Comments
after that blank line are allowed; a subsequent header or descriptor requires
another final blank line. For example, these are original
synthetic test inputs, shown as escaped strings:

```text
"*SC_SAMPLE, Original example\n30,0,0,0,4\n\n"  accepted
"*SC_SAMPLE, Original example\n30,0,0,0,4\n"    missing final blank line
```

Errors use `solidcraft_io::IoError::Invalid`. Text errors start with
`PAT line N:`, using one-based physical line numbers, including the relevant
header or descriptor line. Missing the blank line in a two-line definition
reports expected line 3. UTF-8 errors identify the physical line where possible.
Option and total byte-limit errors start with `PAT import:`, without an
invented line number.

## Polygonal generation: D2 geometry retained by D3

The [clipper][clipper] accepts one `Region2`: an outer loop and zero or more
holes, all made of `Seg2::Line`. Every loop must have at least three edges,
close exactly, use finite nondegenerate segments and be simple. All input and
generated coordinate components must lie within `+/-100000 mm`. Either loop
orientation is accepted; the outer/hole roles come from `Region2`, not winding.
Holes must lie strictly inside the outer loop, without touching, crossing or
nesting within one another. Unsupported or
uncertain geometry returns `hatch::Invalid`; the input is not snapped, repaired
or approximated. Arcs, conics and Bézier segments remain unsupported in D3.

Clipping preserves positive-length interior spans and their boundary contacts.
A span's endpoints may lie on the boundary; its interior must lie in material.
An isolated contact splits spans, including a tangent contact that does not
change crossing parity. A line running along a boundary does not paint that
boundary: coincident boundary intervals are subtracted, rather than rejected
merely for coincidence. Dots are emitted only in the strict interior, never at
a boundary contact. The clipper does not bridge a hole or merge spans across a contact.

Numerical predicates use arithmetic enclosures for the represented inputs.
Unresolved signs, overlapping contact bounds or a dot whose membership cannot
be distinguished from a contact return an error. Endpoints and frame
reconstruction are rounded floating-point values, not an exact coordinate or
universal geometric-tolerance certificate. In particular, some mathematically
valid polygons or near-contact patterns can be refused without a fallback.

The generator uses a normalized tangent and its left normal. In that frame,
member `k` has normal offset `k * delta_y` and tangential dash phase
`k * delta_x`. The phase is applied once, including for negative member indices,
negative perpendicular spacing and negative dash-cycle positions. Output follows
family order, ascending signed member index, then ascending cycles within each
clipped span. This is not a global coordinate sort.

## D3 exact phase and clipping parameters

D2 required exact binary64 results for the normal offset `k * delta_y`,
tangential phase `k * delta_x`, and each cumulative sum of absolute dash
lengths. That policy refused, for example, the cycle sum for `[0.1, -0.2]`
even though the stored pattern passes D1 validation.

D3 retains the exact-binary64 check for every candidate normal offset
`k * delta_y`, including conservative exterior candidates. It replaces the
phase and cycle-sum restrictions with bounded exact arithmetic in the private
[`phase` module][phase]. No new public type, dependency or saved field is added.

### What is exact

The arithmetic starts from the **binary64 values already stored in the
pattern**. A private dyadic value represents `N * 2^e`, with a checked `i128`
coefficient and an integer exponent. This does not recover exact decimal text
from PAT or reverse earlier unit conversion.

The member phase `H = k * delta_x`, dash prefixes and period `P` are built
exactly from those represented inputs. Each candidate parameter is formed as
`H + j * P + prefix` before conversion to floating point. A prefix at `P`
uses the exact successor cycle endpoint. Global signed cycle indices and the
original zero-token identities remain intact across holes and contact splits.

Only the checked dyadic converter constructs the private `Param` value. Its
`lo()` and `hi()` are the finite floating-point neighbours enclosing the exact
parameter; `nearest()` is one round-to-nearest, ties-to-even conversion of the
complete expression. An exactly representable parameter has equal bounds.
The implementation does not first round a cycle base and then add a prefix.

This admits some decimal-valued dash sequences that D2 refused: for instance,
the represented inputs `[0.1, -0.2]` can form an exact dyadic period. Success
still depends on the region, exact normal spacing, arithmetic envelope, clipping
classification and cumulative budgets. It is not a promise to generate every
finite pattern or every decimal spacing.

### Clipping before rounding for output

For each span, candidate cycle bounds use the retained lower bound of its left
contact and upper bound of its right contact. Exact signed floor division,
followed by checked padding, selects candidate cycles. The preceding cycle
remains eligible for a trailing zero. Rounded span midpoints never determine
the candidate window or dot membership.

The private clipping adapter consumes complete parameter enclosures:

- A dot is emitted only when its entire enclosure is strictly inside one span.
  An enclosure proven exterior or exactly on a boundary yields no dot;
  unresolved contact overlap returns an error. Revisiting the same token for
  another candidate span must not duplicate it. Different explicit zero tokens
  at the same coordinate remain separate marks.
- A positive dash requires its start enclosure to lie strictly before its end
  enclosure. Its intersection with a span is selected by comparisons with the
  retained contact bounds. A proven empty intersection yields no stroke;
  uncertain overlap or rounded collapse returns an error. This conservative
  adapter may refuse an exact positive dash whose endpoint enclosures overlap
  or touch. It does not weaken the pattern validator.
- A selected exact dash endpoint uses `nearest()` once. A selected geometric
  contact keeps the existing rounded D2 span endpoint. The resulting scalar
  endpoints are terminal output for the existing frame reconstruction; they
  are never reused as containment certificates.

Thus exact phase arithmetic does **not** make the polygon intersections,
normalized frame or final `Vec2` coordinates exact. The existing finite,
distinct-endpoint and directional-progress checks still apply. The polygon,
hole and boundary-contact rules described above are unchanged.

### Bounded arithmetic envelope

The representation has explicit internal limits:

| Value | Accepted domain |
| --- | --- |
| Raw and normalized coefficient | `-i128::MAX..=i128::MAX`; `i128::MIN` is refused |
| Input and normalized exponent | `-1074..=1023` |
| Nonzero canonical coefficient | Odd after bounded normalization |
| Canonical zero | `(0, 0)`, without merging zero tokens |
| Converted parameter | Finite lower, upper and nearest values |

Alignment shifts, addition, subtraction, integer multiplication, signed floor
division and normalization are checked. An intermediate that does not fit is an
error, even if a wider calculation could cancel or normalize it later.
The arithmetic never saturates, silently discards low bits or guesses a cycle
range. For example, dividing the least positive subnormal dyadic by one may
refuse because the required alignment does not fit `i128`, despite its
mathematical floor being zero. Values below the least positive subnormal are
outside this representation; they are not silently converted to zero.

The generator's separate `+/-2^52` admission limit still applies to enumerated
member and cycle indices, including padded candidates. Exact arithmetic does
not raise that limit or bypass existing progress checks: cycle endpoints and
nonzero token endpoints must still advance after nearest rounding, including
gap-only visits.

## Work and output limits (D2–D3)

One internal [budget][budget] spans preparation, every family, clipping, dash
enumeration, dot membership queries and output. Its default cumulative caps are:

| Counter | Maximum per call |
| --- | --- |
| Input boundary pieces | 8,192 |
| Candidate family lines | 10,000 |
| Geometric work units | 2,000,000 |
| Created contact/intersection events | 65,536 |
| Dash steps | 200,000 |
| Output marks, lines and dots together | 40,000 |

Candidate lines include conservative neighbours of the projected index range,
including lines outside the region. Dash steps include every visited dash/gap/dot
entry in the conservatively padded cycle range, even when it produces no mark.
Repeated work across clipped spans also counts. The event cap counts all events
created during the call; freeing or reusing temporary storage does not restore
that budget. These counters are work limits,
not elapsed-time or accuracy guarantees. The internal limit injection used by
tests can lower these ceilings but cannot raise them, and is not a public API.
Production uses the same generator with the defaults. D3 charges its bounded
dyadic operations and each clipping classification to the existing work
counter, including failing queries and queries that produce no marks. It adds
no counter or budget increase; the extra checks can exhaust the work cap sooner.

Budget or allocation failure, nonfinite arithmetic and unresolved numeric
uncertainty fail the whole call. Member and cycle indices must fit the generator's safe integer range
(`abs(index) <= 2^52`, including padded neighbours); nonzero spacing, dash and gap
steps must still advance numerically. A far-away origin and very fine pattern
can therefore be refused even when few visible marks would remain.
Passing the D1 model validator does not guarantee a region can be generated
within these bounds. There is no implicit coarse fallback or
truncated successful result. Caller-side input-byte/allocation limits and any
later paper-scene record limit remain separate responsibilities.

## Later phases and acceptance

The future paper adapter must choose a visible representation for `Mark::Dot`;
the current scene's butt-cap strokes reject degenerate geometry. D1–D3 supply no
paper adapter, stroke width, colour, export, command or interactive preview.
Curved-region clipping remains outside this slice.

Connecting fills to sketches waits for W38a's drawing-owned sketch record.
The later W38d integration owns persistent boundary identity, retained geometry
and lost-reference reporting, commands and undo, embedded imported definitions,
paper object kinds/layers and export acceptance. Section-derived hatching
remains a separate feature. D1–D3 add no drawing-format field or new dependency.

D1 acceptance covers model validation and serde, converted units and phase data,
Unicode/BOM/line-ending handling, strict final blanks, malformed numeric/header
fields, duplicate ordering, and limit refusal. See the model and
[PAT acceptance tests][tests] for the cases present in the candidate.

D2 acceptance is split between [clipping tests][clip-tests] and
[generation tests][generate-tests]: original analytic polygons and holes,
contacts and boundary runs, global dash phase, strict interior dots, malformed
or unsupported geometry, numeric refusal and cumulative caps. D3 extends these
checks with the independent [phase tests][phase-tests]: signed floor division,
exact represented-input expressions, nearest-even ties, subnormal values and
arithmetic refusal, plus enclosure-aware dash/dot integration cases.

The actual test commands and results belong to the PR's validation record,
bound to the tested source revision. D1/D2 test results do not validate a later
D3 source change, and source review or written tests do not establish execution.
Library checks do not establish paper output, persistence or interactive
acceptance.

## Sources and licensing

The implementation and synthetic test strings are original SolidCraft work
under MIT OR Apache-2.0. Public format documentation informs the grammar; no
commercial pattern files, templates, sample libraries, help text or artwork are
copied or shipped. A PAT imported by a user remains user-supplied data; parsing
it does not grant redistribution rights. Any future bundled external pattern
would need its own source, licence and attribution review.

Simple diagonal and crossed-line constructions are mathematical patterns. This
foundation makes no ISO drafting-conformance claim and includes no standard's
protected tables or illustrations. See the repository's
[attribution policy](../ATTRIBUTION.md) and [NOTICE](../NOTICE).

[issue]: https://github.com/tangency-labs/solidcraft/issues/7062
[handoff]: https://github.com/tangency-labs/solidcraft/issues/5425#issuecomment-6414961262
[model]: ../crates/geom/src/hatch.rs
[reader]: ../crates/io/src/pat.rs
[generator]: ../crates/geom/src/hatch/generate.rs
[clipper]: ../crates/geom/src/hatch/clip.rs
[budget]: ../crates/geom/src/hatch/budget.rs
[phase]: ../crates/geom/src/hatch/phase.rs
[phase-tests]: ../crates/geom/src/hatch/phase_tests.rs
[clip-tests]: ../crates/geom/src/hatch/clip_tests.rs
[generate-tests]: ../crates/geom/src/hatch/generate_tests.rs
[tests]: ../crates/io/src/pat_tests.rs
[pat-reference]: https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Customization/files/GUID-A6F2E6FF-1717-44B6-A476-0CA817ADD77E.htm
[dash-reference]: https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Customization/files/GUID-B67FADB9-0F10-4536-ABC4-BB7D8BDDF5B1.htm
