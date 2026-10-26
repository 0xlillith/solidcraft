# Hatch patterns and PAT import (W38d D1)

D1 supplies a neutral hatch-pattern model and a bounded PAT reader for
[W38d (#7062)][issue], following [the drawing-sketch handoff][handoff].
It is a library foundation: it does not generate or clip hatch strokes, attach a
fill to a sketch, save drawing records, change the paper scene or register a
command. It does not complete W38d or implement section-derived hatching (W34).

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
the data's meaning; D1 does not enumerate family members.

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
are valid definitions that paint nothing. A later generator must keep phase
anchored at `p(k, 0)` across clipped fragments and holes.

The eventual paper adapter must keep, for example, a 3 mm hatch spacing at 3 mm
on paper when a view scale changes. It must transform the region to paper
coordinates without applying that view scale to the pattern a second time.
D1 has no view or placement API. A multi-family pattern also has no single
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
too dense for a later bounded generator. Validation does not authorize unlimited
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

## Later phases and acceptance

D2 still needs bounded region clipping and its own independent geometry tests.
A PAT dot must become an explicit point mark, such as a future
`Mark::Dot { at }`; this is **not a D1 type**. It must not become a zero-length
paper stroke: the current scene uses butt caps and rejects degenerate geometry.
The future paper adapter must choose a deliberate visible representation.

The generator also needs separate caps for candidate lines, intersection work,
dash steps and output marks, with whole-result refusal rather than truncated
success. The paper scene's aggregate record limit remains a separate final
check. General curved-region clipping is not supplied by D1.

Connecting fills to sketches waits for W38a's drawing-owned sketch record.
The later W38d integration owns persistent boundary identity, retained geometry and lost
reference reporting, commands and undo, embedded imported definitions, paper
object kinds/layers and export acceptance. Section-derived hatching remains a
separate feature. This slice adds no drawing-format field or new dependency.

D1 acceptance covers model validation and serde, converted units and phase data,
Unicode/BOM/line-ending handling, strict final blanks, malformed numeric/header
fields, duplicate ordering, and limit refusal. See the model and
[PAT acceptance tests][tests] for the cases present in the candidate.
Passing D1 tests does not establish clipping, paper output or interactive
acceptance; actual commands and results belong to the PR's validation record.

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
[tests]: ../crates/io/src/pat_tests.rs
[pat-reference]: https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Customization/files/GUID-A6F2E6FF-1717-44B6-A476-0CA817ADD77E.htm
[dash-reference]: https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-Customization/files/GUID-B67FADB9-0F10-4536-ABC4-BB7D8BDDF5B1.htm
