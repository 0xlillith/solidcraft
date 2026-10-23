# Geometric-height recovery checkpoint (not accepted)

This branch preserves the owner's separate uncommitted displacement increment from
base `8c3f72ca2e78f8a54cf7073766688cce4f087b1a` without modifying the original worktree or mixing it into the
completed render/materials donor. It is a recovery/source-availability checkpoint,
not an implementation acceptance, claim, release, or request to merge into main.

The original 13 tracked-file changes and three untracked Rust sources are retained.
The tracked binary/full-index patch SHA-256 is `5ab01a28a382c376bf4d2e9c37bb356d487b6a5de555a70fbfa9c78324155e37`.
New sources: `render-geometric-height.rs`, `displacement.rs`, and
`displacement_resources.rs`. A byte-verified private local archive retains the
original patch, index-state record and per-file receipts; no local paths or machine
logs are included here.

No compile, full-CI, native displacement, conservation, persistence, silhouette or
image-quality acceptance is claimed. Validate and integrate it as a separate
coherent increment before advertising it. The completed donor and current-base
integration are tracked separately in tangency-labs/solidcraft#5494 and #5493.
