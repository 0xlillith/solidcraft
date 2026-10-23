# Render and materials — recovery checkpoint (2026-10-22)

**Status: work in progress, not a release, not approved for an upstream merge.**

## Scope and provenance

- Repository: [0xlillith/solidcraft](https://github.com/0xlillith/solidcraft)
- Development branch: `lab/render-materials`, frozen against upstream base `93a2922c5ee272fb605703ae84c05825b4a74607`.
- Upstream changes are **not** automatically merged or rebased into this branch.
- This is a consolidated recovery checkpoint of local CPU/GPU rendering, the result workspace, numerical Render setups, portable embedded HDR, progressive CPU output, materials, PBR appearances, physical cards, portable catalogs and examples. Details and donor acknowledgements are in [render-materials-lab.md](render-materials-lab.md) and `ATTRIBUTION.md`.
- The expanded catalog and SELENE design specifications in [personal PR #3](https://github.com/0xlillith/solidcraft/pull/3) remain a **separate** review stream. Do not assume that all of its resources have been merged into this branch.

## Validation record

- On 2026-10-22, `cargo fmt --all -- --check` passed on the Pegasus Windows checkout.
- `git diff --cached --check` reported no whitespace errors before this checkpoint.
- Prior sessions reported hands-on CPU result export, progressive 16-spp completion and cancellation, GPU image generation on an AMD Radeon 8060S, HDR source-independent JSON/native-file reopen, physical-card save/undo, catalog v2 round trips and 48 individually applied opaque recipes. **Those observations are historical evidence, not a claim that the final consolidated checkpoint has passed full QA.**
- Previous full `cargo xtask ci` attempts were interrupted and never produced a conclusive passing record for this consolidated source. A successful donor PR or a narrower test is not equivalent to passing this branch's full CI.
- The personal-branch workflow is configured to run `cargo xtask ci` after pushes with bounded workers. Its result must be checked on the exact published SHA.
- Full native UI acceptance and full wasm compatibility validation on this exact published SHA remain required before promoting this work to a review-ready integration.

## Known limits and follow-up

- CPU progressive output is bounded by pixel/sample/memory budgets; high-sample requests may be rejected rather than rendering. The previously observed 64-spp scene hit that limit while 16 spp completed.
- Embedded HDR design resources are implemented; runtime body bindings, external image maps, and some optical assignments are not yet document-portable.
- Broad diffuse global illumination, adaptive CPU sampling, unrestricted glass, and EXR output are not complete features.
- Validate icon-manifest line endings on both Linux and Windows. This checkpoint adds an LF policy for the inventory.
- Preserve physical-material and visual-appearance separation. Existing design revisions, provenance, density and mass must not change when applying a visual look.
- Source/asset licensing and attribution require a final review before any upstream contribution.
- This checkpoint is for recovery and subsequent validation. Do **not** tag a release or merge upstream on this evidence alone.

## Next handoff

1. Confirm the pushed commit SHA and the remote branch contents.
2. Inspect GitHub Actions for that SHA; resolve any failing gates without concealing them.
3. Build and run the exact commit on Pegasus, including CPU progressive cancel/export, GPU adapter proof, HDR portability and material-editor undo/reopen.
4. Review [personal issue #1](https://github.com/0xlillith/solidcraft/issues/1) and [PR #3](https://github.com/0xlillith/solidcraft/pull/3) for remaining catalog and SELENE work.
5. Break future work into reviewed, independently testable arcs before considering upstream integration.
