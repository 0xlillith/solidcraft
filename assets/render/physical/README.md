# Physical reference properties for the render material catalog

Version 1.0.0, researched and authored 2026-10-22. This collection adds physical
reference data alongside the visual catalog. Appearance, material chemistry,
manufacturing process and assigned part physics remain separate decisions.

## Coverage and actual availability

| Item | Present in this slice |
| --- | --- |
| Catalog concepts with an explicit physical specification | 600 of 600 |
| Authored PBR surfaces linked to those specifications | 48 of 48 |
| Named reference profiles | 51 |
| Individually sourced numerical records | 373 |
| Source references | 42 |
| Concepts with one or more candidate/constituent/comparison references | 292 |
| Concepts still requiring a specific grade, formulation or construction | 308 |
| Authored surfaces with candidate references | 43 of 48 |
| Partial native schema-1 cards containing supported properties | 44 |
| Profiles whose data is retained only in the extended representation | 7 |
| Native property blocks | 106 |
| Profiles having a single point density suitable for an explicit creation plan | 43 |

**Specification coverage is not numerical completeness.** A link may identify a
constituent or an explicitly non-equivalent comparison, not the effective material
of a complete part. No record is marked certified, simulation-ready or already
assigned to a model. No live design, appearance, body assignment, mass or saved
material was changed by producing this collection.

The five authored surfaces without a numeric candidate remain explicit: matte
industrial paint (SURF-013), satin industrial paint (SURF-015), hammer-finish enamel
(SURF-016), automotive clearcoat surrogate (SURF-017) and woven aramid resin
(SURF-035). Exact formulation/cure, coating stack or fibre/resin/layup data is needed;
an unrelated generic density or elastic modulus would be misleading.

## Contents and ownership

- [profiles.json](profiles.json): editable physical profiles and property-level
  source, conditions, units, original values, uncertainty and derivation receipts.
- [sources.json](sources.json): cited public manufacturer, supplier, trade-body
  and government references, retrieval date and source-review notes.
- [bindings.json](bindings.json): editable associations for all MAT and SURF IDs,
  physical roles, remaining definition requirements and safe assignment scope.
- [quantities.json](quantities.json): reviewed quantity/unit and SI conversion
  registry, independently checked by the validator's dimensional contract.
- [exports/native-cards.json](exports/native-cards.json): generated supported
  schema-1 projections, explicit omissions and partial/extended-only status.
- [exports/coverage.json](exports/coverage.json): generated counts and scope flags.
- [INTEGRATION.md](INTEGRATION.md): runtime contract and loss-aware assignment rules.
- [SOURCE_REVIEW.md](SOURCE_REVIEW.md): source discrepancies and interpretation limits.

The JSON masters are the editable authority. The Python tools validate and derive
exports; there is no second hard-coded recipe generator to keep synchronized.
Review reports and command plans belong in ignored `target/`, not in the resource
collection. No PDFs, copied supplier tables, images or external implementations
are bundled.

## Physical content

The collection includes alloy, stock-polymer, glass, ceramic, elastomer, cured
resin, printed-coupon, laminate, timber, cork, coating and terrestrial-regolith-
simulant references. Selected quantities include density, elastic and shear
moduli, Poisson ratio, tensile/yield/compressive/flexural strength, elongation,
scale-qualified hardness, thermal conductivity/expansion/heat capacity, transition
and melting temperatures, electrical resistivity, frequency-qualified permittivity
and loss, dielectric strength, moisture absorption, porosity and particle data.

Not every profile has every quantity. A temperature limit is not a melting point;
a flexural modulus is not silently renamed tensile Young modulus; elastomer stress
at 100% extension is not a small-strain modulus. Shore A, Shore D, Brinell, Knoop
and Janka retain their individual definitions. Test speed is not converted to a
strain rate without a gauge-length definition.

Native fields are SI: kg/m3, Pa, W/(m K), 1/K and J/(kg K). Rich records retain
source values and conversions, including affine Celsius/Fahrenheit-to-kelvin
conversion. Lower bounds, upper bounds, intervals, test directions and reported
plus/minus spreads remain explicit. A supplier spread is not assumed to be a
confidence interval or a design tolerance.

## Use

From the repository root, Python 3.11 or newer; no third-party Python dependencies
are needed for these physical-data tools:

```sh
python -B assets/render/physical/tools/physical.py validate
python -B -m unittest discover -s assets/render/physical/tools -p 'test_*.py'
python -B assets/render/physical/tools/physical.py resolve SURF-001
python -B assets/render/physical/tools/physical.py resolve MAT-161
```

After a reviewed edit of the JSON authority, regenerate the two portable exports:

```sh
python -B assets/render/physical/tools/physical.py build
```

A selected profile with a supported point density can produce a **non-executing**
material-creation plan:

```sh
python -B assets/render/physical/tools/physical.py prepare steel-c45 --acknowledge-reference
```

That output is a plan envelope, **not** a native catalog or a directly executed
script. It names the required modern `material.create` contract, contains the
schema-1 `physical_card`, converts the explicitly chosen density to g/cm3 for the
material command, and never assigns a body or changes an appearance. Preparation
refuses missing or interval-valued density; it does not select a midpoint.

Run the physical suite and optionally the real read-only native validator:

```sh
python -B assets/render/physical/tools/check_physical.py
python -B assets/render/physical/tools/check_physical.py --cli /path/to/trusted/solidcraft-cli
```

The optional CLI must be a trusted executable. Its SHA-256 is recorded; this gate
does not build it. Native card admission checks numerical structure and source
references, not supplier authenticity, source rights, effective-part behavior,
persistence or safety. Seven valid but empty native envelopes are counted as
**extended-data-only**, not ready material cards.

The existing starter resource gate now includes the physical regression suite,
read-only export validation and physical-source fingerprints in both its full
and authoring-only modes. Its existing workflow path filter already covers this
folder. The standalone native card check remains a separate explicit CLI option;
the resource workflow does not claim to have built or tested material persistence.

## Assignment principles

A painted or anodized part keeps its substrate's physical material. Coating mass
requires area, thickness, coverage and cured density; thermal/contact behavior
requires appropriate interfaces. An aluminium-facing thermal blanket is not a
solid aluminium body. FR-4, copper, solder mask and populated components are not
one interchangeable homogeneous substance.

Timber needs species, moisture and L/R/T axes. Printed parts need feedstock,
process, infill, porosity, build direction and post-treatment. Fibre properties
are not laminate properties; a unidirectional coupon does not identify a woven
composite. Lunar-like visual concepts retain explicitly terrestrial analogues,
not invented lunar-site measurements or vacuum-ready constitutive models.

Missing properties remain unknown, never zero. Friction, fatigue, creep,
viscoelasticity, temperature curves, outgassing, radiation response and general
failure criteria are not supplied merely by assigning an appearance. A future
solver must refuse unsupported models rather than silently promoting these
reference cards to complete engineering datasets.

## Attribution

Original schema, selection, annotations, tools and authored association data are
contributor work under MIT OR Apache-2.0. Public source documents retain their
owners' rights; a null source license is not permission to redistribute them.
Selected numerical facts are attributed individually, not presented as original
measurements. See the root [ATTRIBUTION.md](../../../ATTRIBUTION.md).
