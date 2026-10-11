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

Run from a script or the control channel, every command that adds sketch geometry returns the
ids of what it added (`created: {points, curves, constraints, dimensions}`). Ids are stable and
never reused within a sketch, even after a delete, so a script can dimension or constrain the new
geometry right away.

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
