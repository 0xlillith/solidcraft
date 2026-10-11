# Solid features

Features live on the **timeline** and rebuild incrementally from the first one that changed.
Each one can be edited (double-click), suppressed, renamed, reordered (if nothing it depends on
moves after it), rolled back to, and deleted. Deleting asks first when other features depend on
it.

| Panel | Features |
|---|---|
| Create | extrude (one side, two sides, symmetric, taper, start offset, to a distance or through all; new body, join, cut, intersect), revolve, sweep, loft, box, cylinder, sphere, torus, coil, pipe, rib, web, emboss, hole (simple, counterbore, countersink, tapped; on faces or at sketch points), thread, rectangular/circular/path patterns, mirror |
| Modify | press pull, fillet, chamfer, shell, draft, scale, combine, split body, offset face, replace face, move/copy, align, remove |
| Construct | offset plane, plane at angle |
| Inspect | measure, section analysis, interference, curvature comb and map, zebra, draft, minimum radius, isocurves, centre of mass |

Extrude automatically selects **Join** for additions when the active component already has a
solid, including sideways elbow connections and screw bosses that meet only part of a face.
Extruding into existing material selects **Cut**. In an empty component it selects **New Body**;
choose **New Body** explicitly to keep an addition separate. Changing the default does not alter
operations already saved in a design.

Fillets and chamfers handle straight edges between planar faces, whole smooth loops, runs of
neighbouring edges with mitred or spherical corners, and closed chains of curved edges (a rolling
ball). See the roadmap's known limitations for the rest.

Feature references to faces and edges are stored by name, so an upstream edit that moves geometry
keeps them on the right face (see [Persistent naming](../dev/naming.md)). When a reference can't
be found, the feature warns instead of silently picking another face.

## Editing a feature from a script

`timeline.inspect {feature}` returns a feature's `definition`: the fields `timeline.edit` changes,
with their current values. `timeline.edit {feature, set}` merges `set` into that definition
(objects are merged field by field, anything else replaces the value), rebuilds the feature and
reports `recomputed` and `restored`. The feature's type cannot change, and sketch geometry is
edited with the sketch commands. Expressions are text (`"30"`, `"30 mm"`, `"width / 2"`,
`"45 deg"`); a bare number is a length in the design's units, or degrees for an angle.

```json
{"command": "timeline.inspect", "params": {"feature": "Slab"}}
→ {"type": "ExtrudeFeature",
   "definition": {"sketch": 1, "profiles": {"type": "all"}, "operation": "new_body",
                  "extent": {"distance": "10", "direction": "positive"}},
   "inputs": [{"label": "Distance", "param": "d1", "expression": "10", "unit": "mm"}], …}
{"command": "timeline.edit", "params": {"feature": "Slab", "set": {"extent": {"distance": "12"}}}}
```

`inputs` lists the feature's named inputs. `parameters.change {name: "d1", expression}` sets one
of those directly. Optional fields that are not set do not appear in the definition; they can
still be added through `set`. The fields of the common features:

| Feature | Fields |
|---|---|
| Extrude | `extent`: {`distance`, `direction`: positive\|negative\|symmetric, `distance2`, `start_offset`, `through_all`, `taper`, `to`, `to_offset`, `from`}; `operation`: new_body\|join\|cut\|intersect; `targets`; `profiles`; `sketch` |
| Revolve | `angle`, `angle2`, `symmetric`, `axis`, `to`, `operation`, `targets`, `profiles`, `sketch` |
| Fillet | `radius`, `edges` (a point on each edge), `body`, `style` |
| Chamfer | `distance`, `distance2` or `angle`, `flip`, `edges`, `body` |
| Box | `length`, `width`, `height`, `corner`, `operation` |
| Cylinder | `radius`, `height`, `base`, `axis`, `operation` |
| Hole | `diameter`, `depth`, `position`, `direction`, `hole` (simple, drilled, counterbore, countersink), `thread`, `to` |
| Shell | `thickness`, `faces`, `direction`, `body`, `tangent_chain` |
| Draft | `angle`, `faces`, `neutral`, `pull`, `body` |
| Offset face | `distance`, `faces`, `body` |
| Pattern | `pattern` (its kind and counts/spacing), `features`, `bodies` |
| Mirror | `plane`, `features`, `bodies`, `combine` |

For any other feature, `timeline.inspect` shows the fields.

Press **Delete** on anything selected (bodies, faces, sketches, features, planes, components,
canvases). It runs as one undoable step and lists the features that would go with it or fail.
