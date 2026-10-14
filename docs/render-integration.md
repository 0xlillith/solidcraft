# Phased render and local-library integration

Tracked in #842; the parent render design remains #620. Contributions retain their
original ancestry and are validated together on a pinned upstream revision. The
integration PR is a reviewable combined tree; it does not assert that any constituent
PR has merged. Maintainers can choose the combined or individual merge path.

The refreshed stack incorporates upstream `3669d4b3`. Upstream now owns document
format 19 for editable associative offset dimensions, so saved render setups use format 20 with
explicit migration coverage for 17, 18 and 19. The Design viewport retains upstream GPU
tessellation; the separate product scene continues to upload its prepared CPU meshes.
The already-merged HDR input contribution is retained through its original ancestry.

| Phase | Existing contribution / deliverable | Completion gate |
|---|---|---|
| 1 | R2 #667, R3 #706, R4a #800: saved setups, integrated product workspace and bounded CPU PNG jobs | Setup save/reopen/undo, document isolation, strict capabilities, cancellation, native exports and full repository gate |
| 2 | HDR input #837: cooperative Radiance preparation alongside the existing job host | Codec/host tests together, canvas format guard, unchanged CPU output/capabilities, native and wasm gates |
| 3 | First local scalar library #846, building on #785/#786 and proposal #792 | Portable catalog round trips, bounded hostile input, explicit revisions/conflicts, copied definitions, undo/cold reopen and actual UI evidence |
| Follow-up | Texture sets, wood grain/mapping and HDRI/PBR renderer consumption (#813) | Shared contracts proposed in #1004, actual lighting/texture interpretation, resource admission and separate native/browser visual evidence |

## Current capability boundary

The GPU host provides the product **viewport**. The CPU product executor provides
bounded preview **PNG images**, with approximate sorted transparency. The Radiance
utility prepares linear RGB values in their source colour space; it is not a lighting
implementation. Adding its codec never adds HDRI/PBR/refraction/final/EXR support to
an executor. Unsupported requests remain explicit errors, preserving the last valid
image and CAD state. See [backend contracts](render-backends.md),
[settings](render-settings.md), [workspace](render-workspace.md),
[jobs](render-jobs.md) and [HDR input](render-hdr-input.md).

## Local-library first slice

Reuse existing `Look`, design-local physical materials and assignment commands.
Physical density, visual colour/opacity and render environments remain distinct.
The first catalog handles scalar appearance and density/look definitions only; it
contains no texture files, PBR parameters or environment descriptors. Selected values
are copied into the existing document representation so reopening does not need the
catalog. Catalog provenance/revisions remain in the catalog; this first slice adds
no persistent catalog-update link or new document-format version. It does not claim
the complete resource/provenance embedding proposed in #792.

Catalog inspection/export and application must use engine commands. Programmatic
calls never open a picker. An exact selected revision is required for application;
changed files and conflicting material names cannot silently alter a selection.
Applying an appearance must not change density/mass. Physical-material definitions
reuse existing creation/assignment behavior and have explicit conflict handling.
The source design and library draft/file are separate: exporting a reusable snapshot
does not edit the CAD design, and applying a definition is one undoable design edit.

## Validation and publishing

Run `cargo xtask ci` before each integration/code arc is committed. Record actual
counts, platform, pinned upstream and any failure in the PR, separately from remote
checks. Existing Windows kernel issue #796 is not fixed or suppressed by render work.
No geometry baseline or test assertion is weakened to publish an integration.
On upstream `3669d4b3`, the refreshed full local run also reported robustness-golden
failures; the engine and GPU test binaries were bounded after 15 minutes without
complete summaries. Those interrupted binaries are not passes. The PR records the
exact failed cases, focused reproductions and limits separately from completed gates.
Native compilation, wasm compilation and actual browser interaction are separate
claims. Use original small examples and actual app/PNG evidence for visible changes;
do not commit generated review media or commercial reference assets.

Private `plan/` documents are not present in this checkout. The public contracts and
current repository instructions govern the available work; no missing contents are
invented. Renderer/resource representations are proposed in reviewable phases in #813, open to
maintainer and contributor feedback. No unpublished descriptor is assumed.
