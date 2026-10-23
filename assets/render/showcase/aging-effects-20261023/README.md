# SolidCraft Render Studio — Aging, Effects & Backgrounds

[Download the MP4](SolidCraft-Aging-Effects-Showcase.mp4) · [Downloadable package](../../../../distribution/SolidCraft-Render-Showcase.zip)

This deliberately shipped collaborator showcase uses actual native SolidCraft
renders and application captures, not generated illustrations or repainted CAD.
The video is an edited presentation of stills with comparison wipes and captions,
not a claim of real-time rendering, animation or physical dynamics.

## Video and editable delivery

The MP4 is 1920 × 1080 at 30 fps, H.264/AAC with fast-start playback. Exact duration,
frame count and byte hashes are in `video-receipt.json`. English captions are for
collaborator review. The low-level ambient soundtrack is original oscillator
synthesis, without third-party samples. No fonts, application executables or
media-tool binaries are distributed.

`renders/` keeps thirteen selected native PNGs; `screenshots/` contains real UI
views; `scene/effects-showcase.solidcraft` keeps the original 43-body visual study
and all thirteen canonical production/background shots. The original nine shots
and four added backgrounds were saved and reopened identically in both the old
render-producing application and the new Backgrounds-UI application, which exited
normally. Personal settings remained isolated. The first session's preference
hash baseline follows its explicitly recorded control reconnection.

One expected named shot, `09 Cyanotype`, was absent in the live inventory before
closure. It was restored from its retained numerical recipe. Native serialization
adds explicit default fields and omits null optional fields; exact roundtrip
comparison uses canonical stored settings. This restoration is disclosed in
`scene/shot-restoration.json`, not represented as an unexplained data-loss fix.

## Backgrounds: what is actually functional

The dedicated **Backgrounds** page combines three distinct controls:

1. **Camera presentation:** solid colour or vertical/horizontal/radial gradient.
   This affects uncovered camera pixels, not illumination, reflections or raw EXR.
   The neutral/gradient comparison uses the same exact raw master. Completed
   images are immutable: apply Regrade or render a new result to see changes.
2. **Physical stage:** a floor/wall or polygonal cyclorama receives light and
   participates in shadows/reflections. The examples have two and twenty-six
   panels respectively; the cove is twenty-four explicit strips, not an analytic
   smooth or infinite background. Fine faceting can be visible.
3. **HDR illumination:** the existing resources illuminate and reflect on the
   subject. They do not automatically become a visible camera panorama. A large
   physical floor/wall may cover the image and hide a camera-only gradient.

The ribbon now opens these controls together. Clearing a physical stage also
preserves the current unsaved background, exposure and output draft; it no longer
reloads unrelated older saved values. Existing replacement consent and explicit
transparent-output restrictions are unchanged. Live Preview is a modelling view,
not the completed camera-finishing image. The video includes three background
chapters with actual image comparisons and the new native control panel.

## Aging, materials and photographic presentation

Up to eight ordered original optical aging layers can be combined, starting with
rust, patina, deposited dust, grime, scratches, chipped paint and fading. Masks
have explicit world-mm scale, seed, strength and body identity. They change native
surface transport but not CAD geometry, physical material cards, mass or pose.
This is not elapsed-age prediction, corrosion kinetics, geometry displacement,
cavity detection or measured edge wear. Glass and passive stages are excluded.

Eight ordered original colour looks and bounded highlight-driven flare, ghosts,
halo, streaks, chromatic offsets, distortion, soft focus, sharpening and grain
are artistic image-space operations, not calibrated camera/film emulations.
They preserve the raw FLOAT32 EXR. Spatial effects require opaque output and
extend image edges. Regrading does not trace new rays or reapply denoising when
its geometry guides are absent. Six layers are combined in the aged comparison.

Airborne dust is a separate finite homogeneous participating medium that changes
transport and the raw master. Residual Monte Carlo grain is not a field of
individual dust particles. Optical compatibility restrictions remain explicit;
no lunar atmosphere is inferred. The older SELENE 4K chapter is labelled as an
earlier verified complete-assembly reference, not a newly aged rover render.

## Reproduce

The editable scene retains its portable original material/HDR resources and
named numerical shots. For example, with the explicitly built native CLI:

```sh
solidcraft-cli render assets/render/showcase/aging-effects-20261023/scene/effects-showcase.solidcraft --setup "02 Aged surfaces" --out aged.png
solidcraft-cli render assets/render/showcase/aging-effects-20261023/scene/effects-showcase.solidcraft --setup "13 Background - cyclorama" --out background.png
```

Use a new output destination or the application's explicit overwrite consent.
`tools/render-showcase/instrument.py` reproduces the source model commands;
`compose.py` checks source images against the public video manifest before
assembling the same editorial treatment; `package.py` verifies committed bytes
before packaging. Video assembly uses an external authoring encoder and local
system fonts; neither is an application runtime dependency or redistributed here.

## Acceptance and limits

Frozen render producer: `9f8c3d3031737b19a5bb1bf8d3802f3af6915c1e`.
The corrected full hardware GPU suite passed 115 tests with three explicit ignores;
renderer core passed 482 with CPU and 271 without it. The old UI suite passed 91
Render tests; new Backgrounds-page checks are recorded separately in `validation/`.
Strict Clippy, native builds and the official twelve-crate WASM checks have their
own receipts. The first GPU packing-test failure is retained: its expectation was
updated to the existing five-block material layout without changing transport or
weakening exact offsets/patch identity. These are not aggregate release/CI claims.

`SHA256.json` records the selected shipped files. Generated drafts, intermediate
EXRs, native-control streams, credentials and tool binaries remain outside Git.
Remote Actions status is disclosed separately. PR #11 remains draft: this package
completes the requested media/background acceptance, not every advanced-renderer
roadmap item or an upstream application release.

New code/media: SolidCraft contributors, MIT OR Apache-2.0. Existing resources
retain their attribution. The earlier SELENE reference credits 0xlillith,
SychPL / Mateusz Sych, SolidCraft contributors and Canonical Ltd. for modeled
lettering, with original notices in `assets/render/selene/SOURCE.md`. No font files
are included. Every shipped resource is listed in repository `ATTRIBUTION.md`.
