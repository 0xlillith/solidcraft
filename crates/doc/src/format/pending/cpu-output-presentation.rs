//! CPU render jobs can save optional `options.output` presentation: tone map, finishing,
//! backdrop and raw linear EXR retention. Older designs omit it and retain the exact previous
//! f64 PNG output; exposure remains owned by RenderSetup. The last released format refuses a
//! design containing these values instead of dropping authored output effects or master intent.
//! CPU jobs also save optional `options.lens` using the shared physical thin-lens
//! aperture radius and axial focus distance. Positive aperture requires perspective
//! plus explicit progressive camera sampling; absent/zero aperture keeps old pixels.
//! CPU raytrace jobs also retain default-false `transparent_background` beside options.
//! It exports straight-alpha primary geometry coverage; absent fields keep opaque PNGs.
//! CPU jobs also save opt-in `options.denoise`; omission/false preserves the prior
//! raw PNG calculation. Every complete published pass is filtered without changing
//! raw film or the final unfiltered EXR. Older files need no rewrite for this flag.
//! CPU jobs also preserve optional `options.path: {max_depth: 1..16}` for explicit
//! progressive opaque diffuse/specular transport. Omission keeps the legacy producer;
//! unsupported selected optics are refused, not cleared or silently downgraded.
