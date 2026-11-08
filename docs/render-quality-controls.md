# Render quality, CPU workers and guide diagnostics

The Render workspace exposes sample presets through **128 spp**, an **8K UHD
(7680 × 4320)** output preset, optional **X2 supersampling**, CPU worker selection,
and explicit denoising settings. These controls are saved with the selected
render setup. Check and Render consume the same captured values; reopening a
file, changing an unrelated control or applying Undo does not clamp a legacy job.

## Controls and saved values

| Control | Saved CPU option | Saved GPU option | Meaning |
|---|---|---|---|
| Samples | `progressive.samples_per_pixel` | `samples` | New UI selections from 1 through 128; presets 1, 2, 4, 8, 16, 32, 64, 128 |
| Supersampling | `supersampling` | `supersampling` | `1` keeps native size; `2` traces twice the output width and height |
| CPU workers | `threads` | Not applicable | `0` Auto, `1` Single, or an explicit count from 2 through 64 |
| Denoising | `denoise` | `denoise` | Enable the existing geometry-aware spatial display filter |
| Filter passes | `denoise_settings.passes` | Same | 1 through 3, using steps 1, 2 and 4 in order |
| Filter strength | `denoise_settings.strength` | Same | Finite per-pass blend from 0 (raw image) through 1 (full selected filter) |
| Firefly suppression | `denoise_settings.firefly_suppression` | Same | Optional first-pass isolated-bright-pixel heuristic on ordinary surfaces |
| Guide diagnostics | `denoise_diagnostics` | Same | Capture guide eligibility, bypass counts and a categorical mask |

CPU options live under `job.options` for a saved `cpu_raytrace` setup and under
`raytrace` for a runtime CPU request. GPU options live under `job.options` for a
saved `gpu_path` setup and at the top level of an explicit progressive runtime
request. Width and height continue to describe the **exported image**.

The defaults are native resolution, no diagnostic capture, and the previous
three-pass, full-strength filter with firefly suppression when denoising is
selected. Omitted CPU worker selection uses Auto. Single retains the serial
execution path. Existing API and saved sample counts above 128 remain supported
within their original backend/profile limits; the UI preserves them and explains
that newly authored selections stop at 128. Canonical quality presets that exceed
128 are not silently rewritten when selected in this UI.

## Resolution and linear X2 reconstruction

The output controls accept sides up to 8192 and at most 33,554,432 pixels. The 8K
preset is 7680 × 4320. X2 at that size traces a 15360 × 8640 grid and returns a
7680 × 4320 image. It costs four times as many camera samples at the same spp;
it does not change the spp value or create four independent photon-map rebuilds.

Each 2 × 2 block is averaged in **scene-linear, coverage-premultiplied RGBA**,
using f64 accumulation and one f32 rounding. Alpha is resolved with radiance.
There is no averaging of encoded sRGB values, tone-mapped highlights or straight
RGB at transparent edges. For filtered output, denoising operates on the internal
grid first; reconstruction precedes exposure, tone mapping and finishing.
Consequently the filter footprint measured in final pixels changes with X2.

A retained raw FLOAT32 EXR is separately reconstructed from the unfiltered
internal radiance and coverage. It has the selected final dimensions and excludes
denoising, exposure and finishing. PNGs and progressive previews have the same
selected dimensions. The trace grid and actual camera-sample totals remain
visible in internal sampling/diagnostic metadata.

X2 is a distinct experiment from four times the native spp. For example, 8 spp
with X2 and 32 spp at native resolution have equal camera-sample counts, but
sample placement, photon-map refresh frequency and the denoiser footprint differ.
Compare both the raw masters and the finished images at the same output size.

## CPU execution and resource profiles

Native CPU workers share the immutable prepared scene, BVH, optical certification,
texture storage and completed photon map. Each worker owns only bounded transport
state, texture/effect scratch and a small result tile. Camera results are merged
in pixel/sample order, and photon deposits in photon ordinal order, so worker
scheduling does not alter the random keys or film accumulation order. A photon
map is completed before the corresponding camera pass uses it.

Auto uses the host's available parallelism, capped at 64. Explicit selection is
bounded at 64. The renderer admits worker stacks and scratch before creating a
pool and reuses that pool within the image job. Workers share the cumulative
work budget and cancellation token; no worker gets a new full job budget.
The wasm build retains cooperative serial execution and the native worker
selector is unavailable there.

