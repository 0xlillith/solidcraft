# Drawing-owned sheet sketches: W38a record slice

This is the first, data-only increment of [W38a #7082][issue]. It builds on the
embedded-drawing K3b/K4-prime receiver in [#3891][storage], not the superseded
separate-file drawing session. It does not enable sketch commands or complete W38a.

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
This layer neither evaluates them nor creates a model parameter. A successful
read does not establish that a name resolves in the design, that expressions
have compatible units, or that constraints solve. Those checks belong to the
later receiver-aware read-only evaluation/edit transaction, before tools are
enabled. No model evaluation or geometry repair occurs during record loading.

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
The shared Sketch type and model-sketch reader are unchanged. No solver, cache,
active sketch, source state, external font, dependency or CI setting is added.

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
Actual executed results belong in the PR, not this design note.

The command/store increment still depends on W38.0 #6336 and the real K session
interfaces. It must supply read-only expression bindings and evaluation, solved
state, create/edit/finish/cancel, drawing-local undo/redo and checkpoints, hostile
command handling, reopening after live edits and UI/native/browser parity. The
record-only container checks above are not that end-to-end acceptance. W38d's later
saved-fill integration must retain outer-plus-holes identity after edits. W38b/c/e
and source-edge projection remain with their existing owners.

[issue]: https://github.com/tangency-labs/solidcraft/issues/7082
[storage]: https://github.com/tangency-labs/solidcraft/pull/3891
