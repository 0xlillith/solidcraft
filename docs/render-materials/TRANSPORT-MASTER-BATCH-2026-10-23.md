# Transport and retained-master continuation — 2026-10-23

This is an implementation checkpoint on the personal `lab/render-scale-resources` branch, not a release or a claim that the entire rendering roadmap is complete. Production source and acceptance evidence must be distinguished. The full GPU and freshly linked application acceptance for this batch is pending at this checkpoint.

## Remote gate diagnosis

The resource gate on `3727ae9e` failed in the CPU-enabled renderer tests on both Ubuntu and Windows. The two failing tests asserted an empty retained-memory pool after completion notification but before the native worker had destroyed its temporary job handle. Local serial execution had hidden that scheduling dependency.

Commit `74f3d615` synchronizes the test-owned worker before exact lease assertions. It changes test synchronization only: production reservations are not released early, assertions are not weakened, and locks or permission checks are not bypassed. Both strict renderer feature configurations and the CPU suite with 16 test threads passed locally.

GitHub did not start a replacement run. Manual dispatch returned HTTP 422, `Actions has been disabled for this repository`, while the repository permission endpoint returned `enabled: true` and the workflow itself was active. The reason for that service-side discrepancy is not established. No Actions, permissions, branch-protection or account configuration was changed. The last failed run is retained; a local pass is not represented as a remote CI pass.

## Completed full-model acceptance of the preceding native application

The native application built from `106f5d57`, SHA-256 `e09ea35eb164355782c34335129f4be057ecd1a669b22a047246b0d341814211`, opened an isolated copy of the exact SELENE functional master. Source SHA-256: `3bd2bfa1c6336394c4a4208d30b52c80d06658a3bd7ea57cc93b774411d1bd68`.

The model retained 2,116 evaluated bodies, 2,094 originally visible bodies and 2,144,099 visible triangles, without timeline errors or warnings. A complete-geometry 480 × 300, 32-sample GPU pair used the reviewed surface and dielectric assignments. Bloom changed PNG presentation without changing the raw EXR digest: `68c1d403b745c6e9d1c31b5b0bb43e887dfcf9eb669d5115c2b346d73272c4fc`.

Native light/docked and dark/floating captures were visually reviewed. The process quit normally with exit code 0, without forced termination; the original model and personal preferences remained unchanged. This is full-geometry integration acceptance at low output resolution, not a new 4K production/convergence benchmark, and not acceptance of the new transport code below.

## Independent-pilot adaptive allocation

The optional GPU `adaptive` object contains `pilot_samples`, `min_samples`, `relative_error`, `absolute_error` and `coverage_error`. The ordinary `samples` value is the production ceiling, not a certified noise bound.

Each pixel first evaluates an independent pilot stream. RGB and coverage moments estimate a bounded production allocation. Pilot radiance is discarded. The original production stream then supplies a fixed number of samples conditional on that pilot allocation; pilot values never become an adaptively stopped final mean. Final per-pixel means feed the same raw EXR, display transform and optional surface denoiser.

Bounds are 16–256 pilots, at least 16 production samples and the existing ceiling of 4,096 production samples. RGB and coverage are considered separately. Nonfinite or overflowing estimates choose the cap, and invalid path radiance fails rather than being hidden. Rare paths can still be missed by a finite pilot, and capped pixels may remain noisy. Dispatch compaction and a global certified stopping rule are not implemented.

Two auxiliary float4 arrays add an explicitly admitted 32 bytes per pixel. Completed job metadata reports minimum/maximum production counts, capped pixels, total production samples and actual primary-path evaluations. This does not count every secondary or shadow ray and is not a timing guarantee.

## Conditional Fresnel stratification

`transmission_samples` accepts 1, 2 or 4. One retains the original stream. Additional strata share the primary and prefix path, stratify the first glass reflection/transmission decision, then decorrelate later random dimensions. Snell refraction, Fresnel response, absorption and geometric medium certification are not replaced by clamping or denoising.

Each invocation traces one bounded subpath. An initial nested-path-loop implementation caused excessive native shader compilation and its review was stopped before any numerical acceptance; that attempt is not a passing test. Strata now advance through host dispatch rather than wrapping the whole integrator in another shader loop. Extra strata cost extra work, and opaque prefixes can be repeated. Quantitative acceptance compares an absorbing-sheet fixture at equal path budget, not merely equal sample labels. No universal speed or noise-free glass claim is made.

