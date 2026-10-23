# Render and Materials — Upstream Synchronization and Contribution Plan

**Status:** Research only; no synchronization or integration authorized.  
**Assessment date:** 2026-10-22 (recheck all moving references before execution).  
**Repository:** `0xlillith/solidcraft`  
**Target upstream:** `tangency-labs/solidcraft`  
**Purpose:** Preserve the experimental render/materials laboratory while preparing a safe, reviewable route to current upstream and eventual official integration.

## Executive decision

**Do not merge or rebase the entire laboratory onto upstream, and do not submit its aggregate diff as one upstream PR.** Instead:

1. Preserve the laboratory and its provenance/checkpoints.
2. Fast-forward the fork's clean `main` to an explicitly pinned official `main` when authorized.
3. Create a new integration worktree/branch from that official SHA.
4. Inventory and classify each laboratory capability as already upstream, independently proposed in an open PR, original pending work, or experimental.
5. Port coherent vertical slices, preserving authorship and coordinating existing PR owners.
6. Prove persistence, resource integrity, CPU/GPU execution, UI behavior, and CI on the *final* integration SHA.
7. Submit narrowly scoped, issue-linked PRs; verify each landed behavior on upstream after its merge queue and squash.

The existing lab branch is isolated from automatic upstream updates, **not an orphan Git history**. A common ancestor exists. Keep it intact as a recovery and comparison reference.

## Pinned observations (not live promises)

| Reference | Observed revision | Notes |
| --- | --- | --- |
| Official `main` | `3d3dd27439995f37eaed1feaa1c53a0e42d2fe39` | October 22 review target; will move |
| Latest observed official release | `v0.19.0` | Release compatibility baseline |
| Personal fork `main` | `93a2922c` | Previously selected 0.18.2 development baseline |
| `snapshot/upstream-2026-10-21` | `93a2922c` | Historical reference |
| `lab/render-materials` | `5cc4d5fdcc08a4e3cba47b02e19a09e8cffacde6` | Consolidated WIP CPU/GPU/materials checkpoint |
| `docs/render-catalog-selene` | `10ada1b4` | Catalog and SELENE integration concepts |
| `assets/render-starter-pack` | `3ffaa5bd` | PBR/HDR/lighting resource handoff |

Earlier GitHub comparisons reported 11 laboratory-only and 41 upstream-only commits, and 39 overlapping changed paths. These are **not** a measured merge-conflict count. The checkpoint's reported 260 files and 74,299 added lines include other contributors' work; do not claim the whole diff as original. The laboratory checkpoint is WIP, not fully accepted or CI-green. Local Pegasus worktrees may contain uncommitted work and must be inventoried before migration.

