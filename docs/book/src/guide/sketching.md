# Sketching and constraints

A sketch lies on a plane: an origin plane, a construction plane or a planar face. Start one with
**Create Sketch** (`sketch.create`) and click a plane, or pick a face first. **Finish Sketch**
returns to the model.

## Geometry

Lines and polylines (L), rectangles (two-point R, three-point, centre), circles (centre C,
two-point, three-point, tangent), arcs (three-point, centre, tangent), polygons, slots, ellipses,
fit-point and control-point splines, conics, points and text. **Construction** (X) turns curves
into reference geometry that doesn't form profiles. Modify tools include trim (T), extend, break,
fillet, chamfer, offset (O), mirror, patterns, move/copy and scale.

**Project** (P) and **Intersect** bring model edges into the sketch as linked geometry that
follows later changes. Sketching on a face projects its edges automatically.

## Constraints and dimensions

The solver is SolveCraft's own (damped Gauss–Newton with degree-of-freedom analysis). Constraints:
coincident, horizontal/vertical, parallel, perpendicular, tangent, equal, concentric, collinear,
midpoint, symmetry, fix, curvature (G2) and polygon. **Sketch Dimension** (D) adds linear,
aligned, horizontal, vertical, radius, diameter and angle dimensions. Each is a parameter
(`d1`, `d2`, …), so it can hold an expression.

Under-constrained geometry is blue and fully constrained geometry turns dark, and the status
shows the remaining degrees of freedom. A constraint or dimension that would over-constrain the
sketch is refused rather than applied. Drag a point to move it; the solver keeps every
constraint while you drag.

## Profiles

Closed loops become profiles automatically, including regions split by crossing lines and
islands inside outlines. Extrude, revolve, sweep and loft pick profiles.

A feature remembers a picked profile by the curves around it, not by where it was clicked:
`"profiles": [{"curves": ["l1", "l2", "a1", "l3"]}]` (or `[{"curves": ["c1"]}]` for a circle)
picks the region whose outer loop is made of exactly those curves, or the pieces of them that
other curves cut off. It is found again on every recompute, so the feature keeps the same region
when a parameter moves the geometry. Islands inside the region are its holes. When no region, or
more than one (both halves of a circle cut by a line), is bounded by exactly those curves, the
feature reports an error rather than taking another region. Profiles picked by index or by point
(`[{"point": [x, y]}]`) are stored the same way when the feature is made.
