# CPU indirect lighting for opaque surfaces

The CPU-feature renderer build and numerical regressions have executed on the
combined source. Current combined document, engine, UI and native workflow
qualification remains separate and pending; this is not full-project or release
acceptance.

CPU Sampling can explicitly enable **Indirect lighting (opaque surfaces)**. Runtime intent is `raytrace.path: {"max_depth": 8}`; saved CPU intent is `job.options.path` with the same contract. Omission preserves the existing direct/selected-reflection algorithm, pixels and work. It adds no path allocation when omitted; the fixed task metadata remains within the existing allowance, subject to compilation guards. Path depth is 1–16 shaded surfaces, starting with the first visible surface. A depth of 1 uses direct lighting only; higher values allow more light bounces and may take longer. Finite depth truncates longer paths; this is not a certified convergence or speed guarantee. No automatic quality, profile, size, seed, camera, default or method change is made.

The mode requires progressive camera sampling, primary_samples1 and traced shadows. It evaluates the complete existing isotropic opaque BRDF, including its existing clear-coat approximation, and uses an exactly matched cosine/GGX-NDF/coat mixture. Null proposals retain their probability mass. Secondary hits preserve captured body/face appearance and existing surface effects; visible surface emission can illuminate diffuse neighbors through actual path continuation. Small mesh emitters have no next-event sampling and may remain noisy.

Directional and point/spot sources retain their delta direct estimates. Rectangles become intersectable one-sided emitters. Rectangular and HDR/constant environment estimates use light/BSDF power weights with the actual direct sample counts, previous-scatter density/position and no nonexistent continuation competitor at the depth endpoint. Geometry wins equal-distance emitter ties. Finite shadow intervals exclude their sampled source; other rectangles occlude both strategies consistently. Uniform-solid-angle HDR sampling preserves the existing explicit sRGB/D65 policy; no importance-distribution quality claim is made.

A primary miss remains the separate unexposed display background. Complete paths add scene-linear radiance to the existing primary film exactly once. Existing every-pass publication, presentation, raw final EXR, final ownership and exact export routes remain authoritative. A partial path cannot advance film coverage/SPP or publish a completed result. Guided noise reduction and transparent PNG coverage compose with this opaque
transport without entering raw EXR radiance or changing primary coverage.
Composition regressions check an analytic partial-coverage emitter, every
checkpoint, worker quanta and unchanged raw masters; these renderer regressions have executed.

This first opaque producer refuses selected reflection, certified anisotropy and
dielectric overrides rather than silently ignoring authored values. Stable saved
optical bindings also refuse at pure setup validation before HDR resource reads
or scene capture; temporary overrides refuse at runtime option validation.
Removing path lighting explicitly restores those existing optical routes. General glass, layered/nested media, volume, subsurface transport, emissive-mesh next-event sampling, adaptive CPU sampling and multithreading remain separate work. The CPU Sampling checkbox keeps fixed/saved optical choices intact; adopt progressive sampling explicitly first when required. Disabling path lighting preserves the existing settings and returns to their original transport.

Memory is one exact-capacity fallible path continuation, prepaid in CPU job admission. There is no second film, ray queue, worker, host pool or raised cap. Sampler/source/emitter scans, traversal, shading and the final film commit are separate bounded work continuations sharing the original cumulative budget and cancel token. The unchanged fixed host metadata/task-state bounds remain integration gates.

Executed renderer regressions cover analytic Lambert density/furnace, full coat PDF consistency, projected-solid-angle rectangle means, real diffuse red bounce and blocker, every-prefix/quanta/raw-master invariants, exact admission delta and cancellation/work exhaustion. Offline contracts, actual Sampling click/Undo/reopen and saved/runtime capture equality still need their combined document, engine and UI qualification; native visual acceptance remains pending.


Rectangle support uses the full admitted two-vector basis, including the common
near-orthogonal tolerance and extreme aspect ratios. Geometry wins model/emitter
equal-distance ties in both continuation and direct visibility. Positive-area
overlapping, same-facing coplanar rectangle supports refuse before capture;
the bounded projected-bounds guard is conservative for rotated supports and
retains all authored sources. Adjacent supports and opposite one-sided emitters
remain valid. Stratified HDR/area source inputs alone are made representably
open at floating-point endpoints; the camera counter stream is unchanged.

The CPU denoising prepass tests each admitted analytic rectangle only when this
path mode is enabled. A strictly closer, front-facing emitter produces a bypass
guide instead of borrowing the diffuse body behind it; geometry keeps exact
distance ties. Each of at most four source scans has its own cumulative work
debit and cancellation check. Its scalar continuation state is prepaid in the
existing exact guide-state size, with no additional image plane. The absent-path
guide algorithm performs no new source scans. Combined numerical and lifecycle
renderer tests for these invariants have executed; native image-quality acceptance remains pending.
