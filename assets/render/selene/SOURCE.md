# SELENE improved showcase source

**The selected baseline is `SELENE-showcase.solidcraft`, the improved mechanism demonstration described by SychPL in Tangency Labs' SolidCraft project. Its native source has not been located in the public material inspected.** The active source is therefore `awaiting_native_source`.

[source-lock.json](source-lock.json) is the authority for this selection and its admission state. The package does not substitute the original `Rover-v7.solidcraft`, reconstruct an assumed derivative, or treat a recording as an inspected native assembly. Source-dependent scene and motion compilation must remain blocked until the actual improved source is admitted.

## Why this variant is selected

The author's [showcase comment on PR 1540](https://github.com/tangency-labs/solidcraft/pull/1540#issuecomment-6205857321) describes additions to the original rover: anchored modules, solar-actuator loops, a six-wheel motion chain, a rack-driven demonstration track, a turret/dish gear coupling, a vial-cap screw, arm inverse kinematics, motion traces and a coordinated study. [PR 1680](https://github.com/tangency-labs/solidcraft/pull/1680) explicitly names `SELENE-showcase.solidcraft` as its input file.

This establishes the intended variant and the published behavior to investigate. It does not provide the file bytes, native IDs, full assembly inventory or exact saved pose. The author says that joints and helper parts were added; the derivative must therefore be inspected as its own artifact.

