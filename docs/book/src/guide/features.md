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

Editing an extrusion retains its signed distances, From/To Object inputs, taper expressions
and explicit body targets. Revolve edits retain world, sketch and explicit-line axes, both
angle expressions and To Object targets. An unchanged **All profiles** selection continues
to include profiles added later. Cancel restores the complete timeline; a successful edit is
one undo step, including downstream parameter changes. Editing a placed component previews the
edited feature before its downstream history. Cylindrical Emboss profile guides follow the
sketch's chosen occurrence frame.

New symmetric revolves split the entered angle evenly around the sketch profile, regardless
of which way the selected axis points. Two Sides measures the second angle backward and the
first angle forward along the sketch-normal sweep direction. Profiles on opposite sides of
an axis each use their own sweep direction. Designs saved before format 11 keep their original
angular placement when opened, edited or copied; creating another revolve uses the new behavior.
To Object follows that same direction for each selected profile, including curved target faces.
Pick a target away from the axis so its angular position is defined.

For cylindrical **Emboss**, select a supported exterior cylinder face and a tangent sketch
profile. **Deboss** cuts radially into the wall. Orthogonal line profiles, including holes and
concave outlines, wrap with exact radial walls; the profile must stay clear of seams and ends.
Complete cylinders and coaxial sleeves are supported. Capture pending component positions
before applying. The feature stores the selected target and author occurrence paths, so edits
and reopening retain the chosen instances. Other curved surfaces and free-form profiles remain
unsupported. A new Emboss without an explicit face infers a unique eligible supported cylinder;
select the face when the target is ambiguous. Older saved features without a face retain their
planar placement.

Fillets and chamfers handle straight edges between planar faces, whole smooth loops, runs of
neighbouring edges with mitred or spherical corners, and closed chains of curved edges (a rolling
ball). See the roadmap's known limitations for the rest.

Feature references to faces and edges are stored by name, so an upstream edit that moves geometry
keeps them on the right face (see [Persistent naming](../dev/naming.md)). When a reference can't
be found, the feature warns instead of silently picking another face.

Press **Delete** on anything selected (bodies, faces, sketches, features, planes, components,
canvases). It runs as one undoable step and lists the features that would go with it or fail.

Face Extrude, face Revolve, and Press Pull retain linked generated profiles. When a modified planar face has no construction certificate, these internal profiles use the legacy named-face attachment and projection behavior so upstream edits still update the plane and outline. This compatibility path is private to those modeling tools: newly attached user sketches require a certified face or an explicitly fixed plane. Legacy internal attachments can follow the nearest matching face after topology changes; inspect their result after major upstream edits. Activate the intended occurrence before generating a face profile from a repeated component; capture a separately moved source position before creating a legacy profile across components.

A negative Press Pull across a planar face containing a round through-hole can still fail when the Boolean cutter has coincident curved faces. This operation reports an error and preserves the design atomically; positive face extrusion and Press Pull retain the hole and remain linked.
