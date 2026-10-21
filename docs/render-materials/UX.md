# Render workspace UX plan

## Product objective

The common path is: **select a subject, choose a scene recipe, adjust the visible result, render**. A designer should obtain a useful presentation without knowing shader graph terminology. An expert should be able to inspect every physical assumption, replace any resource and automate the same operation.

The workspace uses SolidCraft's existing theme tokens and camera navigation. Its panels read engine state; every state-changing operation goes through the shared command registry. No new visual design system, icon family or independent camera controller is introduced by this plan.

## Layout and entry points

| Area | Default content | Deeper controls |
|---|---|---|
| Top workspace bar | Subject, saved shot, Preview/Final, Undo, Render | Executor capabilities, job queue, batch variants |
| Left library | Search, Recent, Favorites, Materials, Lighting, Environments, Scenes | Domain, surface family, physical scale, capability, provenance and memory filters |
| Center viewport | Progressive preview, selection outline, light handles when selected | Region render, raw/finished A/B, false color, normal and noise diagnostics |
| Right inspector | Contextual controls for the selected material, rig, environment or camera | Full physical parameters and resource provenance in expandable sections |
| Bottom status | Preparation phase, samples, elapsed time, quality status, Cancel | Memory, dependency resolution, warnings, saved render manifest |

The library is a browser, not a wall of 1,160 equally prominent tiles. Start with recommended packs for Product, Automotive, Electronics, Making, Aerospace and SELENE. Search must also work by material ID, process, intended use and finish. Preserve filters when moving between a category and an item's inspector.

Show material thumbnails under one declared neutral calibration rig. Offer a second preview under the current scene. Consistent calibration makes roughness and color easier to compare; scene preview reveals whether the material suits the shot. A resource with an unavailable capability shows that fact before application.

## Four primary workflows

### 1. Fast product presentation

1. Select a body, component or assembly and open Render.
2. Choose a scene card such as **Electronics Bench Hero** or **Automotive Paint Review**. The card includes thumbnail, intended subject size, quality tier and availability.
3. Confirm the automatically resolved subject and ground contact. The scene applies as one undoable action, preserving locked camera and appearance assignments.
4. Adjust no more than six primary controls: finish, roughness, texture size, light rotation, exposure and camera framing. Render after preflight succeeds.

The future target is three to five deliberate actions from a selected subject to a sensible default image. This is a usability target to measure with first-time users, not a claim about the current implementation.

### 2. Precise appearance assignment

Select face, body, component definition or occurrence; show the scope in plain text before applying a finish. A shared-definition edit indicates how many occurrences it affects. Dragging a material highlights the prospective target without changing it. Drop applies one command; Escape cancels. Multi-selection offers a shared override with explicit mixed-value fields.

The first inspector section contains base finish, color, roughness, physical texture size and direction. Additional sections expose coat, conductor/dielectric behavior, transmission, absorption length, subsurface response, fibers, flakes, layers and mapping only where relevant. Sliders have units, direct numeric input, reset and useful bounds. Do not rename optical roughness to an ambiguous “quality” setting.

A separate Engineering panel retains density and design properties. A later Dynamics panel owns contact and inertial assumptions. Applying “dusty anodized aluminum” must not alter assembly mass, friction or restitution.

### 3. SELENE lunar still

Choose a pinned SELENE scene card, resolve the model binding and select a shot. The primary controls are Sun azimuth, Sun elevation, exposure, camera pose, terrain seed and dust coverage. A clear **Physical lunar** mode locks atmospheric scattering and studio fill off. Entering an editorial mode is an explicit, recorded change.

The default lunar sky is dark and the Sun is owned once. An Earth backdrop remains disabled until location, time, orientation and visibility are resolved. “Improve shadows” must never silently inject skylight. Provide a diagnostic view explaining that unlit areas receive only the contributions actually present in the scene.

### 4. Future simulation and final animation

