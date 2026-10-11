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

Extend finds intersections with exact ellipses, rational conics and spline spans. It keeps the
original line or circular support and extends only beyond the selected endpoint; conflicting
constraints or an ambiguous overlap leave the sketch unchanged.

Offsets of smooth ellipses, conics and splines follow their source after dimension changes,
dragging and reopening a design. Their stored cubic spline has a whole-span normal-distance
error bound of 0.002 mm. Singular tangents, curvature cusps, possible self-intersections and
excessive subdivisions are refused. Offset these curves individually: mixed chain junctions
currently require separate geometry. `sketch.offset.break` releases an offset for independent
editing, and undo restores its association. Older independently sampled offsets remain independent.
Smooth open and newly created smooth closed offsets can be offset again, including bounded
refinement after a source edit. New closed outputs use a periodic cubic fit with continuous
tangent and curvature at the seam. Break Offset Association preserves that interpolation,
and its closing point remains joined when dragged. Deleting an output also removes its
unused private fit points; deleting a source with a retained dependent is refused.

For explicit closed fit splines, `sketch.spline.fit_point` accepts `periodic: true` and at
least three distinct cyclic samples. It closes the loop automatically. Existing calls and
older saved closed splines retain natural interpolation; older associative closed offsets
also retain their saved natural fit. A legacy seam without a unique normal still refuses
another offset. Periodic intent is stored on each curve and survives save, reopen and Break.
While drawing a fit spline, click its starting point after three distinct samples to close
it smoothly. Right-click finishes an ordinary natural fit. Offset accepts both ellipse
handles and dense fitted curves; the second click sets the side and distance from the curve.

Enable **3D Sketch** in the Sketch Palette to draw lines, fit splines and points off the plane.
New 3D geometry follows its sketch's component placement, including the selected occurrence
and pending position changes. Click a drawn fit point to show its move triad; its arrows move
along world X, Y and Z. Scripts also supply world coordinates to `sketch.line3d`,
`sketch.spline3d`, `sketch.point3d` and `sketch.move3d`. Activate the desired occurrence before
editing an explicit sketch belonging to a component with multiple placements.

Older drawn or detached 3D wires keep their saved world positions. Included and surface-linked
wires with an explicit component marker keep that meaning after the link is broken. Old files
whose links became lost after an incorrectly scoped cross-component refresh may require a
fresh link once the source is available; their stored point values are preserved on opening.

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
