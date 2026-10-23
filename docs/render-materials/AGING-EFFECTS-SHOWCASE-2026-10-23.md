# Layered aging, photographic effects and native showcase

Owner-directed integration work on `lab/render-aging-showcase`, draft PR #11, based on committed `8c3f72ca` from the personal `lab/render-scale-resources` branch. The inherited uncommitted geometric-displacement work in the previous worktree is deliberately preserved untouched. Nothing here merges upstream, updates main, publishes a release or declares the entire advanced-material roadmap complete.

## Layered surface aging

The saved GPU option `aging` is an ordered list of at most eight original procedural optical layers. Available starting recipes are iron oxide/rust, copper-green patina, deposited dust, grime, fine scratches, chipped paint and faded coating. Layers can be enabled, disabled, reordered or removed; each retains strength, coverage, world-millimetre pattern scale, deterministic seed, origin, colour, roughness, metallic fraction and upward-facing deposition bias.

The same modified material closure feeds direct and indirect GPU transport and the native denoiser guides. An enabled layer therefore changes the raw linear master as well as the displayed PNG. It does not rewrite source textures, engineering materials, mass, CAD geometry, model visibility or pose. Passive render-stage panels and transmissive surfaces are excluded. An explicitly named glass target refuses rather than being made opaque.

Targets are all eligible opaque bodies or an exact captured ordinal/name pair. Named targets must still match. The inline name is bounded to 256 UTF-8 bytes; the stack has fixed storage rather than an unbounded JSON vector. Fields are world-anchored, not occurrence-local animation bindings. The periodic coordinate domain is explicitly rebased for the GPU and limited to 4,096 repeats across a body. Choosing an excessively small scale refuses instead of silently changing the requested detail.

These are authored optical approximations, not measured corrosion kinetics, elapsed-year prediction, true grooves, displaced silhouettes, cavity detection or curvature-derived edge wear. The scratches are a dominant-axis projected pattern. A contamination layer also covers the earlier clear-coat weight where its mask applies. Material classes refer to the captured optical metallic fraction, never chemical identification.

## Airborne dust is separate transport

The Aging inspector also links the existing finite homogeneous-medium editor. This is the previously implemented Beer/HG participating volume, not an additional surface layer or an image fog overlay. All explicit volume, glass, denoising and transmission-strata compatibility checks stay active. The UI does not disable incompatible options behind the user's back and never adds a physical atmosphere to a lunar scene automatically.

## Creative looks

Up to eight ordered display-linear look layers can be combined with individual strength and enable state: monochrome, sepia, cyanotype-inspired, warm, cool, teal/orange, faded print and bleach-bypass-inspired. These are original numerical transforms, not imported commercial film LUTs or calibrated camera emulations. They run after tone mapping and before the existing contrast/saturation/vignette finishing.

## Photographic and lens-inspired controls

The optional `finishing.photographic` object provides highlight-driven flare, ghost rings, a halo, horizontal anamorphic-style streaks, radial chromatic edge offsets, radial distortion, soft focus, sharpening and deterministic grain. Flare source discovery uses at most four separated high-luminance points from the actual scene-linear image, with an explicit linear threshold. No arbitrary screen light is inserted when there is no source above that threshold.

Spatial sampling is performed in decoded display-linear RGB, not by interpolating encoded sRGB values. Source sampling extends image edges. These artistic spatial effects currently require opaque output: a transparent request is refused, not silently flattened. Original raw EXR bytes bypass the entire display path. Depth of field remains the separate physical thin-lens control; the new flare/distortion controls are not a calibrated optical-element simulation.

The processing work is split into cancellable bounded quanta. Additional output/scratch storage is explicitly admitted before rendering or retained-master regrading. The source image and original master leases are retained until publication, and partial photographic results are never exported as completed frames.

## UI and commands

The shared inspector now has local Aging and Effects pages alongside the existing model/material/light/environment/camera/output sections. It reuses Nateaux's shared sections, lists, dropdowns, numeric and colour fields, buttons, icon strip and existing original SVG aliases. There is no parallel widget or icon library.

`render.aging.describe` and `render.effects.describe` are read-only registry commands with schema/recipe descriptions. Activating them from the ribbon or command search opens the corresponding editor rather than silently applying a preset. Numerical edits belong to the same draft and are persisted by the existing Save workflow. Source EXR, immutable selected result identity and the asynchronous export guards are unchanged.

The standard `render.result.regrade` action can apply looks and photographic effects to a selected retained master without reevaluating geometry or tracing new rays. Denoising is still not reapplied to a raw master because its geometric guides are not retained. Aging layers require a new render because they change the actual surface closure.

## Showcase provenance

The requested MP4 will contain only actual native application captures and actual SolidCraft-produced renders, with clearly labeled editorial transitions, captions and before/after comparisons. No generated illustration or repainted substitute can stand in for renderer output. Any edited video camera motion is a presentation move over a still frame, not evidence of real-time rendering or a simulated model animation. Final resolutions, sample counts, immutable PNG/EXR hashes, source revision and video checksum must be recorded after execution.

Video assembly tooling remains isolated outside the application dependency graph. No fonts, executable or video tool binaries are redistributed in the showcase package. The language-neutral renderer schemas and original English source remain unchanged by localized UI labels.

## Acceptance status at implementation checkpoint

Static validation of the combined path shader and denoiser-guide shader passes with a 1,168-byte parameter layout. This is not native GPU execution evidence. Tests cover bounded layer storage, hostile input, reorder/disabled identities, periodic field rebasing, look order, deterministic photographic quanta, cancellation, transparent-output refusal, exact raw-master preservation, native layer execution and UI save/reopen behavior. The numerical, native and visual results must be filled in from actual runs; they are not inferred from these test sources.


## Background usability and delivery closure

The Backgrounds ribbon previously led only to physical stage controls on the Scene page, while camera colour/gradient controls were elsewhere under Environment. It now leads to a dedicated shared-kit Backgrounds page containing camera presentation, physical stage controls, an exact-master regrade action and a separate HDR-library link. Merely navigating does not change a shot, renderer or document.

The page explicitly explains that a completed image is immutable, Live Preview is a modelling view, and a large physical floor/wall can hide a camera-only backdrop. A gradient affects only uncovered camera pixels; it does not recolour the lighting, replace visible geometry or turn the HDR into a panoramic background. Transparency remains explicit. Physical-stage replacement still requires consent.

A real numerical-draft loss bug was also corrected: clearing a physical stage now forwards the current complete draft to the registered clear-stage command, rather than restoring other values from an older saved shot. Stage controls no longer nest a second same-titled section. Regression sources check navigation, nonmutation and exact preservation/reopen of unsaved camera background, exposure and output dimensions.

On the frozen native producer `9f8c3d30`, actual 1920x1080 background images completed: no-stage neutral and radial-gradient presentations share an identical raw EXR; the two-panel floor/wall and 26-panel polygonal cyclorama change the raw transport and retain the same subject/camera. These are real native renders, not video-editor substitutions. Their source/header hashes and visibility limitations are retained in the delivery manifests. The new UI-only build/acceptance and the final video/package checks are recorded separately after execution.
