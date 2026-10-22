# Built-in render resource generation and native probes

All tools here are offline contributor tools, not application runtime dependencies.
They use original resources already attributed in the starter pack. Source
manifests and their authoring-readiness flags are never rewritten.

## Fixed built-in lookup

```sh
python tools/render-library/generate.py --check
```

Without `--check`, the generator writes `library_data.rs`. It verifies all 192
runtime eight-bit maps and 24 HDR files against their published encoded hashes.
The 48 sixteen-bit height maps remain authoring-only and are not silently decoded
as eight-bit color.

## Focused hardware GPU probe

```sh
python tools/render-library/run_native_smoke.py --out /new/output/directory --target-dir /existing/cargo/target
```

Requires a clean committed source and a Vulkan hardware adapter. The runner builds
only normal geom/render/gpu libraries, then links `native_smoke.rs` to the exact
Cargo-reported artifacts. It does not build the CAD kernel, engine, desktop UI or
all test binaries. The GPU must satisfy the explicit hardware policy; no software
or CPU fallback is allowed.

The original coupon is rendered three ways: four authored PBR maps with absolute
base color, legacy color modulation, and a scalar-control request using the
captured roughness/metallic values. The probe checks actual executor identity,
frame hashes, dimensions, PNG CRCs and bounded scanlines, distinguishable control
images and released reservations. The renderer process must log one non-software
adapter. Original SURF-001/HDR-001 source hashes and source-tree stability are
checked separately. Inspect the resulting PNGs before recording visual acceptance.

The output directory is new and caller-selected. Receipts/logs/images are not
committed. A passing probe is native renderer evidence for this original scene,
not GUI acceptance, a full-engine command test, measured material fidelity or a
performance benchmark.

## Bounded CPU scale probes

```sh
cargo build --locked --profile ci -p solidcraft-render --features raytrace-cpu --example cpu_resource_smoke
# Select one case per fresh output folder:
cpu_resource_smoke /new/output/directory 4k
cpu_resource_smoke /another/output/directory 8k
cpu_resource_smoke /another/output/directory dense
cpu_resource_smoke /another/output/directory bodies
```

`4k` renders an original analytic plate at 3840x2160 under Studio; `8k` renders it
at 7680x4320 under Workstation. `dense` renders a 500,000-triangle planar grid at
256x256; `bodies` renders 20,000 separated tiles at 512x256. These are deliberately
simple synthetic cases, not representative CAD assemblies or speed benchmarks.

Every case verifies that Compact refuses the oversized request, the selected
profile completes a real image containing geometry, and retained host reservations
return to zero. A separately identified 20,000-body admission-only control is not
confused with the rendered case. The job wait is bounded at 300 seconds; failure
or timeout is reported, never silently retried at reduced quality.
