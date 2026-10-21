# Prepared SELENE render and motion resources

This is the concrete authoring slice of the [render/materials catalog](../../../docs/render-materials/README.md), prepared for the personal SolidCraft render branch. It includes **48 original appearance recipes, an executable 48-swatch script, 33 semantic roles, 10 stages, 12 shots, 12 mechanism acceptance areas and an 11-state future dynamics sequence**.

**The selected native source is the improved `SELENE-showcase.solidcraft`, documented in Tangency Labs' project. Its file has not been obtained from the public locations reviewed.** [source-lock.json](source-lock.json) therefore remains `awaiting_native_source`. Exact improved-source hashes, IDs and transforms are deliberately unresolved. The historical Rover v7 is excluded as a substitute. [SOURCE.md](SOURCE.md) records the evidence, search scope and admission requirements.

## What is ready, and what it means

| Files | Prepared content | Current meaning |
|---|---|---|
| [Appearance pack](materials/appearance-pack.json) and [guide](materials/appearance-pack.md) | 48 original numerical recipes; separate current opaque projection and advanced optical intent | Authored values, unmeasured and not visually validated. The manifest is not a portable scalar library. |
| [Appearance swatches](materials/appearance-swatches.json) | 99 existing-command operations: new design, 49 boxes and 49 appearance assignments | A complete input for a current opaque preview. No native SELENE file is needed. It has not been replayed in this environment. |
| [Roles](scenes/roles.json) | 33 geometry, appearance, layer, optical and source roles | Semantic selections awaiting review against the improved native source or an original stage generator. |
| [Stages](scenes/stages.json) and [shots](scenes/shots.json) | 10 explicit stage configurations and 12 camera/composition recipes | Future renderer inputs with source ownership, capability gates and no guessed native transforms. |
| [Mechanism acceptance](motion/mechanism-acceptance.json) | 12 review areas and four analytic planning examples | Documented behavior to preserve, with native/runtime evidence still unresolved. |
| [Read-only preflight](motion/preflight-commands.json) | Five supported inspection commands | Inventory of joints, studies, appearances, physical materials and render scene on an opened design. |
| [Sample sequence](motion/sample-sequence.json) | 11 states for collection, closure, release, flight, capture, storage and failure | A future dynamics contract, not a canned motion clip or simulated result. |
| [Demo UI flow](ui/demo-flow.json) | Simple three-step scene/quality/render flow and contextual advanced controls | An integration specification for the existing Render workspace. |
| [Preparation tool](tools/prepare.py) and [validator](tools/validate_prepared.py) | Native inspection, review templates, guarded opaque overlay compilation and package checks | Python 3.11 development tools only; no application runtime dependency. |

The 600-material, 280-light and 280-environment catalog remains the broader design inventory. These 48 appearances are realizations of existing concepts, not 48 additional concept IDs. The 10 stages and 12 shots specialize existing presets for SELENE.

## Inspect the available opaque previews

From the repository root, in an environment with the built SolidCraft CLI:

```sh
mkdir -p target/render-selene
solidcraft-cli run assets/render/selene/materials/appearance-swatches.json --save target/render-selene/swatches.solidcraft --results
solidcraft-cli render target/render-selene/swatches.solidcraft --out target/render-selene/swatches.png --width 768 --height 576
```

The script starts a **new design**. Use it as a standalone CLI input, never as an overlay on a working assembly. Save native output with `--save`; `--out` on `run` selects a geometry export. The render command uses the current fixed-light opaque preview. These boxes do not demonstrate transmission, emission, texture detail, coatings, anisotropy, terrain or path tracing. Image generation and visual review remain pending; no preview screenshot is bundled as evidence.

The numerical projection has exactly `color`, `opacity: 1` and `pbr: {metallic, roughness, dielectric_ior}`, plus a custom appearance label. All values are original illustrative controls. Glass, window and diffuser swatches are clearly named opaque inspection surrogates; the LED phosphor face is unpowered. Physical density, friction, electrical properties and flight qualification are not inferred.

## Admit and prepare the improved native model

The missing external input is the actual improved native file, with a source link or original attachment establishing its identity. Once available:

1. Keep an unchanged copy as the source authority and inspect its bytes and metadata.
2. Review the source record and notices, including retained original-model lettering and any helper geometry. Fill the source lock from actual evidence and complete its explicit admission review.
3. Review the mechanism manifest against native components, joints, links and studies. Preserve the improved tour and model hierarchy; create a named derivative for any further work.
4. Generate a body-role template and resolve its eligible opaque targets against the **evaluated** model. A declared history name alone does not prove that a body survives the feature history.
5. Compile and apply the reviewed subset to a new native output, then inspect the result and run the mechanism regressions. Keep the source hash, review file and applied command script beside the derivative.

