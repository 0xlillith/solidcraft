# Phased render and material integration

Tracked in #842, with parent render design #620 and physical/visual material roadmap
#922. The combined integration keeps contribution ancestry; it is an alternative
review path, not evidence that the individual PRs have merged. Each result in GitHub
is pinned to its tested source revision. Do not reuse an older green result for a new
merge or describe compiled wasm as verified browser interaction.

## Delivery phases

| Phase | Contribution / deliverable | Acceptance gate |
|---|---|---|
| 1 | R2 #667, R3 #706, R4a #800: document-owned setups, product viewport, bounded CPU PNG jobs | Setup migration/save/reopen/undo, document and job isolation, strict capabilities, cancellation, native evidence and repository gates |
| 2 | Merged HDR input #837 and surface reference #1019 | Bounded codec/BRDF tests; unchanged existing CPU capabilities until actual consumers exist |
| 3 | Opaque product PBR #1046; engineering-card persistence #1060 | Explicit supported optical fields, source/condition preservation, one-step undo, cold reopening, unchanged geometry and independent assigned density |
| 4 | Scalar catalogs #863; richer transport #1086; inspector/import #1094 | Exact selected revisions, copied definitions, hostile-input bounds, no lossy downgrade, scoped file access, reviewed import and actual native evidence |
| 5 | Environment consumer #813, then texture/wood mapping #1004/#786 using resource admission #909 | Actual linear/sRGB interpretation and lighting, persistent object-local frames, cancellation/memory bounds, transformed instances, separate native/browser evidence |
| 6 | Family-specific optical/data quality under #922 | Wood L/R/T and moisture/grade conditions; polymer grade/process/temperature/rate; glass thickness/IOR/absorption; metal alloy/temper/conductor response and brushed anisotropy; source/license review before shipping data |

The released document format is 19; unreleased persisted additions share format 20.
Saved render setups migrate from 17/18/19 with older-reader refusal. The Design
viewport retains upstream display-plan tessellation; Render uses its separate
prepared product meshes. Neither runtime jobs nor device handles are persisted.
See the versioned contracts rather than reserving another format independently.

## Current foundation boundary

The GPU host provides a product **viewport**. The CPU product executor provides
bounded preview **PNG images**, with approximate sorted transparency. The Radiance
utility prepares linear RGB in the source colour space; it is not lighting. The
numerical opaque surface contract is not proof that a host renders those properties.
Imported resources or saved intent never silently add HDRI/refraction/final/EXR
support. Unsupported requests fail and preserve the last valid image and CAD state.
See [backends](render-backends.md), [settings](render-settings.md),
[workspace](render-workspace.md), [jobs](render-jobs.md),
[HDR input](render-hdr-input.md) and [surface reference](render-surface.md).

Both atomic PNG publication modes check the existing caller file scope. The
control-channel regression rejects outside folders and traversal and permits a
valid destination, including explicit overwrite. This guard and document-adoption
regression now belong to standalone R4a, so its safety does not depend on this
combined integration. Source production code in this integration matches that
foundation; the additional guide is not a second renderer or file-access service.

## Picker integration before release

#1114 replaces blocking native pickers to fix a macOS abort and keeps the response
bound to the requesting document. Open render/library consumers must adopt that
contract when integrated. For PNG export, capture the explicit completed job ID
and overwrite choice before requesting the dialog. Do not substitute a newer image
when its answer arrives; a closed document drops the answer and an evicted result
fails explicitly. Verify delayed answers, cancel, tab switching/close and result
replacement. Keep this pending dependency visible; the synchronous picker must not
be described as safe on macOS merely because the clicked test harness passes.

## Material, appearance and resource ownership

Physical properties, assigned density, visual appearances and environment lighting
are distinct. A card-only edit does not replace assigned density or visual style;
applying an appearance does not change mass. Missing engineering values stay unknown,
with per-property sources and measurement conditions. Numerical consistency is not
certification or a structural solver. Mechanical axes are not inferred from a texture.

The first scalar catalog remains usable without optical/engineering extensions.
Richer transport must preserve copied PBR/card records and reject lossy downgrade;
reopening a design must not require the source catalog. Keep source revisions,
conflicts and applicability explicit. All inspection/application uses existing engine
commands; programmatic calls never open pickers. Library export edits the reusable
file rather than the CAD design. Applying a definition is one undoable design edit.

Use original synthetic fixtures first. A licence string in imported data is a source
claim, not redistribution permission. Any shipped dataset or texture needs actual
source/license review and attribution. Commercial reference artwork is not copied.
No second renderer, certified material database or simulation solver is implied.

## Verification and collaboration

Run `cargo xtask ci` before each committed arc. Record local/remote results, pinned
upstream, actual platform and failures separately. Do not weaken geometry baselines,
mask warnings or infer whole-app performance from cached capture/cancellation times.
Attach actual app/PNG evidence for visible behavior; label older evidence with its
source revision instead of relabeling it as current. Use scoped original examples.

The private `plan/` documents are unavailable in this checkout; no content is
invented. Public contracts and current instructions govern this work. Propose bounded
interfaces in GitHub where a handoff is missing, invite feedback, and check existing
claims/PRs before implementing. Keep independent contributor branches untouched.
