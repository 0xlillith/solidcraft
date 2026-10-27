# Drawing-owned sheet sketches: records, solving and command lifecycle

These increments of [W38a #7082][issue] build on the embedded-drawing K3b/K4-prime
receiver in [#3891][storage], not the superseded separate-file session. R1 adds
records, R2 read-only solving and atomic edits, R3 composes the shared store,
and R4 connects a bounded command lifecycle. This is not the complete native or
browser Drawing UI, all-tools parity, or completed W38a.

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

R3/R4 compose the actual W38.0 #6336 store with the embedded K session and route
a first set of sheet commands through R2, including entry, finish/cancel and
history. Drawing-owned dimension definitions, the remaining tools, cached live
presentation and native/browser UI parity still need implementation and acceptance.
The record-only container checks above do not qualify all those editing paths. W38d's later
saved-fill integration must retain outer-plus-holes identity after edits. W38b/c/e
and source-edge projection remain with their existing owners.

[issue]: https://github.com/tangency-labs/solidcraft/issues/7082
[storage]: https://github.com/tangency-labs/solidcraft/pull/3891

## Shared command-store receiver (R3)

The existing W38.0 store from CamWight's #6336, revision `07984653`, is now
composed with this embedded K receiver. This is a source-compatible port of the
same `cmd::sketch_store`, not another store or a replacement main-target PR.
R3 retained the original module, including its routing scan and null/exact-number
tests; R4 extends that same module with the explicitly owned sheet adapter. The calls present on this receiver go through `target`, `active`,
`read`, `write`, `update` and `checked` as in the original work.

The receiver predates several unrelated main changes. This adaptation does not
import pattern-expression refresh, live-link caching, Cancel Sketch, newer
projection de-duplication, coincident-circle tools, motion traces, or later
MidPoint/standard-part behavior just to make a cherry-pick apply. In particular,
`run_links` retains this receiver's original projection behavior inside the
shared update transaction. Model-only joint/plane lookups stay explicitly
marked. Original author/source references remain in the commit and PR.

At the R3 checkpoint this was **model-store routing**, not the sheet receiver.
R4 below changes the part gate only for its admitted tools; no drawing id is put into
`Session::active_sketch`. Two receiver-specific engine tests exercise model/sheet
id collisions, refusal of model sketch commands with a drawing open, successful
model editing after closing it, and callback failure without publishing a clone.
Their executed results, if available, belong in the PR.

The R4 command slice below now supplies explicit sheet ownership and R2 routing.
Drawing-owned dimensions, remaining tools, live presentation and UI acceptance
remain incomplete. Integrating this port is not evidence that #6336 landed on
main or that its independent full gate qualifies the combined receiver.

## R4: exact-owner command lifecycle

`drawing.sketch.create {owner, name?}` creates and enters a sketch on the exact
encoded sheet id of the open drawing. `drawing.sketch.edit {sketch}` enters an
existing exact encoded id; repeating it on the same sketch preserves the first
cancel boundary. `drawing.sketch.inspect {sketch?}` reports the resolved paper-mm
points, local entity ids and solver state without adopting that solution.

The existing `sketch.rectangle.two_point`, `sketch.line` and `sketch.point` tools
use the same geometry implementations through the shared store's sheet adapter.
There is no parallel sketcher or alternate model-feature lookup. A separate
session entry names drawing, sheet and sketch explicitly; the model's
`active_sketch` is never assigned a sheet id. An explicit tool target can confirm
that entry, not silently switch to another drawing or a same-numbered feature.
The ordinary model paths of these tools remain available with no drawing open.

Point and line expressions read the containing design's parameter values, under
R2's declared-source checks. Bare expression values are paper millimetres even
when the model displays inches. The legacy tool callback may read a copy of the
source document, but any attempted mutation of that source is refused before
publication. R2 validates and solves the changed sketch before storing it.

`sketch.finish` ends sheet editing. `sketch.cancel` restores only that sketch's
entry record, or removes a sketch created in that entry, then leaves editing.
The cancellation is itself one drawing undo step when data changed; Undo restores
the edited record and its active entry, and Redo returns to the cancelled state.
Other sketches, drawing properties, other drawings and model history are not
rolled back. Session checkpoints, scratch previews and restored drawing history
carry the explicit sheet entry. Switching or deleting drawings while it is active
requires Finish or Cancel. These commands must not evaluate a stale model merely
because a model-only operation was attempted from the drawing.

This first command slice deliberately does **not** enable dimensions that define
parameters, the other sketch geometry/modify/constraint tools, text, patterns,
offsets, clipboard, live drag or model projection paths in Drawing. They remain
explicitly unavailable, not silently redirected to model geometry. New source
bindings, auxiliary payload validation, all-tools parity, cached solved-state
presentation and actual native/browser editor UI acceptance remain subsequent
work. W38d saved-fill association is not enabled by these commands.

The lifecycle acceptance cases are in the existing engine `cmd/drawing_tests.rs`:
paper rectangle size, source-read-only point/line expressions, failed calls,
finish/cancel and history, checkpoint/scratch/reopen, colliding identifiers across
two drawings and a model sketch, schemas/hostile input, and no stale-model
re-evaluation. Executed results and any remaining failures belong in the PR.

New command messages are supplied as a pending locale delta, per current project
rules. This pinned K receiver predates the pending-catalog reader; native UI
translation qualification requires the already-merged shared prerequisite #6448
(`1ceab2ed4c03832f1937e0da98b9c8c50f49aa3e`) to reach `drawings`. The four pending
ids were checked against the current main English catalog and are not duplicates.
This feature does not modify the i18n loader, shared catalogs or CI to bypass that
integration requirement.