## Bounded homogeneous participating medium

The optional GPU `volume` object describes one finite world-millimetre box, scalar extinction per millimetre, RGB scattering albedo and an HG mean cosine. Analog free-flight events occur on camera and indirect paths. Real scattering uses area, environment and sun sources; straight lighting connections include Beer attenuation. The corresponding no-event density is included in the path/light MIS comparison. Volume scattering changes raw transport; it is not a camera-overlay fog filter.

Zero extinction preserves prior transport and consumes no new random variates. Bounds, mean-free-path precision, extinction, albedo and anisotropy are validated before execution. Volume transport requires explicit studio or environment lighting. Glass boundaries, multiple Fresnel strata and the surface-guided denoiser are explicitly refused with an active medium. The UI does not silently clear those choices.

This is not heterogeneous density, nested glass/volume transport, emission, spectral scattering, surface subsurface scattering, a measured optical material, or a physical lunar atmosphere. The existing finite path-depth budget remains authoritative. Physical-vacuum lunar scenes must keep the medium off.

## Nondestructive retained-master presentation

`render.result.regrade` accepts an exact completed job identity and an explicit `presentation` object. It creates a distinct, cancellable job without capturing geometry, invoking a transport executor or mutating the design. Its fixed dimensions and camera belong to the captured master, not a later scene edit.

The decoder accepts only the exact retained native uncompressed FLOAT32 RGBA linear-sRGB/D65 profile. Header, dimensions, offsets, row lengths and sample values are validated. Arbitrary external EXR input is not supported. A PNG without a retained raw companion cannot be reconstructed into a master.

Exposure, tone mapping, RGB gains, contrast, saturation, vignette, camera backdrop and optional display bloom use the shared presentation implementation. The source EXR remains byte-identical and is shared through its original frame lease. Repeated regrading points directly to the original master instead of creating a retention chain. Provenance records the immediate source job, original master job, master digest and exact presentation settings.

A separate metadata lease survives as long as either the derivative job or its frame retains that provenance. Working pixels, bloom buffers and PNG encoding are bounded and admitted. Cancellation cannot publish a partial derivative or destroy the source. Denoising is not reapplied: the raw master does not retain its geometric guides. The shared ribbon and Output inspector expose the same command and use existing completed-frame export protections.

## Resource coverage and clean-room boundary

The native studio set is expanded from 8 to 25 authored compositions, covering all 24 existing original HDR environments while preserving the previous eight identities. Automotive, paint-band review, CNC tooling, service bays, makerspaces, electronics rework, FDM inspection, printed-part photography and aerospace inspection have explicit native compositions. Each keeps camera, geometry and optical assignments intact and declares what it does not synthesize. Complete resolve/apply/undo/offline-reopen and runtime acceptance are required for every new entry; source availability alone is not that evidence.

These producers do not make every advanced material or scene concept executable. Existing authored resources and explicitly supported projections remain separate from unbound specifications. Density grids, nested optical media, spectral transport, displacement, subsurface surfaces, temporal production and other unresolved dependencies retain their unavailable state. The expanded-catalog roadmap is not marked complete.

The algorithms and code in this batch are original. Public mathematical references cover variance/stratification, Beer transmittance and phase-function definitions; no third-party integrator or codec implementation is copied. No runtime dependency or external scene imagery is added.

References:
- https://pbr-book.org/4ed/Monte_Carlo_Integration/Improving_Efficiency
- https://pbr-book.org/4ed/Volume_Scattering/Transmittance
- https://pbr-book.org/4ed/Volume_Scattering/Phase_Functions
- https://openexr.com/en/latest/OpenEXRFileLayout.html

## Acceptance checkpoint

The renderer core has executed 473 passing tests with zero failures, including allocation-oracle, native-master decode/regrade/lease/cancellation and homogeneous-medium mathematical checks. Earlier strict all-workspace checks passed before the final volume integration. The combined current-head strict checks, real GPU transport tests, native application walkthrough and any final catalog-family acceptance must be recorded separately when actually executed. No aggregate project CI, merge or release is claimed here.
