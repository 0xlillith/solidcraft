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
The origin is the physical sheet's lower-left corner, x right and y up, exactly
as in the paper scene. It is not the drawing-border corner; no margin translation
or view scale is applied.
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
validation and Drawing receiver paths; it is later W38a work. Record admission
alone does not enable a tool. R4's explicit rectangle/line/point command subset
is described below; it supports source expressions, not a separate numeric-only
sketcher, and does not enable these auxiliary payloads.

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

## Read-only solving and staged edits (R2, revised after review)

`Document::solve_sheet_sketch(drawing_id, sketch_id)` still resolves the exact
stored sketch against the current containing design, without publishing its
solution. This public document read validates the complete drawing. Repeated
calls read the current source values, not a model-evaluation cache.

`Drawing::solve_sheet_sketch(design, sketch_id)` is the staging counterpart:
owner/header identities, cumulative sketch budgets and the selected payload are
checked, without deeply validating every unrelated payload. It is used only
inside a drawing editing boundary, or after the full document-read check.

`Drawing::edit_sheet_sketch(design, sketch_id, change)` stages an edit on this
Drawing. The callback receives a private `&mut Sketch`. The original payload is
checked before the callback, the candidate before solving and its solved result
afterwards. Cumulative entity and validation-work limits include the unchanged
sketches as well, before the solver runs. Errors leave the Drawing unchanged;
a no-op retains its selected record Arc. The operation reads source parameters
and uses `Document::solve_sketch`; it cannot write into the source Document.

The previous `Document::edit_sheet_sketch` publication API has been removed.
The engine captures the immutable source and calls the staged operation inside
`Session::edit_drawing`. That existing Session boundary validates the **whole**
result once and adopts only the open Drawing. Its command's history therefore
belongs to that same Drawing. The shared store never publishes a Document copy
itself for sheet geometry. A library caller adopting a staged Drawing likewise
must validate the complete record at its publication boundary.

Unchanged Arcs are not a validation certificate: the complete check still rejects
an invalid unrelated sketch, even if it was not edited and its Arc is unchanged.
No implicit trusted-record cache has been added. Full reading and saving retain
complete validation; `Drawing::to_json` now applies that rule regardless of
whether the drawing contains sketches. An invalid empty-sketch drawing is no
longer serialised as a successful file.

The current embedded G3 Source means the containing design. `Sheet.documents`
is only the title block's component selector. Source values keep their units;
bare point-expression values keep paper-mm/degree defaults. For example,
`Inset = 1 in` plus bare `5` produces 30.4 paper mm. Names, expressions and their
error states remain authored data, not baked values. Only named references need
a declared Source; unrelated source errors do not retarget the sketch.

This is not a drawing-local parameter-definition syntax, another solver, an
incremental validation certificate or a whole-process latency/memory guarantee.
The original unsupported auxiliary payloads remain refused. Resource limits and
error expectations are unchanged; timing evidence and executed checks are in the
PR rather than being asserted as performance thresholds in correctness tests.

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

## Sheet ownership and deletion

The shared sheet-deletion rule belongs to the sheet commands (#3875): refuse a
sheet that contains objects and list its views/sketches; only an explicit
"delete everything" choice removes that sheet and its contents as one drawing
undo step. W38a does not add a competing deletion command or silently detach a
sketch. A missing owning sheet remains invalid. The eventual sheet command must
also respect the active edit boundary (finish or cancel before switching or
removing its owner). This records the agreed integration rule, not acceptance
of a sheet-deletion UI that is absent from this receiver.

For future shared-scene integration, sheet sketches use the same lower-left
frame and the proposed paint band after views and before tables/title blocks.
The paper ObjectKind/scene implementation remains with the shared-scene owner;
this review change adds neither a second scene nor an exporter.


## R6: more shared geometry tools and sheet-safe snap input

The sheet command receiver additionally admits the existing centre/two-point/
three-point circles, centre-point/three-point arcs, and centre/three-point
rectangles. Only their command part is extended; their model constructors,
constraint rules and solver implementation are reused unchanged. The same R5
Session publication boundary validates the drawing and owns its undo step.
This is command coverage, not interactive GUI or all-tools acceptance.

Numeric positions and radii remain paper millimetres. Centre and arc point
arguments retain the existing exact local-point-id and source-expression forms;
numeric-only parameters keep the existing command contracts. This increment
does not add radius expressions, drawing dimension definitions or source-edge
projection. Ordinary modelling commands retain their existing behaviour.

A design may retain an active model sketch while a drawing is open. Shared point
preprocessing must not turn a model `mid:`, `on:`, `vertex:` or `on_edge` snap into
a newly allocated model-local point id and then interpret that id on the sheet.
The sheet route now has no model target and explicitly refuses these automatic
snap/projection forms before it reads or changes a model sketch. This includes
malformed marker objects that also contain an `at` fallback. Plain 2D points,
expressions and exact existing local point ids remain available. The model snap
path is unchanged. Dedicated sheet/view projection remains later work.

Original synthetic tests use a deliberate model/sheet local-id collision,
independently calculated circle centres/radii and signed arc endpoints,
rectangular corners and construction diagonals, undo/cancel/reopen, invalid
geometry and exact targeting. A separate acceptance test calls
`Session::accept_computed` from a scratch session that opens a different drawing;
it checks the adopted owner and entry, cancellation and undo, closing, the
original checkpoint and host file state. Results are recorded against the exact
source in the PR, not inferred from compiling these tests.

The pending locale file uses the existing underscore-normalised command keys
(`cmd.drawing_sketch_create`, `cmd.drawing_sketch_edit`,
`cmd.drawing_sketch_inspect`). It no longer duplicates main's
`cmd.sketch_cancel`. The actual #6448 loader/catalog sync is still required by
this older receiver. Likewise, when main's #3938 reaches it, retain **one**
`sketch.cancel` specification/label/icon and dispatch to the sheet cancel path
when a drawing is open, keeping main's complete model-cancel branch. This
increment neither copies that model-entry implementation partially nor claims
that the outstanding main-to-drawings composition has happened.