- [Laboratory record](https://github.com/0xlillith/solidcraft/blob/5cc4d5fdcc08a4e3cba47b02e19a09e8cffacde6/docs/render-materials-lab.md)
- [Laboratory checkpoint](https://github.com/0xlillith/solidcraft/blob/5cc4d5fdcc08a4e3cba47b02e19a09e8cffacde6/docs/render-materials-checkpoint-2026-10-22.md)
- [Upstream comparison](https://github.com/tangency-labs/solidcraft/compare/93a2922c5ee272fb605703ae84c05825b4a74607...3d3dd27439995f37eaed1feaa1c53a0e42d2fe39)
- [Lab comparison](https://github.com/0xlillith/solidcraft/compare/93a2922c5ee272fb605703ae84c05825b4a74607...5cc4d5fdcc08a4e3cba47b02e19a09e8cffacde6)

## High-risk seams

### Document format and migration

The laboratory introduced pending document descriptors on its older base, reaching experimental format 29, while upstream independently reached official format 29 and subsequently 30. **Equal version numbers do not imply equal schemas.** Never overwrite current upstream `format.rs` with the laboratory copy or merely increment a format constant. Reconcile descriptors through the project's pending-format mechanism and test both JSON and native `.solidcraft` containers. Test older lab documents, current upstream documents, cold reopen, undo, resource retention, and older-reader refusal. Do not claim the current official reader accepts experimental lab fields without actual cross-version tests.

- [Current format and contribution rules](https://github.com/tangency-labs/solidcraft/blob/3d3dd27439995f37eaed1feaa1c53a0e42d2fe39/CONTRIBUTING.md)
- [Current saved render settings](https://github.com/tangency-labs/solidcraft/blob/3d3dd27439995f37eaed1feaa1c53a0e42d2fe39/crates/doc/src/render_settings.rs)

### UI architecture

Upstream 0.19.0 introduced Floating/Compact composition and the universal `ui_kit::Container` policy. Our older UI alignment targeted an earlier redesign. Preserve interaction semantics while rebuilding outer panel composition using the current shared container and controls, without bypassing `cargo xtask ui-containers`. Validate both layouts and themes, window sizes, panel docking, document switches, and keyboard/pointer workflows.

- [Current UI container audit](https://github.com/tangency-labs/solidcraft/blob/3d3dd27439995f37eaed1feaa1c53a0e42d2fe39/docs/ui-container-audit.md)

### Other contributors and stacked work

Coordinate with existing upstream proposals instead of importing their code as a second competing implementation:

| Topic | PR / issue |
| --- | --- |
| CPU renderer and workspace | [#1640](https://github.com/tangency-labs/solidcraft/pull/1640), [#1698](https://github.com/tangency-labs/solidcraft/pull/1698) |
| GPU path executor | [#2067](https://github.com/tangency-labs/solidcraft/pull/2067) |
| Asynchronous GPU setup and completed image panel | [#2780](https://github.com/tangency-labs/solidcraft/pull/2780) |
| GPU studio, glass, clear coat, denoising | [#2191](https://github.com/tangency-labs/solidcraft/pull/2191) |
| Vulkan-only GPU path policy | [#3306](https://github.com/tangency-labs/solidcraft/pull/3306) |
| Physical cards and portable rich libraries | [#1060](https://github.com/tangency-labs/solidcraft/pull/1060), [#1086](https://github.com/tangency-labs/solidcraft/pull/1086) |
| Render preview resource limits | [#4154](https://github.com/tangency-labs/solidcraft/pull/4154) |
| GPU rendering roadmap | [#3151](https://github.com/tangency-labs/solidcraft/issues/3151) |
| Overall Render and material roadmaps | [#620](https://github.com/tangency-labs/solidcraft/issues/620), [#922](https://github.com/tangency-labs/solidcraft/issues/922) |

Recheck PR state, maintainer decisions, branch heads and landing order before implementation. PR descriptions can lag behind GitHub's actual draft/review status. Preserve original contributor ancestry and credit. Do not modify third-party branches.

### Kernel boundary

The laboratory checkpoint's net change did not touch `crates/kernel/` in the reviewed comparison. Continue renderer, scene, IO and UI work without using it as a pretext to change paused Solidcore/kernel ownership. Keep the layered architecture intact.

- [Architecture](https://github.com/tangency-labs/solidcraft/blob/3d3dd27439995f37eaed1feaa1c53a0e42d2fe39/docs/architecture.md)

### Assets and licenses

The starter pack reports 48 surfaces, 240 PNG maps, 24 HDR environments and 24 lighting recipes. These are authored resources, **not** proof of end-to-end renderer integration or native library import. Start with a small accepted vertical slice, then expand. Honor exact hashes, dimensions, sRGB versus linear channels, normal orientation, 16-bit height treatment, environment orientation, millimeter tiling, physical-versus-optical property separation, and cold reopening. Preserve editable masters and exact `ATTRIBUTION.md` entries; no copied proprietary CAD assets or third-party renderer code.

- [Starter pack](https://github.com/0xlillith/solidcraft/blob/3ffaa5bdb3f807eb80503c4222205f60ccc10a03/assets/render/starter/README.md)
- [Asset instructions](https://github.com/tangency-labs/solidcraft/blob/3d3dd27439995f37eaed1feaa1c53a0e42d2fe39/assets/AGENTS.md)

## Proposed execution phases — future work only

**Phase 0 — Audit and freeze.** Inventory every remote branch, local worktree, dirty file, commit, provenance note, fixture, test and asset receipt. Identify already-landed and independently claimed functionality. Pin source SHA and official target SHA. Save recoverable checkpoints before any integration.

**Phase 1 — Update fork base.** Confirm the fork `main` is a strict ancestor of current official `main`; fast-forward it only with explicit authorization. Preserve `lab/render-materials`, the upstream snapshot and independent assets/catalog branches unchanged. Establish a new clean integration worktree from the pinned upstream SHA; run baseline checks first.

**Phase 2 — Reconcile contracts.** Resolve document-format descriptors, scene capture, image job ownership, capability admission, stable resource references, engine commands, library schema and shared UI contracts. Never claim a renderer supports a property merely because a document stores it.

**Phase 3 — Port bounded vertical slices.** Recommended provisional order, subject to upstream landing decisions:

1. Engineering cards, provenance and physical/visual separation.
2. Portable rich material libraries and exact-revision application.
3. Shared CPU/GPU job interfaces, result retention, cancellation and admission.
4. Persisted Render setups and matching Check/Start controls.
5. Embedded HDR and texture ownership with migration and cold-reopen tests.
6. Progressive CPU image jobs and checkpoint behavior.
7. Texture mapping, persistent assignments and renderer consumers.
8. Full current-design Render UI, curated presets and SELENE showcase acceptance.

A slice may cross `doc`, `io`, `engine`, `render`, `gpu` and `ui-egui` if needed to deliver one complete behavior. Avoid splitting a transactional feature into unsafe intermediate states. Parallelize only independent units.

**Phase 4 — Validation.** Execute the current mandatory repository gates and scoped tests. Include the asset pack's separate Python reproducibility/numerical checks, which are not automatically covered by `cargo xtask ci`. Run real native Windows/Vulkan/CPU tests with device and driver recorded, plus software fallback. Capture genuine application and PNG evidence pinned to the exact final head. Test cross-version native files, undo, resource removal, document isolation, output identity, and error/cancellation behavior. Do not conflate compile-only WASM with browser interaction.

**Phase 5 — Official contributions.** Coordinate issue ownership, create one coherent topic per draft PR, keep descriptions and GitHub communications in English, respect clean-room and MIT OR Apache-2.0 obligations, respond to maintainer review, and request the heavy CI gate according to current policy. Do not treat local green tests or a PR label as a merge.

**Phase 6 — Post-merge reconciliation.** Verify the exact upstream landed tree after merge queue and squash, update the inventory to distinguish accepted, superseded and still-pending capabilities, and rebase the next *owned* PR branch as required. Do not rewrite the shared lab branch.

## Acceptance checklist

- [ ] The full source/provenance inventory has no unclassified local or remote work.
- [ ] Official baseline is pinned and verified independently of our changes.
- [ ] Format migration passes for upstream and lab documents, including native container cold reopening.
- [ ] Visual appearance changes do not silently alter mass or physical material assignments.
- [ ] Resource integrity, exact scopes, hashes, limits and lifetimes are tested.
- [ ] CPU/GPU image jobs enforce capability checks, bounded memory, cancellation and document identity.
- [ ] UI works in Floating/Compact modes and relevant themes, with real native evidence.
- [ ] Current `cargo xtask ci`, UI/asset gates, and separately required pack checks pass.
- [ ] GPU backend/device, reference scenes and actual timings are reported precisely.
- [ ] Every proposed official PR has clear ownership, issue coordination and preserved attribution.
- [ ] The final upstream commit is verified after merge, not merely the PR head.

## CI and instruction precedence

The current CI skips ordinary draft PR runs; non-draft unapproved PRs receive quick checks, while `approved-for-ci` unlocks heavy tests. The merge queue and `ci` status have their own acceptance semantics. Inspect the **actual jobs executed** before claiming a green result. Cross-target manual gates and native acceptance remain relevant.

Where old guidance differs, apply current, more specific `AGENTS.md` rules to agent contributions. In particular, rebase owned PR branches when necessary instead of merging `main` into them, but **do not rebase the shared laboratory branch**. Historical `docs/render-integration.md` describes much older document format versions and cannot supersede the current source.

- [Agent instructions](https://github.com/tangency-labs/solidcraft/blob/3d3dd27439995f37eaed1feaa1c53a0e42d2fe39/AGENTS.md)
- [Current CI workflow](https://github.com/tangency-labs/solidcraft/blob/3d3dd27439995f37eaed1feaa1c53a0e42d2fe39/.github/workflows/ci.yml)
- [Historical integration document](https://github.com/tangency-labs/solidcraft/blob/3d3dd27439995f37eaed1feaa1c53a0e42d2fe39/docs/render-integration.md)

## Explicit non-claims and next decision

This document does **not** claim a successful upstream merge, an executable migration, a fully green laboratory CI run, verified cross-version compatibility, or a measured number of Git conflicts. It does not authorize changes to `main`, the shared lab, official PRs or the application. The next decision, when requested, is to authorize **Phase 0 and a disposable integration rehearsal**, then select the official SHA and actual contribution sequence.