The [pinned mechanism reference](https://github.com/tangency-labs/solidcraft/blob/463cea0615a22be2f7d6c30512211f9eda97ff28/skills/solidcraft-mechanisms/reference.md) documents the actuator setup and several earlier SELENE experiments. Its descriptions are useful acceptance evidence. Counts mentioned in different examples must not be combined into an invented inventory of the later showcase.

## Authoritative evidence

| Evidence | Pinned identity or discussion | What it establishes |
|---|---|---|
| Mechanism reference | [`463cea0615a22be2f7d6c30512211f9eda97ff28`](https://github.com/tangency-labs/solidcraft/blob/463cea0615a22be2f7d6c30512211f9eda97ff28/skills/solidcraft-mechanisms/reference.md) | Actuator splitting, closed loops, mirrored tilt, wheel links, anchoring and visibility treatment described by the project. |
| Author's improved-showcase comment | [PR 1540, comment 6205857321](https://github.com/tangency-labs/solidcraft/pull/1540#issuecomment-6205857321) | The particular combined demonstration and its added mechanisms. Discussion text is mutable; it was inspected on 2026-10-21. |
| Inverse kinematics change | [PR 1540](https://github.com/tangency-labs/solidcraft/pull/1540), merged as `4d862cd76ccfa3a171c66c48d9a184209a33a945` | Published reach and jointed-drag behavior. The change explicitly excludes contact-set checking from `joint.reach`. |
| Motion export change | [PR 1680](https://github.com/tangency-labs/solidcraft/pull/1680), merged as `2ae21143ee2cb8ffb3af916ee2ffadac956a9faa` | Exact input filename, named tour and headless export examples. |
| Motion analysis change | [PR 1712](https://github.com/tangency-labs/solidcraft/pull/1712), merged as `37496198bf2c99285de703899a67a5f87ac60b7a` | Plots and sampled arm-versus-wing interference demonstrations, with stated sampling limitations. |
| Combined-showcase recordings | [SychPL media tree `df2e74230e6659409d65501b5d9396b55c6503d3`](https://github.com/SychPL/solidcraft/tree/df2e74230e6659409d65501b5d9396b55c6503d3) | Three linked GIF artifacts; no native model in this tree. |
| Motion-export recordings | [SychPL media tree `1e2ad6842a5ad5c3b2cedee3e6508a8e4e3eb3d4`](https://github.com/SychPL/solidcraft/tree/1e2ad6842a5ad5c3b2cedee3e6508a8e4e3eb3d4) | Two GIFs and a dialog image; no native model in this tree. |

The JSON references preserve the original author, media filenames, upstream merge revisions, source URLs and evidence levels. A software merge revision identifies a code change; it is not a native-model revision or model hash.

## What was searched

The read-only public search was performed on **2026-10-21**. Its exact scope is recorded under `public_search` in the source lock.

| Location | Scope | Result |
|---|---|---|
| `tangency-labs/solidcraft` | Complete observed main tree, 2,583 entries with no truncation; relevant documentation; all 42 returned releases and their asset inventories. | No improved native SELENE model in those locations. |
| `tangency-labs/models` | Complete tree at `57abdd6f71c4202fcda78d19dcfdb54dde80a7a9`, README, license and four-commit setup history. | No models at all: documentation, templates, checks and CI only. |
| `0xlillith/solidcraft-rover` | README, attribution and complete branch inventory, with one `main` branch at `b32437915372d32b71419dc8f5143fe22ff1e869`. | The original v7 and construction-recording variants remain the documented downloads. |
| `SychPL/solidcraft` | Complete inventory of 109 branch names; four relevant source trees; one release's asset inventory. | No improved native model in the examined source trees or release. This does not claim that every branch tree was inspected. |
| Six SychPL media branches | `media/joint-reach`, `media/motion-analysis`, `media/motion-animation`, `media/motion-traces`, `media/selene-demo`, `media/selene-showcase`; complete trees and histories. | Thirteen GIF/PNG files. Each branch is one root commit with no parent history; none contains a native file. |
| PRs 1540, 1680 and 1712 | Combined issue comments, inline review comments and review submissions, plus separate review-thread queries. | Respectively 3, 1 and 0 discussion records; no review threads and no native or ZIP attachments. |

This bounded search did not locate the file. It does not establish that no copy exists elsewhere, and no guessed repository path or download URL has been inserted.

## Source-lock contract

`active_source` deliberately contains:

- The selected variant and expected filename.
- `status: "awaiting_native_source"`.
- `null` repository, commit, path, download URL, SHA-256, byte length and format.
- Required millimeter units and Z-up orientation. These are admission requirements, not observations of the missing source.

`admission_review` keeps identity confirmation and license review as separate false flags. Reviewer, review date and evidence URL remain null. `license_review` holds the detailed license expression, notices and supporting records once the actual artifact is reviewed.

A future admission must identify the actual file through provenance-bearing HTTPS evidence, verify its bytes, inspect native assembly references and complete the license review. Only an explicit reviewed transition may set `active_source.status` to `pinned_native`. The local preparation tool must not mark either review flag true automatically.

Matching geometry, a plausible filename, extra joints or familiar feature counts cannot admit a source. The native review must resolve the actual components, occurrences, joint/link graph, limits, grounded roots, study keys and helper geometry. Ambiguous bindings block preparation without changing a scene.

A pinned file still has no runtime validation by itself. Replay, collision checking, dynamic simulation and rendering require their own recorded evidence. A source hash change invalidates source-dependent bindings and motion evidence.

## Legacy comparison and licenses

The original `Rover-v7.solidcraft` remains only a historical comparison in `legacy_sources`. Its known SHA-256 is:

```text
fcb08540b1db08325f366fd0ff235f84f66d96406961482736bc097d7248c6d6
```

That hash is explicitly ineligible for the active source, including if the file is renamed. Its 17 revolute joints and original component IDs are not bindings for the improved variant.

The [original rover attribution](https://github.com/0xlillith/solidcraft-rover/blob/b32437915372d32b71419dc8f5143fe22ff1e869/ATTRIBUTION.md) declares MIT OR Apache-2.0 for the original model and records Ubuntu Light lettering under the Ubuntu Font Licence 1.0. The actual improved native file and any added helper geometry still need a file-specific inventory and review.

Tangency's [existing rover recording attribution](https://github.com/tangency-labs/solidcraft/blob/463cea0615a22be2f7d6c30512211f9eda97ff28/ATTRIBUTION.md) concerns the construction replay of `Rover-recorded.solidcraft`. Those rows do not identify this improved native file. The separate [Tangency benchmark-model collection](https://github.com/tangency-labs/models/blob/57abdd6f71c4202fcda78d19dcfdb54dde80a7a9/README.md) uses CC0; that does not change SELENE's licenses.

These source and mechanism manifests are original SolidCraft metadata and prose under MIT OR Apache-2.0. No third-party model, screenshot, GIF, extracted commercial asset or GPL implementation is bundled.

## Mechanism review

[motion/mechanism-acceptance.json](motion/mechanism-acceptance.json) contains twelve review areas with independent documentation, native-inspection and runtime statuses. [The motion guide](motion/README.md) explains their use and four original analytic planning checks.

The mechanism demonstration is kinematic. Lunar gravity, actuator effort, traction, flexible cables, granular contact, sample release and capture remain separate simulation work.