```sh
python3 assets/render/selene/tools/prepare.py inspect --model path/to/SELENE-showcase.solidcraft --out target/render-selene/source-inspection.json
python3 assets/render/selene/tools/prepare.py template --model path/to/SELENE-showcase.solidcraft --out target/render-selene/body-bindings.json
```

Those commands **do not approve a source**. After explicit source and target review, the compiler can be used:

```sh
python3 assets/render/selene/tools/prepare.py compile --model path/to/SELENE-showcase.solidcraft --lock path/to/pinned-source-lock.json --bindings path/to/reviewed-body-bindings.json --out target/render-selene/opaque-overlay.json
solidcraft-cli run target/render-selene/opaque-overlay.json --in path/to/SELENE-showcase.solidcraft --save target/render-selene/SELENE-opaque-review.solidcraft --results
```

The committed pending lock intentionally blocks compilation. A pinned lock must match the actual source digest, size, format and units and contain a recorded identity/notice review. The current adapter is checked against native format 21 and the pinned opaque appearance contract; a newer source format requires adapter review against the executor that will open it.

An overlay is a sequence of existing commands, not an atomic engine transaction. A failed script can leave the in-memory copy partially changed. Work on a separate process/copy and retain the original file. The tool never opens the application, executes commands or rewrites a native design. Outputs refuse replacement. Do not use `timeline.export_script` to round-trip the assembly: its current contract omits appearance overrides and does not support component/joint designs.

The adapter emits only reviewed **subject-body opaque appearances**. It excludes glass and other optical roles, dust layers, emitting surfaces, light sources, framing selections and generated terrain. Those require their own bound renderer resources. It rejects duplicate targets, unresolved roles and the known historical v7 hash. A different concept than the suggested role material requires a recorded override reason. This is a preparation aid, not a geometric selector, solver, visual approval or proof of photorealism.

Body appearance edits can affect every occurrence of a shared component definition. The review must explicitly confirm that scope. Generated instance names are not accepted as stable source-body names; two instances that need different finishes require a separately supported binding or an explicit reviewed model derivative. The compiler does not create occurrence-specific appearance overrides.

## Scene and UI integration

The [scene guide](scenes/README.md) explains the 12 prepared shots: low/high Sun, sampling macro, LED work patch, polar ridge, transfer, release/capture coverage, full-shadow audit, neutral studio, turntable, labeled editorial hero and vial closure detail.

Resolve stage geometry, appearances, source ownership, camera framing and executor support before mutation. Physical lunar scenes have one finite-angular Sun, reflected light from actual geometry and explicitly owned hardware emitters where requested. They have no automatic sky fill, atmosphere, Earth disk or star overlay. The terrain is an original conceptual procedural stage, with no implied named-site accuracy. Exact contact placement and swept-volume clearance require the admitted rover bounds.

Integrate the authored UI flow through existing command, resource, theme and camera-navigation systems. The default flow is **choose scene → choose quality → render**, with availability and actual blockers visible. Advanced controls expose layers, units, mapping, lighting, camera, output and provenance in context. A preset never changes engineering or simulation properties, and unsupported features are never silently removed.

## Reuse motion; add validated dynamics

Keep the improved source's existing IK, motion links, closed loops, traces and tour. The [motion guide](motion/README.md) separates published demonstrations from native inspection and runtime replay.

The future sample sequence extends that foundation with a complete independent vial, closure and grasp states, mass/inertia, contact proxies, bounded actuation, continuous collision detection, regolith bookkeeping and release/capture events. It includes a visible failure path. A visually plausible parabola alone does not validate a catch. An approved bake remains independent of render resolution, sample count and frame rate.

## Validation and provenance

```sh
python3 docs/render-materials/validate_catalog.py
python3 assets/render/selene/tools/validate_prepared.py
```

These validate authored data and command projections. They do not run the application or establish runtime readiness. [The validation record](../../../docs/render-materials/VALIDATION.md) records actual checks and the Rust-toolchain limitation. Full repository gates and actual visual/mechanism evidence are required before merge and production publication.

All added recipes, procedural choices, metadata, scripts and prose are original clean-room work under the repository's MIT OR Apache-2.0 choice. Exact non-Markdown paths are listed in [ATTRIBUTION.md](../../../ATTRIBUTION.md). No external native model, texture, HDRI, media, commercial CAD asset or GPL implementation is bundled. [The clean-room record](../../../docs/render-materials/CLEAN_ROOM.md) and source lock document the limited public references.
