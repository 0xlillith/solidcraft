# Matched GGX visible-normal sampling foundation

This constituent exposes an original safe-Rust numerical proposal through
`AnisotropicSurface::sample_visible_normal`, `visible_normal_pdf`,
`visible_reflection_pdf` and `sample_visible_reflection`. It does not yet install
the CPU reflection consumer, alter GPU shaders, introduce a renderer capability,
or persist new data. The consumer remains tracked by #1698 and #1654.

The proposal conditions on the view direction using the spherical-cap
construction. Its PDF and throughput use the same GGX distribution, separable
Smith masking and Fresnel law as the existing surface. Geometric or shading
hemisphere rejection is a null event, never retried or renormalized. Invalid
variates and nonfinite arithmetic refuse explicitly. Existing projected-NDF
methods remain available as independent numerical controls.

At dielectric IOR exactly one, the specular contribution is zero at every angle;
a metallic mixture retains only its conductor fraction. The shared Fresnel helper
and both isotropic/anisotropic numerical paths use that same limit. No tolerance,
energy clamp, sample budget or ignored test is changed.

The original regressions exercise analytic moments, density mass and Jacobians,
throughput/BSDF agreement, unequal roughness axes, rigid frame rotation,
hemisphere rejection and resolved grazing/end-point cases. They do not establish
a universal floating-point proof or measured image-quality/performance benefit.
Actual source-pinned test results belong in the PR evidence, not this contract.

## Primary references and provenance

The original donor implementation was derived from the mathematical equations in
Eric Heitz, *Sampling the GGX Distribution of Visible Normals* (JCGT 7(4), 2018),
and Jonathan Dupuy / Anis Benyoub, *Sampling Visible GGX Normals with Spherical
Caps* (2023). References:

- https://jcgt.org/published/0007/04/01/
- https://arxiv.org/abs/2306.05044

The Rust implementation and tests are retained from the owned, reviewed donor;
no external implementation or dependency is imported by this extraction. No paper
text, figure or listing is included. Project code remains MIT OR Apache-2.0.
