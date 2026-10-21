# Validation record

Date: 2026-10-21. Scope: the English catalog, preset specification and prepared SELENE resource contribution in PR #3.

## Package checks

The package validator completed with **0 errors and 0 warnings** on the authored data:

| Collection | Records |
|---|---:|
| Material concepts | 600 |
| Lighting concepts | 280 |
| Environment concepts | 280 |
| Renderer capabilities | 120 |
| UX requirements | 56 |
| Future simulation requirements | 36 |
| Roadmap phases | 19 |
| Quality presets | 10 |
| Studio presets | 24 |
| Environment presets | 24 |
| Complete scene presets | 24 |
| Current-contract setup examples | 6 |
| Analytic physics fixtures | 6 |

Reproduce from the repository root:

```sh
python3 docs/render-materials/validate_catalog.py --json
```

Checks include unique IDs and names, expected series and counts, required data fields, finite numbers, capability/provenance/source references, roadmap dependency cycles, sample and path-depth bounds, output capabilities, light geometry, passive modifiers, unit declarations, physical-vacuum environment rules, scene dependency unions, ground/rock role references and solar-source ownership.

The six current-contract examples were checked against the pinned `docs/render-settings.md`: command/field shape, backend/mode/format, setup names, dimensions, pixel product, color bytes and exposure/environment bounds. They were not executed in a SolidCraft application. Preflight against a real document must also account for existing setup names and the 32-setup total limit.

Analytic fixture expectations were recomputed for constant-gravity flight, millimeter/SI conversion including inertia and torque, offset release velocity, principal-inertia admissibility and sample mass bookkeeping. These checks validate the specification's arithmetic; they do not test an implemented physics solver.

Six deliberately invalid temporary variants were rejected: a duplicated solar source, an oversized baseline pixel product, an incorrect gravity unit, a missing PNG capability, hidden lunar ambient fill and acceptance of impossible principal inertia. The invalid variants are not committed.

Internal Markdown file links and exact attribution paths were checked. The data and documentation were reviewed for English wording, stable legacy IDs, source ownership, availability claims and clean-room scope. A second review identified and corrected missing quality dependencies, ambiguous temporal budgets, insufficient inertia constraints, inconsistent UX level names and publication sequencing.

## Source-model verification

The historical v7 SELENE native file was read as a container and its document inspected. SHA-256, file size, format, units, component/occurrence inventory, revolute joints, angle limits and selected vial feature/body references were recorded. No native-file rebuild, geometry regeneration, collision simulation or application render was performed by this contribution. The model itself is not bundled here.

## Prepared-resource continuation

The prepared package under `assets/render/selene/` contains 48 appearance recipes, a 99-command swatch input, 33 semantic roles, 10 stages, 12 shots, 12 mechanism review areas, a five-command read-only preflight, an eleven-state future dynamics sequence, a UI integration flow and two development tools.

Reproduce its read-only data check:

```sh
python3 assets/render/selene/tools/validate_prepared.py --json
```

The check completed with **0 errors**. It covers source-state honesty, historical-source exclusion, catalog references and input hashes, exact opaque PBR field/value bounds, swatch construction/assignment consistency, semantic-role references, inherited capability unions, physical-lunar Sun/ambient ownership, camera bounds and motion-state transitions. The full catalog validator also passed again with 0 errors and 0 warnings after all ten SELENE scene specifications were redirected to the improved-source lock.

Thirteen focused adapter checks passed using temporary original synthetic fixtures and the previously inspected historical v7 file: captured-byte identity; blocked pending source; successful reviewed synthetic opaque assignment; unknown target rejection; overlapping assignment rejection; dust-layer rejection in the opaque adapter; mismatched source digest rejection; boolean-opacity rejection; invalid-color rejection; empty-label rejection; legacy-v7 rejection even when relabeled; existing-output preservation; duplicate-native-JSON-key rejection. The synthetic fixture does not establish the improved model's identity. No fixture admission record or generated test output is shipped.

