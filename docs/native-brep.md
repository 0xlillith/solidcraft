# Portable native B-rep, schema 1

This is SolidCraft's geometry interchange data model in `solidcraft_geom::brep`.
It has no modelling-backend dependency. It is the basis for #303's disposable
native cache; it is not an editable-session snapshot or a STEP replacement for
external interchange. The container reader, binary codec and kernel adapters are
separate layers. The schema is not yet emitted by application saves.

## Geometry and topology

Coordinates are IEEE-754 binary64, in millimetres, right-handed Z-up. Angles are
radians. Integer references are unsigned 32-bit shell-local indices, never kernel
addresses or runtime IDs. The schema version is an explicit unsigned 32-bit field.
A body contains ordered shells; a shell contains ordered vertices, edges and faces.
Order is retained for SolidCraft's face provenance and naming. Closed and open
shells are representable; material containment is not inferred from shell order.

An edge references its start/end vertices in its curve's forward direction. Each
face contains ordered closed loops of oriented edge uses. A reversed use swaps
edge endpoints. Face `forward` controls surface orientation independently of the
loop traversal; adapters must retain both. Shared edges are stored once per shell. This initial subset uses two distinct
endpoint vertices per edge; closed single-vertex edges remain a follow-up.
The validator checks references and connected closed loops. It does not certify
watertightness, geometric coincidence, absence of self-intersections or trustworthy
modelling state; backend extraction has additional requirements.

Curves are tagged mathematical records:

- `line`: start/end points, linear parameter from 0 to 1.
- `polyline`: ordered points, segment i spans parameter i to i+1.
- `spline`: full nondecreasing knot vector and Cartesian control points. Degree is
  knot count minus control-point count minus one.
- `rational_spline`: the same basis with homogeneous `[x*w,y*w,z*w,w]` coefficients,
  retaining the original coefficients rather than normalizing/reapproximating them.
- `intersection`: two supporting surfaces, a polyline or polynomial spline leader,
  and positive spatial tolerance. The leader supplies parameterization and branch
  guidance; it does not replace the actual surface intersection with a polyline.
  An adapter unable to preserve these semantics must refuse the cache.

Surfaces are `plane` (origin and two independent, non-normalized parameter axes),
`spline` or `rational_spline` (u/v knots, u-major rectangular coefficient grid), or
`revolution` (profile, origin and unit axis, full 2-pi angular domain). Revolution
stores an affine matrix as four column vectors, applied after revolving, and an
independent orientation flag (`false` swaps the surface u/v parameters, reversing
the normal). No transforms are silently baked into approximations.
Unsupported additional analytic or procedural forms require a schema extension or
an exact conversion; a cache miss is preferable to a lossy conversion.

## Validation and trust

Schema 1 caps 1,024 shells, 100,000 combined topology/geometry items, geometry
nesting at 16, spline degree at 16 and control points per spline dimension at
4,096. Numbers must be finite and bounded in magnitude by 1e12. Knot order, active
spline domain, multiplicities, rectangular grids, positive rational weights,
nondegenerate planes, unit revolution axes and invertible affine transforms are
checked before an adapter is called. These are resource and structural guards,
not a geometric validity certificate. The binary reader must additionally bound
bytes, allocation and nesting before creating these structures.

Schema tests build and validate an open triangular face without constructing or
linking a kernel body. Kernel-specific structures must appear only inside adapters;
they must never acquire authority over persisted field names or encodings. A
second backend still requires an implemented and tested adapter before claiming
cross-kernel interoperability.

Face-source certificates, operation history, appearances and persistent document
identities belong to a versioned SolidCraft body/state envelope around this geometry.
They must be preserved by the cache codec, not reconstructed from face counts or
floating-point comparisons. A decoded cache remains provisional until authoritative
re-evaluation finishes. The document and original blobs remain sufficient to reopen
when a cache, representation, producer or adapter is unsupported.
