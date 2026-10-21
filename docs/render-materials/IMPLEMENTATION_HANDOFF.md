# Handoff to the render/materials implementation thread

## Scope of this contribution

This PR provides English planning content and data under `docs/render-materials/`, plus exact attribution rows. It changes no application command, shader, renderer backend, native document format, model geometry or existing example. The integration base is the user's `lab/render-materials` branch at the recorded revision. Do not replace it with a newer upstream snapshot as a side effect of integrating this package.

## Immediate implementation tasks

| Order | Task | Acceptance |
|---|---|---|
| 1 | Inventory the actual landed backend and settings support. | Mark each CAP record present, partial or missing with evidence; concept rows remain unavailable. |
| 2 | Define versioned appearance, rig, environment and shot resource contracts. | Units, roles, inheritance, locks, provenance and capability requirements round-trip. |
| 3 | Implement a read-only preset resolver and atomic apply through the engine. | The same resolved request reaches UI, CLI and MCP; unsupported input leaves the scene unchanged. |
| 4 | Bind a small representative material/lighting/environment set. | Authored resources, thumbnails and tests demonstrate the claimed capabilities. |
| 5 | Connect library, contextual inspector and final-job preflight. | A first-time user can apply a scene, change a finish, undo and reproduce output. |
| 6 | Establish raw/finished image and numeric reference scenes. | Saved manifests identify source, executor, resources and color treatment. |
| 7 | Bind quality profiles with explicit budgets and resource admission. | Converged, unconverged, canceled and unsupported outcomes are distinguishable. |
| 8 | Integrate SELENE role bindings and procedural lunar stages. | Native hash, model scale, source ownership and material assignments pass review. |
| 9 | Publish the first validated content pack. | Availability is driven by actual resource and executor support. |
| 10 | Develop animation and dynamics in its own implementation arc. | Analytic fixtures and physical contacts pass before producing the throw/catch demo. |

## Architecture boundaries

Follow the existing layer direction: rendering primitives and capability contracts remain below the document/engine layers; persisted scene intent belongs at the appropriate document boundary; commands orchestrate changes; UI panels display and invoke commands. Keep device/windowing/UI dependencies out of lower layers according to `AGENTS.md`. Use the existing navigation handler for all new render views.

JSON in this package is a specification format, not a requirement to load unvalidated dictionaries inside the Rust application. Design typed Rust data with finite-value checks, caps, version migration, resource hashes and errors. Bound array lengths, resource size, recursion, texture allocation and loop work before execution. Unknown fields, unavailable features and unresolved resources need clear outcomes; do not silently discard them.

Do not use Python as an application dependency. The supplied validator is a development tool. Do not introduce a second settings implementation in the UI or bypass the command registry to apply catalog entries.

## Stable identity and migrations

Treat concept IDs as durable. Resource versions can evolve without renumbering the concept. An authored material is a versioned realization of a concept, potentially with several executor bindings. A new optical interpretation that changes expected behavior requires a new resource revision and revalidation, not an invisible edit to all saved shots.

Preserve the old ID ranges when moving from the previous planning inventory. Use English display names and searchable aliases where helpful. The retained `LUZ` prefix is an identifier compatibility decision, not a request for Spanish UI text.

## Suggested first resource slice

Select exact concept IDs after the material registry is bound, covering: neutral dielectric, coated plastic, clear glass, colored absorbing glass, brushed aluminum, stainless steel, painted steel, sparse-flake paint, rubber, silicone, FDM polymer, PCB solder mask, copper trace, wood with directional grain, a wood coat, LED diffuser, light-guide polymer, solar-cell assembly appearance, thermal blanket and mare/highland regolith.

Pair these with neutral product, dark housing, metal strip, glass bright-field, PCB, wood, LED and spacecraft rigs; use simple physical stages before complex rooms. Publish only the subset supported end to end. Specialized material names must not mask unimplemented optical behavior.

## SELENE-specific dependencies

The still demo needs precise geometry/tessellation, layered surfaces, stable physical mapping, transparent container optics, sampled emitters for LED views, terrain detail, one-Sun ownership, linear color/output and memory-safe quality settings. The animation additionally needs independent vial bodies, explicit closure and grasp state, collision proxies, mass/inertia, contact, joint actuation, granular approximation choices, analytic verification, immutable bakes and shutter-time sampling.

Do not make animation a prerequisite for the still. Do not advertise the animation as physically verified because it follows a parabola visually: release velocity, rotation, contacts, constraints, mass changes and storage also need evidence.

## Publication and validation

Use [roadmap.json](roadmap.json) for the broader F00–F12 and S01–S06 sequence. The current PR's validation report is [VALIDATION.md](VALIDATION.md). Package validation checks the specification and its references; renderer/physics validation remains work for the corresponding implementation.

Before merge, run the repository's required checks in an environment with the required Rust toolchain and dependencies. `ATTRIBUTION.md` is embedded by the UI and can expand the CI scope even though this contribution contains no application behavior change. Do not report a successful `cargo xtask ci` run unless it actually completed on the submitted tree.