Resource profiles still apply. Compact keeps its 64-spp progressive ceiling and
its smaller pixel, camera-sample and memory envelopes. Use an explicit larger
profile for 128 spp or large output. With X2, the selected profile is charged for
the internal grid and all camera samples. Workstation's internal pixel envelope
allows up to 134,217,728 pixels, subject to its existing finite memory/work caps.
Legacy native-size CPU Workstation requests retain their 16,384-side contract
when denoising, guide diagnostics, raw EXR and nonidentity output presentation are
disabled. Their profile pixel, memory and work limits still apply. New UI size
selections and CPU jobs using X2 or those output features use the 8K output
envelope; the UI does not silently resize legacy jobs on load.

An 8K preset is not a guarantee that every combination of X2, denoising, raw
retention, scene complexity and device will fit. GPU admission checks the actual
storage binding limit, buffer limit and finite 12 GiB host reservation. Device
requests use available adapter capacity instead of the earlier 1 GiB software
cap; this does not allocate the maximum at startup. CPU admission includes
film, guide records, output scratch, diagnostics and worker storage. An exceeded
limit produces an error with the requested values intact. No profile, resolution,
sample count or executor is changed automatically.

## Denoising controls and optical policy

Pass count, strength and firefly suppression affect display filtering only.
They do not change transport, the sample stream or the raw EXR. Original coverage
is preserved. Strength zero returns the original linear RGBA. The firefly toggle
controls the isolated-bright-pixel heuristic; it does not disable the ordinary
bilateral contrast weights.

Optical guides keep their conservative policy: they follow supported sharp
reflection and smooth-glass branches to compatible opaque endpoints, require
both contributing endpoint identities to agree, and use at most the first two
spatial steps. Firefly suppression remains disabled on optical guides. Raising
pass count or strength does not make unsupported optics eligible. Fine glints,
soft shadows and difficult glass still require sample convergence.

See [spatial denoising](render-denoise.md) for the guide and numerical contracts.

## Guide mask, counts and export

Diagnostics are opt-in and can be captured with filtering either enabled or
disabled. They record the actual reason selected during guide generation rather
than inferring optical eligibility from the final image. Surface and Optical
identify eligible guides; the other categories identify bypass conditions:
background, emitter, unsupported material, nested medium, depth of field, volume,
path depth, degenerate path, incompatible exit, total internal reflection,
further transmission, reflective endpoint, identity limit, partial coverage or
unavailable guidance.

The Inspector reports counts over the **internal trace grid**. The result viewer
can switch between the completed image and the guide mask. The mask is a separate
opaque PNG with fixed category colors and nearest-neighbor viewing; it never
replaces or modifies the color image or linear master. X2 reduces classes
conservatively: a bypass in a 2 × 2 block remains a bypass, and mixed eligible
Surface/Optical classes become Unavailable. Category numbers and palette colors
are never averaged.

Explicit mask export uses the existing atomic export boundary:

```json
{"command":"render.export","params":{"job":{"session":3,"serial":1,"generation":1},"path":"guide-mask.png","format":"denoise_mask"}}
```

`render.result.view {"view":"denoise_mask"}` selects the mask for a completed job
that captured it. Export replies label the file as `categorical_srgb` and opaque.
The `denoise_mask` metadata has its own dimensions, byte length and SHA-256;
`frame.png_sha256` still identifies the color PNG. Each retained file is hashed
independently and charged to the host lease. Cancellation or invalid output
cannot publish a partial mask.

Regrading a retained raw master preserves access to its source mask and original
diagnostic report. It does not regenerate geometry guides or reapply denoising.
The derivative's `denoise_applied` metadata remains false. A completed job that
did not capture diagnostics cannot acquire them from a later editor change.

## Example saved GPU setup

```json
{
  "command": "render.settings.set",
  "params": {
    "name": "Quality comparison",
    "set": {
      "width": 1920,
      "height": 1080,
      "job": {
        "method": "gpu_path",
        "options": {
          "samples": 32,
          "supersampling": 2,
          "denoise": true,
          "denoise_settings": {
            "passes": 2,
            "strength": 0.75,
            "firefly_suppression": false
          },
          "denoise_diagnostics": true,
          "linear_exr": true
        }
      }
    }
  }
}
```

Preflight this setup on the intended device before executing it. New saved fields
are documented by the pending `render-quality-controls` format note. Existing
qualification renders in [OPUS qualification](render-opus-quality.md) retain their
original source/binary identities; they are not evidence for a later build.