Keep motion planning and physics approval separate from render quality. The timeline displays the state sequence, contacts and events; the simulation inspector reports mass, inertia, limits, constraint state and solver diagnostics. The user approves a baked trajectory before a final render queue starts. Changing friction, gravity, object topology or motion parameters invalidates the bake and displays the affected frames.

## Progressive disclosure

| Level | What the user sees | What remains accessible |
|---|---|---|
| Quick | Recommended scene cards and a small set of useful controls | A single “More controls” expansion; no separate document format |
| Detailed | Material layers, light contributions, ground, camera and color sections | Exact numbers, resource links and clear reset/inheritance behavior |
| Expert | Integrator, sampling, limits, spectral assumptions, passes, resource and execution details | Reproducible request export and the same command path as automation |

These are views of one scene, not three incompatible modes. Moving to Expert does not reset Quick settings. Defaults come from the resource and can be restored at field, section or whole-shot level. Changed fields visibly distinguish inherited values from overrides.

## Scene composition and conflict resolution

Resolve a scene recipe before mutation. The preview of changes lists new resources, required capabilities, preserved locks, replacement scopes and unresolved references. Apply succeeds atomically or leaves the shot unchanged. Unsupported dependencies are named with a useful next action: select a supported preset, choose a suitable executor, or stay in a clearly identified approximation.

Never silently replace path tracing with raster preview, convert a transmitting lens to opaque plastic, remove a coating, add a second Sun or change the model scale. An optional preview approximation is saved as such and cannot inherit the “validated final” badge.

Preset ownership has four levels: resource defaults, scene composition, shot overrides and explicitly locked user fields. The resolved request records all four. Switching a lighting preset should not unexpectedly replace the camera, engineering properties or unrelated material assignments.

## Interaction details that matter

- Hover previews are temporary and cancelable; committing a recipe is a separate action.
- Direct light handles display the edited physical property and unit; selecting a Sun edits direction, not a nearby point-light distance.
- A/B comparison locks camera, exposure and display transform by default so the comparison measures the intended change.
- Search, lists, sliders, selection scope and render controls are keyboard accessible. Focus returns to the initiating control after dialogs.
- Status never relies on color alone. Use readable labels such as Concept, Ready, Preview approximation, Unsupported and Needs revalidation.
- Long operations expose progress phases and cancellation. A failed export preserves the completed image buffer where possible and offers a different destination.
- Autosave persists scene intent and references; cache eviction must not destroy a material definition or the last successful render.
- Offline reopening reports missing dependencies before rendering and identifies the exact pack and version.
- Library packs load on demand. Thumbnail generation and search indexing must not freeze geometry interaction.

## Proposed command surfaces

The command IDs below are design proposals, not registered commands. Confirm naming and ownership with the implementation thread before adding them:

| Proposed action | Required behavior |
|---|---|
| `render.preset.resolve` | Read-only dependency, unit, lock and capability resolution. |
| `render.preset.apply` | Atomic application through the engine with one undo record. |
| `render.appearance.assign` | Explicit face/body/definition/occurrence scope. |
| `render.shot.duplicate` | Preserve references while creating independently editable overrides. |
| `render.request.validate` | Produce the exact immutable request later used to start the job. |
| `render.job.start` | Reject stale or unsupported requests without partially changing the document. |
| `render.job.cancel` | Release owned job resources and preserve the design. |

Existing `render.settings.set` is the current-contract example in this package. Do not send future recipe fields to it or register parallel UI-only actions.

## Acceptance study

Use a small range of CAD subjects: machined bracket, painted enclosure, PCB, FDM part, wooden jig, LED optic and SELENE. Observe first-time and expert users completing the same three tasks: obtain a default image, change a specific finish without affecting sibling instances, and reopen/export an identical shot.

Record task completion, intervention count, accidental scope changes, time to a usable result, misunderstanding of availability, and whether people can recover with Undo. Test keyboard-only operation and mixed scale assemblies. Treat the “few clicks” objective as unmet if the image looks attractive only after undocumented expert repair.

Detailed acceptance requirements are in [ux.json](ux.json).
