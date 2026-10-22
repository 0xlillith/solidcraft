# Render and materials development branch

## Fixed project baseline

The owner requested an independent development branch on 2026-10-21. The personal
repository is [0xlillith/solidcraft](https://github.com/0xlillith/solidcraft), and the
working branch is `lab/render-materials`. The application remains **SolidCraft**.

The baseline is upstream commit
`93a2922c5ee272fb605703ae84c05825b4a74607` (SolidCraft 0.18.2), preserved as
`snapshot/upstream-2026-10-21`. The personal `main` was fast-forwarded to that
same commit. The development branch retains its normal Git ancestry so that
reviewed changes can be integrated upstream later.

No scheduled job synchronizes, rebases or merges upstream into this branch.
Upstream changes are considered deliberately at a future integration point.
Development and validation are saved in the personal repository; upstream issues
and pull requests are references, not automatic publication destinations.

The existing engineering, licensing, asset and quality rules still apply.
In particular, `cargo xtask ci` is required before a feature commit. The personal
workflow uses ordinary GitHub runners, Rust 1.99.0 and one compile/test worker;
it does not depend on the upstream project's runner capacity or merge queue.

## Rendering and materials goals

1. A useful Render workspace: a large completed-image canvas, preview/result
   switching, fit and pixel inspection, explicit backend selection, captured
   scene information, progress, cancellation and export of that exact result.
2. A material editor that distinguishes engineering properties from appearance.
   Physical cards need explicit units, provenance, validation, document-owned
   persistence and inspection. Visual choices must not change mass or axes.
3. Production-quality CPU and GPU image paths, implemented in validated slices:
   antialiasing, physically based reflection, lights, environments, transmission,
   emission, surface detail and image finishing. A control is usable only when
   the selected executor supports its complete request.
4. An original appearance collection with editable values and honest capability
   descriptions. An illustrative visual preset is not measured engineering data.
5. Reproducible exports and portable resources. A format that cannot represent a
   material must refuse it explicitly instead of silently discarding properties.

## Source ledger

| Work | Exact source | Integration policy |
|---|---|---|
| GPU1 and asynchronous image workspace | [#2780](https://github.com/tangency-labs/solidcraft/pull/2780), `e1ee5b3b1b294700a594b863a08774795ccbd4ac`; includes [#2067](https://github.com/tangency-labs/solidcraft/pull/2067) | Preserve the author's commits in a normal merge; adapt subsequent UI work to this baseline. |
| Original opaque finishes | [#1366](https://github.com/tangency-labs/solidcraft/pull/1366), `1cc46354a469fc6f160c8dd69aef3247fdb88761` | Port the finish values and regressions while preserving the current explicit-color precedence; validate inherited PBR before editing. |
| Bounded local validation | [#2011](https://github.com/tangency-labs/solidcraft/pull/2011), `eb2da15272db19fc292d7c0541b53ae149e7cb11` | Port only the opt-in test-worker limit, its tests and help text. |
| Physical material persistence | [#1060](https://github.com/tangency-labs/solidcraft/pull/1060), `01c00a5eb6a71c1c181644a968593f8596deab7d` | Adapt to the current material types and pending-format mechanism; never import an old format number. |
| CPU ray tracing and image controls | [#1640](https://github.com/tangency-labs/solidcraft/pull/1640), `c728a0261fd57ce423bfc0aad32c9c766ad39af6` | Port rendering changes separately from historical kernel, vendor and CI repairs. |
| Advanced GPU studio | [#2191](https://github.com/tangency-labs/solidcraft/pull/2191), `d710e31d0eeb285d65678ff935c0e6d47a6dc06f` | Adapt the scoped studio, optics and resource changes with exact donor attribution. This port does not add `d710e31` as a merge parent; the retained donor merge ancestry is the GPU1 source above. |
| Vulkan image-device selection | [#3306](https://github.com/tangency-labs/solidcraft/pull/3306), `d489f0c03c20fea16210a1bed0048088c66322fe` | Port only the explicit Vulkan selection and matching-adapter checks; keep the viewport's independent device policy. |
| Advanced CPU recovery | Original local work based on `64339aedbc5e2f0d46e89dd915724deb06d5702d`; recovered render-only patch SHA-256 `746f4e18b75ae9e6e4095e7c40812bad92c20078275979e2ccef7b3605924b77` | Adapt optical and film components without importing historical kernel or vendor changes. See [CPU integration](render-cpu-advanced-integration.md). |
| Numerical render setup persistence (3A) | Original personal-branch work dated 2026-10-21; exact backend/UI patch digests below | Integrate on the unit2 r7 source tree, retain all preceding fixes, and validate this combined tree separately. |
| Embedded HDR resources (3B) | Original personal-branch work dated 2026-10-21; exact input patch digests below | Extend the numerical setup contract with immutable HDR bytes and explicit capture, retaining the runtime binding boundary. |
| CPU progressive sampling | Original personal-branch work dated 2026-10-21; exact backend/UI patch digests below | Integrate bounded camera accumulation and provisional image inspection with the saved HDR contract, preserving final-only export. |

Historical PR test results belong to their original commits. They do not certify
this branch or a new combination of features. Validation records must identify
the source, toolchain, adapter/backend and actual command result.

## Contracts to preserve

- A render owns an immutable captured scene. Check and Start use the same request;
  cancellation cannot publish a late frame, and Export writes the retained frame.
- Every job keeps its complete session/serial/generation identity. A completed
  image from an earlier capture may remain visible, with its origin made clear.
- Backend availability, preparation and request support are separate states.
  An unsupported feature must never cause a silent switch to a different engine.
- CPU raster admission remains bounded at 384 MiB; the shared image-host pool is
  512 MiB. Executors account for their own work and retained results.
- Light accumulates in linear space. Decode source color once, apply the selected
  display transform once, and encode output once.
- Dielectric IOR for opaque reflection is not glass transmission. Alpha is not
  a substitute for absorption, refraction or validated solid-medium geometry.
- New saved data uses its own pending-format descriptor and survives undo/redo,
  copying and cold reopening. Existing catalogs must reject unsupported data.

The unit labels below record the integration history. Their source changes are
consolidated into one personal-branch delivery, whose complete CI and observed
application-output acceptance must identify the same combined revision.

## First integrated unit

The first unit combines GPU1, the completed-image canvas, CPU ray tracing,
opaque finishes, the original starter appearance pack and physical material
cards with an in-panel editor. The desktop app and CLI enable `raytrace-cpu` by
default; `--no-default-features` retains an opt-out. The library feature remains
optional, and the browser never advertises a native CPU worker it cannot run.

The completed image has Fit and 100% inspection, pan/zoom, its capture revision
and executor identity. The image camera is independent of the product camera.
CPU controls include 1/4/16 primary samples, lighting presets, shadows and
exposure. GPU1 controls include samples, path depth, seed and exposure. Settings
are assembled once for Check and Start; CPU and GPU options cannot be mixed.

Physical Material supports creating, duplicating and editing a document-owned
material, its assigned density and optional engineering card. The card stores
nine property kinds, isotropic or orthotropic elasticity, units, sources and
conditions. Save executes one undoable command; it does not assign bodies.
Cancel discards a pending draft and does not undo a completed Save. Editing a
physical card preserves its optical appearance.

The starter pack has 40 original illustrative recipes. The ordinary Appearance
editor can assign the 34 opaque recipes; its six transmission recipes remain
disabled there. The second unit adds their explicit GPU runtime consumer with
geometry certification. Wood recipes in this unit are uniform optical values,
not procedural grain. These recipes are not measured
engineering properties. See [starter appearance pack](starter-appearance-pack.md)
and [physical material cards](physical-material-cards.md).

## Second integrated unit

The second unit combines the GPU studio, CPU optical extensions and portable
catalog v2. It preserves the first unit's physical editor, default native CPU
feature and completed-image canvas. The current integration and acceptance
status is tracked in [personal issue #1](https://github.com/0xlillith/solidcraft/issues/1).

| Capability | CPU image path | GPU image path |
|---|---|---|
| Opaque appearance | Shared scalar/PBR looks, GGX rough reflections and fixed 1/4/16 primary AA | Shared scalar/PBR looks and progressive path sampling |
| Illumination | Explicit studio lights, captured HDR and bounded reflection sampling | Environment maps or artistic sky, sun and rectangular emitters with MIS |
| Transmission | Runtime API for certified thin sheets and axis-aligned boxes | Whole-body runtime editor for thin sheets and certified closed solids |
| Surface detail | Runtime API for bounded procedural bump and roughness fields | Runtime editor for planar color/normal images and clear coat |
| Image finishing | Exposure and the existing display transform | Exposure, Reinhard/ACES fitted, depth of field and optional final denoising |
| Result | Retained final PNG through the shared job protocol | Retained final PNG through the shared job protocol |

These are distinct supported models. CPU emission is visible to camera and
reflection rays; it does not illuminate neighboring surfaces. CPU optical
transport rejects clear-coat combinations it cannot represent. CPU glass is
limited to the certified geometries above; general meshes, nested media,
caustics and colored transmission shadows are not accepted capabilities.

The GPU material editor obtains exact body names and ordinals from the captured
scene. Changing the document or revision requires explicit revalidation. Glass
recipes convert their original linear transmittance to absorption per millimetre;
they do not change saved alpha. Glass cannot share a body with coat or image maps.
Check and Start build the same request; geometry certification may occur during
asynchronous job preparation. See [Render workspace](render-workspace.md).

Catalog v2 copies complete opaque looks and validated physical cards, including
their sources and conditions. Applying a copied label cannot select a recipe by
accident, and component-instance targets use ordinary world-to-local assignment
mapping. V1 remains readable; explicitly exporting to v1 refuses richer data
before writing. See [local libraries](book/src/dev/local-libraries.md).

## Numerical render setup persistence (3A)

The 3A source integration is complete on unit2 r7 tree
`a022afae3f855e0ba8bcf8aec93287fca38b956e`. It retains all of that tree's preceding
source fixes. Its two frozen inputs are original personal-branch work:

- Backend/document/IO patch SHA-256:
  `df19dfd8620b5e7d1af01b995cf00c58e7d10ab50cbaee84ac0456d8d6392e55`.
- Workspace UI/locales patch SHA-256:
  `bbce164d058838222dec71c090a8346808325528393349bc526c65bfacc7bb96`.

A named `RenderSetup` now optionally owns one complete numerical `RenderJob` for
its active raster, CPU raytrace or GPU path method. The format change has its own
`render-job-intent.rs` pending descriptor. The implementation includes document
validation, undo/redo and file/container round-trip regressions, strict command
parsing, common lowering before dispatch, and the existing Render setup UI.
See [persistent settings](render-settings.md) for the complete fields and limits.

Save does not require a renderer or evaluated geometry. The method selector can
prepare unavailable CPU/GPU intent while Check and Start retain their execution
gates. Both consume the same complete setup; parallel runtime controls cannot
silently override it. `RenderSetup.exposure_ev` is the one saved exposure authority.
The live product preview retains its independent fixed studio response.

The panel saves, reloads and switches complete numeric drafts, including fields
without dedicated editors: CPU area sampling/seed and spot cones, GPU procedural
sky gradient/radians, full RGB studio lighting, lens and finishing options.
Absent values, explicit empty light lists and black environments remain distinct.
Typing a new setup name is Save As. Camera restoration remains explicit, and
switching a setup preserves completed-image inspection/export state.

The source includes regressions for offline saving, exact request lowering,
method/host separation, values not edited by the UI, Save As/Reload/setup switching,
rejection atomicity and persistence. Static checks completed for this preparation
are directed Rust formatting, diff whitespace, JSON/locale coverage, exact replay
of both input patches and preservation of all r7 fixes. Materials and UI authors
also reviewed their shared DTO boundary independently. These checks accept the
source preparation only. Publication requires the complete CI gate, native/wasm
tests and observed application-output QA on the combined source. Validation of a
preceding unit2 tree does not certify 3A; publication records must identify the
exact source and results of the combined revision.

## Embedded HDR resources (3B)

The HDR increment extends the combined studio/3A source tree
`608ed47d2a406f54f2c5cecc4e9a2d4d5a0f62fd`. Its frozen inputs are:

- Document/engine/IO/shared-source patch SHA-256:
  `399cca9c83167596b326e1cd9b7941301a210164d21801ebfa967572928792fc`.
- Workspace UI/locales patch SHA-256:
  `44be399412bc8b1a1fec0eeb6a97c71342a47b991b5ff83082dbec4781d83cf1`.
- Saved-job CLI example patch SHA-256:
  `a9b1d15ee8a6c6a2f502e7d399aba6cf68d9c8c83b6f699bc4ad31822014c47e`.

An explicit inspection captures an HDR file's digest and metadata. Saving its
setup then verifies the same digest and installs the immutable source and setup
in one undoable command. A changed file is refused until it is explicitly
inspected again. The UI switches to the embedded resource only after Save
succeeds; Check and Start use the same selection and numerical controls.

Named CPU and GPU setups share resources by SHA-256. Design JSON contains the
complete bounded payload, while the design container uses its existing hashed
blob storage. Both reopen without the original path. Document snapshots and
undo share immutable source buffers; GPU and CPU consumers also retain those
buffers while charging their full capacity against each job's budget. Limits
are 16 MiB per source, 64 MiB total and 64 resources. JSON admission checks logical
base64 size, including escapes, before building the payload Value; container
admission checks references and sizes before reading the HDR blob.

CPU requires declared sRGB/D65 primaries. GPU permits an explicit linear-sRGB
assumption only for an undeclared source. Neither path silently converts an
unsupported declared color space. An embedded HDR can also accompany temporary
runtime body bindings, without reconstructing a path or persisting those bindings.

The CLI evidence example selects its host from inline or named saved intent.
CPU requests remain independent of GPU initialization; metadata reports the
executor actually selected. See [HDR resources](render-hdr-resources.md) for the
commands, color policy, file round trips and retained-resource limits.

Resource deletion/garbage collection remains a follow-up. Saved design files
and autosave snapshots include the HDR, but replaying a command-only crash
journal before a checkpoint still needs the original file and digest. Capture
does not pre-serialize the whole design; container saving retains its existing
global size gate. Publication requires CI and observed save/reopen/render QA on
the complete integrated source, including file-change rejection and reopening
after removing a temporary copy of the original HDR.

## CPU progressive sampling

The next combined source adds native CPU camera accumulation to the HDR unit.
Its two frozen inputs are:

- Film/host/document/engine patch SHA-256:
  `70c63b11333645400b175e267924bac866c31970a27bbb5186f9476880c0288e`.
- Workspace UI/locales patch SHA-256:
  `288d4d09de10c702fb8c9a39eb2d5ae910401e826c17e752593cdef55bf027be`.

CPU raytracing offers fixed 1/4/16 primary AA or progressive camera sampling.
The progressive option uses 1..64 samples per pixel and a separate u32 seed,
with at most 1,048,576 output pixels and 16,777,216 camera samples in total.
Changing samples leaves the requested dimensions intact; the UI offers an
explicit resize action when the draft exceeds those bounds. The same limits
are validated while saving a setup without a render host.

Samples accumulate linear RGB and primary coverage. Deterministic counters
depend on seed, pixel and absolute sample ordinal, so changing the final target
or worker quantum preserves earlier sample prefixes. Only complete passes at
1, 2, 4 and subsequent powers of two below the target become provisional PNGs.
The exact target produces the final image, including non-power-of-two counts.
Exposure and the display transform are applied once during each resolve.

Each provisional image has its own sequence and admitted memory reservation,
under the existing job and capture identity. Upfront admission covers retained
checkpoints as well as working film and final output. Cancellation, failure and
completion clear the provisional slot. Only the final result can become the
session's retained completed image or be exported. The canvas preserves its
inspection camera across checkpoints and final publication; a cancelled job
returns to the previous completed image.

Saved CPU intent includes samples and seed together with embedded HDR, hidden
numerical values and one exposure authority. The new field has its own pending
format descriptor. CPU requests use nested `raytrace.progressive`; top-level
`progressive: true` keeps selecting the GPU executor.

This increment samples the existing direct-light and selected-reflection CPU
transport. It does not add diffuse global illumination, adaptive convergence,
CPU denoising or unrestricted glass. Its initial progressive combinations
exclude area/LED lights, dielectrics and anisotropic-face overrides; fixed CPU
rendering retains its existing support. See [the complete contract](render-cpu-progressive.md).

The source includes backend, persistence, engine and UI regressions for prefixes,
linear averaging, PNGs, retained-memory accounting, publication/cancellation,
offline saving, HDR reopening, explicit resize and preview-to-final navigation.
Independent static review found and corrected a completion/preview UI race.
These preparation checks do not certify execution. The integrated revision
requires full CI and observed progressive, cancel/export and save/reopen QA.

## Remaining acceptance and development

External image maps and CPU/GPU optical body or face bindings remain
explicitly temporary per-document-tab state. Saving a complete job containing
those inputs is refused before document mutation, including disabled GPU binding
rows. Legacy nondefault environment rotation/intensity are retained and require
explicit conversion; they are not silently cleared. Image-map embedding and
stable document-owned target identities require a subsequent contract and
migration. Catalog v2 continues to transport opaque appearance and physical-card
data; it does not export HDR environments, runtime maps or body bindings. Only
the active image method is saved; inactive method drafts remain local.

CPU diffuse global illumination, broader progressive optical combinations,
EXR export and an unrestricted final-quality mode remain separate work.

Each integrated unit needs its own native, wasm, regression and observed-output
validation. Source recovery or a donor's green CI does not mark it complete on
this branch.

Completion means usable controls, supported engine behavior, persistence where
needed, regression coverage and observed application output. A placeholder
control, a parsed schema or a backend enum alone does not complete a capability.
