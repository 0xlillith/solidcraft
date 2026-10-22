# Native Render studio UI

Integration-branch source, continuing the frozen personal renderer fork. Actual execution and visual-acceptance results belong in `render-materials/UI-BATCH-2026-10-22.md`; this guide is not itself proof of acceptance.

## Workspace and navigation

RENDER remains one ribbon tab. The existing Browser card has a local Model / Library switch in both Docked and Floating layouts. The native model tree, visibility and selection are not replaced by a competing scene tree. The central canvas retains the original product-navigation and completed-image tools.

The right inspector has eight local sections: Scene, Appearance, Lighting, Environment, Camera, Output, Diagnostics and All controls. These are sections of one saved render draft, not rendering modes. All controls is the same dense native editor assembled from shared controls, not a separate simplified/advanced execution path. Each open document retains its own page and library state.

Check, Render, Cancel and Export remain outside the scrolling inspector body. They use the existing immutable job protocol. A later draft edit does not substitute a different image for a retained completed export. PNG export copies the completed native frame, not the reduced UI display texture. Check and Start use the same request builder; inspection does not silently start a renderer or open a GPU.

## Resource library

The native library includes original surface maps, SELENE appearances, HDR environments, authored rigs, physical-reference profiles, advanced scene/material/light/environment concepts, quality specifications, capability requirements and the twelve authored SELENE shot specifications. Four separate `NATIVE-STUDIO-*` entries are executable compositions; they do not relabel the unbound `SCENE-*` concepts as finished.

Category-specific search, page and selection survive switching categories. Favorites are bounded at 256 entries and recent inspection at 32; these are per-open-document session state, not an on-disk shared library. Search, availability filters and collection inspection are read-only. Physical profiles remain engineering references, never optical substitutes. Resource details retain original values, omissions, source locators and provenance.

An optional source preview decodes the bounded, hash-verified original base-color PNG. It is labelled **source map**, not a measured BRDF swatch or a path-traced thumbnail. Every surfaced projection remains explicit: uniform optical appearance, planar authored maps, embedded HDR, complete rig or acknowledged emitter-only variant. Physical texture scale and XY / YZ / ZX mapping feed the native validator. No height displacement is implied by a normal map.

## Native scene composition

`render.preset.resolve` accepts a native studio identity, saved setup name, optional current typed GPU setup patch and optional expected revision. It resolves original lighting/HDR/passive ground and validates conservative clearance without changing the document. Its response identifies replacements (lighting, environment, stage and exposure), preserved fields and limitations.

`render.preset.apply` requires the reviewed revision, explicit lighting/environment/exposure consent, and stage-replacement consent when needed. It reconstructs and validates the whole candidate before publishing one undoable render-settings change. Stale reviews, empty subjects, unsupported methods, existing-stage consent bypasses and unbound SCENE identities refuse atomically. Camera, dimensions, sampling, optical bindings, CAD geometry, engineering materials, pose and visibility are preserved. The three-light composition's 1/16 radiance multiplier is a creative variant, not the original measured/calibrated reference.

The scene inspector also fits a finite render-only ground with explicit margin, gap and linear reflectance. Passive panels do not create engineering mass. Conservative AABB separation is not exact B-rep collision or physical contact certification.

## Shot lifecycle and unsaved edits

Saved shots can be duplicated, renamed and removed through `render.shot.duplicate`, `render.shot.rename` and `render.shot.remove`. Each uses an expected revision, validates names and rejects destination conflicts. Resources and captured jobs are retained; no file deletion or camera movement is implied. Mutations use one document undo.

Changing or reloading a shot with an edited draft opens Save and continue / Discard draft and continue / Keep editing. Removal has a separate confirmation. A review cannot be reused after the document revision or identity changes. A failed save preserves the draft rather than continuing with lost data.

## Optical, light and camera controls

The Appearance section provides an explicitly applied opaque body editor and the existing exact-target glass, clear-coat and four-channel map controls. Body color is sRGB; roughness, metallic fraction and optical IOR are not engineering constants. The reviewed target is revision/document checked. Existing face-specific appearances and physical material assignments remain separate. Temporary optical overrides retain their existing portability limitations and cannot masquerade as saved intent.

GPU illumination exposes analytic area/sun lighting and the shared environment controls. CPU controls expose directional and point/spot lights plus bounded rectangular emitters. CPU rectangle vectors are **full sides**, while GPU vectors are **half edges**; labels and native request schemas preserve the distinction. Sky zenith/horizon/ground are linear RGB. Display background is not an extra light source.

The Camera section edits the shared native camera via public commands, with millimeter position/distance, degree-displayed angles and explicit perspective FOV. Thin-lens controls remain GPU-specific. Navigation, camera fit and restore are not reimplemented as private mouse gestures. Unedited numeric values do not change merely because an editor is shown.

## Output and capability honesty

Output dimensions follow the selected CPU/GPU/raster envelope. Named size buttons are explicit edits; invalid combinations stay visible for correction rather than being silently reduced. GPU fixed samples, path depth, seed, tone mapping and denoising preserve the existing numerical contract. CPU resource and sampling profiles remain explicit. Completed PNG supports the existing RGBA8/sRGB path; EXR and raw-linear master export are not advertised as executable formats.

`render.quality.prepare` resolves an acknowledged `fixed_samples_png` projection of a Q specification. It returns only compatible dimensions, fixed sample count, supported maximum depth, seed and denoise intent. It lists adaptive stopping/error targets, independently authored transmission-depth/Russian-roulette rules, raw-linear output, EXR and convergence guarantees as omitted. It refuses excessive profiles rather than clamping them. The UI changes only those budget fields, preserving executor, camera, lighting, optics, exposure and stage.

Diagnostics exposes the exact current request, last preflight, logical-memory estimate, captured-job metadata, scene counts and structured errors. Stale checks are identified. Adaptive convergence, further transmission estimation work, unrestricted nested media, displacement/volumes/spectral transport and production queues remain explicitly unavailable. Seeing a concept in the library is not runtime admission.

## Verification

Focused regression sources cover atomic native composition, saved-shot conflicts and undo, quality projection refusals, original source-preview identity, per-category navigation, bounded recent inspection, context-page nonmutation, sticky job actions in both layouts/themes, draft confirmation, stale reviews and real UI composition application. Existing dense-inspector interaction tests explicitly select the public All controls section and retain their numerical assertions. They exercise the same production request/actions as the contextual inspector.

Actual application screenshots must be captured from an isolated native application session using a public-safe model fixture, in both themes/layouts, after the coordinated batch build. Review images/logs belong outside production assets. The existing native SELENE master and selected 4K image remain unchanged by this UI work.


### Isolated native review profile

The desktop application accepts `SOLIDCRAFT_SETTINGS_DIR` as an explicit absolute directory for its settings, graphics preference and startup marker. Invalid/relative/file paths refuse before startup; an explicit profile never adopts the ordinary or legacy settings. Without the variable, normal behavior is unchanged. This is profile isolation, not a filesystem sandbox. Set the existing recovery/crash/control directory overrides separately when running a disposable native review. It allows the actual hardware application to be exercised without rewriting personal preferences or substituting safe-mode/software behavior.
