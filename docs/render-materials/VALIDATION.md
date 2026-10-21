# Validation record

Date: 2026-10-21. Scope: the English catalog and preset specification contribution in PR #3.

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

The pinned SELENE native file was read as a container and its document inspected. SHA-256, file size, format, units, component/occurrence inventory, revolute joints, angle limits and selected vial feature/body references were recorded. No native-file rebuild, geometry regeneration, collision simulation or application render was performed by this contribution. The model itself is not bundled here.

## Repository gate limitation

The required command `cargo xtask ci` was attempted in the isolated checkout and could not start:

```text
cargo: command not found
```

The environment lacks the Rust toolchain. The PR therefore remains a **draft** until the relevant repository checks run on the submitted tree in a suitable environment. This is an actual uncompleted gate, not a claim that documentation requires no checks. The root attribution file is embedded by the UI and can expand CI's affected scope.

The package validator and whitespace review do not substitute for repository CI, renderer reference images, native/WASM execution or physics validation. There are no screenshots of new runtime behavior because this PR introduces no runtime behavior.

## Not established by this contribution

- Runtime availability of the 1,160 content concepts or 82 future preset specifications.
- A completed appearance/HDR/terrain asset pack or calibrated source photometry.
- Measured optical, mechanical, electrical, thermal or granular properties.
- A photorealistic SELENE render, a reachable catch or a physically simulated animation.
- Competitive performance, platform parity or a successful full Rust CI run.

These are explicit implementation and validation milestones in the handoff, capability registry and roadmap.
