# Drawing-owned sheet sketches: records and atomic edits

R1 of [W38a #7082][issue] stores sheet-sketch records; R2 resolves their source
references and applies candidate edits through the existing solver. Both build
on the embedded-drawing K3b/K4-prime receiver in [#3891][storage], not the
superseded separate-file drawing session. These library interfaces do not yet
enable sketch commands, an interactive editor or complete W38a.

## Stored meaning

`Drawing.sketches` contains `Arc<SheetSketch>` records. A record has `id`, `sheet`,
`name` and the existing `solidcraft_sketch::Sketch` payload. Its id belongs to the
same drawing-wide namespace as the drawing, source and sheet ids. Its `sheet`
reference must resolve inside that drawing. Local ids such as `origin`, `p1`
and `l1` remain local to the payload and may repeat in a different sketch.
`Drawing::sheet_sketch(id)` searches only that drawing; it does not inspect the
active model or search other drawings.

Positions and length dimensions are paper XY millimetres, angles are radians.
Changing `Drawing.units` is not a geometry conversion. For example the authored
rectangle (15,10) to (55,30) remains 40 by 20 paper mm when units are `in`.
There is no view scale or source path in this sheet-owned record.

Point expression strings and dimension parameter names are retained verbatim.
Record loading neither evaluates them nor creates a model parameter. A successful
read does not establish that a name resolves in the design, that expressions
have compatible units, or that constraints solve. The explicit R2 solve/edit
interface below performs those checks on a candidate before adopting it. No
model evaluation or geometry repair occurs during record loading.

## Current supported payload

The first slice admits the shared 2D point/curve/constraint payload: finite
bounded points, valid local identities and point/curve references, bounded
splines, radii and conic weights, and constraints checked by the existing
`Sketch::validate`. Periodic splines use the existing validator. The fixed
local origin must remain point zero. Input is not snapped or solved.

**Explicitly not connected yet:** nonempty links (including generated text),
patterns, offsets/freeform offsets and 3D wires, and the model-only slice/3D
palette state. They are refused, not dropped. Linked points/curves are also
refused. Enabling these auxiliary record families requires their bounded
validation and Drawing receiver paths; it is later W38a work. This is not an
alternative numeric-only editing mode: no editing mode or command is enabled.

## Identity, ordering and compatibility

An absent/empty `sketches` list is omitted on write, retaining the previous
record shape. One `drawing/format/pending/sheet-sketches.rs` entry versions the
optional addition; there is no new design-format step or manually edited
version number. Builds that do not know the pending change refuse the file.

Records are serialized by numeric drawing object id, independently of their
insertion order, without mutating the live vector. Reading adopts that order.
Sheet order and all local sketch geometry/ids remain unchanged. Duplicate or
zero drawing ids, duplicate local ids and unknown owners are errors. There is
no nearest-sheet fallback. New ids allocated by `Drawing::allocate_id` also
avoid existing sketch records.

## Bounded admission

The encoded drawing still uses the existing 16 MiB limit. `from_value`, used
by embedded plain-JSON designs, applies the same new raw record checks before
typed sketch decoding. The new admission limits are explicit implementation
bounds, not a claim about geometric precision:

- At most 1,000 sheet-sketch records per drawing.
- Each payload uses the shared `MAX_POINTS`, `MAX_CURVES`, `MAX_CONSTRAINTS`
  and `MAX_SPLINE_POINTS` caps. Their point/curve/constraint and spline-index
  entries together cannot exceed 80,000 across the drawing.
- Stored string values are at most 4,096 bytes; local ids/field names at most
  512 bytes, and sketch names follow the drawing's 256-byte name rule. Counter
  maps have at most 64 prefixes. Expressions cannot be blank or contain controls.
- Raw payload depth is at most 32 and all sketch records together at most
  1,280,000 JSON nodes and 16 MiB of string/key bytes. These do not replace the
  encoded-file cap or make a pre-existing JSON Value allocation bounded.
- Constraint checks have a cumulative 2,000,000 work allowance. Slot-length
  validation is charged conservatively for its nested constraint searches.

The typed edit boundary repeats array, identity, range, owner and constraint
checks. Nonfinite positions/dimensions are errors before a drawing with sketches
can be written by `Drawing::to_json`; NaN is not silently stored as JSON null.
The shared Sketch type and model-sketch reader are unchanged. No new solver,
cache, active-sketch store, external font, dependency or CI setting is added.

## R2: read-only source resolution and atomic editing

`Document::solve_sheet_sketch(drawing_id, sketch_id)` returns a
`SolvedSheetSketch`: explicit drawing, sheet and sketch identities, the candidate
Sketch payload and its full shared solver report. It never adopts that result.
Repeated calls read current source parameter values rather than a stale cache.

`Document::edit_sheet_sketch(drawing_id, sketch_id, change)` gives the caller only
`&mut Sketch` on a private copy. The callback cannot use that argument to mutate
the containing design or change the record's identity/owner. Existing record
validation runs before the solver reads the candidate and again after solving.
The implementation calls the existing `Document::solve_sketch`, with the original
sketch's driving values, instead of adding an alternative solver or evaluator.

The lookup uses exact ids in the selected drawing. A missing or duplicate drawing
id, a missing sketch, invalid owning sheet or malformed payload is an error;
there is no name lookup or fallback to a similarly numbered model sketch.
Named references require that drawing's declared Source. In the current embedded
receiver a Source means the containing design, and at most one is admitted.
`Sheet.documents` names the component for the title block; it is not a separate
parameter source or permission to inspect another document.

Stored point expressions and dimension parameter names use the existing source
parameter evaluator read-only. Source values keep their own units and become
internal millimetres/radians. Bare numbers in the sketch's point expressions
retain the expression engine's mm/degree defaults, not the design's display units.
For example a source `Inset = 1 in` plus a bare `5` places X at 30.4 paper mm.
Literal-only sketches need no source. Errors in referenced source parameters,
including cycles, remain explicit; an unrelated parameter error does not redirect
or unnecessarily block an independent sketch.

A failed callback, invalid candidate, missing name, wrong dimension, nonfinite or
out-of-range evaluation, or incompatible solve publishes nothing. On success only
the chosen drawing is replaced. A no-op keeps the drawing and sketch Arcs. Point
expressions and source parameter names remain authored data, not baked values;
this API does not introduce a drawing-local parameter definition syntax or allow
a dimension command to write a source parameter. Expression text work is charged
against the existing drawing byte ceiling; other solver limits remain the shared
solver's. This is not a per-frame latency or whole-process memory guarantee.

This boundary supplies resolved state and atomic commits for the later
receiver-aware store. Session undo, edit entry/finish/cancel, command schemas,
view updates and UI integration are not implemented by these Document methods.
There is no second active-sketch store or history, and no model geometry is
requested or evaluated here. The R1 unsupported auxiliary payloads remain refused.

## Acceptance and remaining work

The record tests live in the existing `crates/doc/src/drawing_tests.rs`. Original
fixtures cover a paper rectangle and stored expressions; deterministic ordering;
drawing/local identity separation; unknown owners; absent/empty sketches; size
limits before typed allocation; finite/index/constraint rejection; unsupported
auxiliary data; inline-design round trips without model changes; and bounded
JSON mutations. The existing `crates/io/src/design/drawing_tests.rs` also exercises
these records through real design-container and plain-JSON writers/readers:
round trips retain the paper-mm geometry, expressions and drawing-local ownership;
changing one sketch changes only its drawing entry, not the model/container metadata
or a second drawing; damaged sketch records refuse the whole input without falling
back to another drawing's sheet. These checks do not run an editor or evaluate a model.
The additional R2 tests in `drawing_tests.rs` check source resolution without
mutation, paper-mm defaults under inch display units, fresh source values, atomic
success/refusal, conflict and malformed-expression handling, exact ownership,
source cycles, unrelated errors and no-op Arc identity. They exercise the actual
shared evaluator and solver. Actual executed results belong in the PR, not this
design note.

The command/store increment still depends on W38.0 #6336 and the real K session
interfaces. It must route reads/edits to the resolved-state boundary above and
supply drawing-owned dimension bindings, create/edit/finish/cancel, drawing-local
undo/redo and checkpoints, hostile command handling, reopening after live edits
and UI/native/browser parity. The
record-only container checks above are not that end-to-end acceptance. W38d's later
saved-fill integration must retain outer-plus-holes identity after edits. W38b/c/e
and source-edge projection remain with their existing owners.

[issue]: https://github.com/tangency-labs/solidcraft/issues/7082
[storage]: https://github.com/tangency-labs/solidcraft/pull/3891
