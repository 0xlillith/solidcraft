# Working with individual instances

Each occurrence placement has a full path of occurrence IDs, from the design root to that placement. A child definition reused under several parent instances has a different full path in each parent. Names are labels; paths identify placements across rename and position capture.

Select a placement through `select.set`:

```json
{"items":[{"type":"occurrence","path":[2,4]}]}
```

Properties accepts the leaf occurrence ID and the full path. Move can use world coordinates, including a world rotation pivot:

```json
{"occurrence":4,"path":[2,4],"space":"world","translate":[10,0,0],"capture":false}
```

World movement includes pending parent and child positions. The default space remains parent-local for existing scripts. Capture Position commits pending transforms; Revert Position drops them. Copying an occurrence copies that selected occurrence's effective transform, including its pending position.

An occurrence definition inside a component is shared by every instance of that parent component. Moving, grounding, copying, or deleting that nested definition changes all its parent instances. Selecting its exact path determines the world frame used for movement and Properties; it does not create an independent child definition.

Activate a placement to author shared geometry through its frame:

```json
{"component":3,"occurrence":4,"path":[2,4]}
```

New geometry belongs to the component and appears in every instance. The selected frame determines how world inputs become local component geometry. Planar sketches display at the active placement during authoring. Component-only activation keeps the first-placement behavior. Existing 3D wires retain their prior coordinate behavior; their mixed legacy coordinate storage needs a separate migration before occurrence authoring can apply consistently.

Visibility is an independent view setting for exact placements. Hide a path and all descendants with `browser.visibility`:

```json
{"visible":false,"paths":[[2,4]]}
```

`visible:true` removes that path's own hide setting. A hidden ancestor still hides its descendants. Add `isolate:true` to a visible path list to hide other branches. Show the bodies folder to clear occurrence hides. Selection, activation paths and visibility are session state; the saved document retains occurrence IDs and transforms. Undo follows the existing behavior of clearing selection.

Paths are checked for missing links, repeated IDs, component mismatches and a maximum depth of 64. Descendant expansion rejects more than 100,000 placements or bodies instead of returning incomplete results. Computed body names normally retain their readable occurrence labels; collisions use full occurrence IDs so visibility and body commands still address the correct source.

Placed bodies retain their definition identity separately from display names. Body materials and appearances remain shared, with body overrides taking precedence over component overrides and material colors. Face overrides follow the placed geometry, including pending positions, and painted exports retain these colors.