Four mechanism planning calculations were independently recomputed from the documented rounded inputs: actuator stroke and stop, rack travel, candidate gear magnitude and candidate screw travel. These are arithmetic checks, not a replay of the source mechanisms or a physical simulation. The mechanism guide records the coordinate precision and unbound direction/driver assumptions.

An independent review corrected two appearance-to-concept associations, confirmed the explicit layered/anodized surrogate descriptions, and identified input validation and shared-definition scope issues in the preparation tool. The final tool parses the same byte snapshot that it hashes and requires explicit review of all occurrences affected by a body-definition appearance edit.

The improved `SELENE-showcase.solidcraft` native file was not found in the bounded public-source search documented in `assets/render/selene/SOURCE.md`. Its source lock remains `awaiting_native_source`; native selectors, runtime mechanism evidence and full scene execution remain unresolved. The opaque swatch script was checked against existing command shapes but has not been executed or rendered in this environment.

## Redesign-aligned UI continuation

The UI specification was reconciled with the implemented upstream design at `728f753b8094e1a54c5cc8b7e453982afa028023` (0.18.4), keeping the personal integration base at `93a2922c5ee272fb605703ae84c05825b4a74607` (0.18.2). All 21 source-record blob pairs in `ui-integration.json` were checked against Git objects: 17 are identical; the recorded toolbar, navigation, document and shell-composition source files have differences that do not authorize an automatic base update.

The review verified 13 current command/icon registrations, the named UI-kit controls, the theme-token names and the distinction between the one Render tab and its Scene/Image ribbon groups. Existing `ws_render`, `appearance` and `render_image` artwork was previewed at 64, 24 and 16 pixels in light and dark through the repository palette. No SVG, palette or acceptance record changed. These previews are not application screenshots or evidence that the planned UI is implemented.

The catalog and prepared-resource validators passed again with **0 errors**; the catalog also reported **0 warnings**. Their coverage now includes the unbound UI contract, source-identity field consistency, current tab/picking declarations, disclosure/category rules, exact quality names and references, all twelve per-shot recommendation groups, preserved shot defaults, command reuse/proposal consistency and isolated swatch-opening declarations. The 56 stable UX requirement IDs remain intact. Four deliberately invalid temporary variants were rejected: invented current Render picking, a wrong quality label, a changed shot default and a false swatch runtime-execution claim.

An independent source review corrected the Render tab's selected treatment to `render_ink` on `render_soft`, and cross-file review aligned the lunar quality recommendations and six default controls. The primary lunar set is Sun azimuth, Sun elevation, exposure, camera framing, terrain seed and supported dust coverage. Appearance variants stay in the appearance context.

At this review, **99 local Markdown links** resolved and the new JSON contract had an exact attribution row. The existing commands and source metadata were inspected, not executed. No new runtime UI, render, screenshot, screen-reader result, usability measurement or platform-parity result is claimed. The acceptance matrix is work for the runtime implementation.

## Repository gate limitation

The required commands `cargo xtask ci`, `cargo xtask assets` and `cargo xtask icons` were attempted in the isolated checkout and could not start:

```text
cargo: command not found
```

The environment lacks the Rust toolchain. The PR therefore remains a **draft** until the relevant repository checks run on the submitted tree in a suitable environment. This is an actual uncompleted gate, not a claim that documentation requires no checks. The root attribution file is embedded by the UI and can expand CI's affected scope.

The same three required commands were attempted for the UI continuation and again could not start because `cargo` is unavailable. No successful Rust gate is inferred from the Python data checks.

The package validator and whitespace review do not substitute for repository CI, renderer reference images, native/WASM execution or physics validation. There are no screenshots of new runtime behavior because this PR introduces no runtime behavior.

## Not established by this contribution

- Runtime availability of the 1,160 content concepts or 82 future preset specifications.
- A production-validated appearance/HDR/terrain asset pack or calibrated source photometry.
- Measured optical, mechanical, electrical, thermal or granular properties.
- A photorealistic SELENE render, a reachable catch or a physically simulated animation.
- Competitive performance, platform parity or a successful full Rust CI run.

These are explicit implementation and validation milestones in the handoff, capability registry and roadmap.
