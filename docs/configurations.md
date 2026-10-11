# Design configurations

The configuration table stores named design variants. Add parameter, feature suppression,
physical material, and body appearance columns with `config.configure` or `config.column`.
The desktop Configuration Table's **Add Column** menu offers the same targets. Material and
appearance cells have a **Library** picker; their text fields also accept exact saved values.

```json
{"command":"config.configure","params":{"params":["width"],"materials":["Box1"],"appearances":["Box1"]}}
{"command":"config.row","params":{"name":"Aluminum blue","values":{"param:width":"80 mm","material:Box1":"Aluminum","appearance:Box1":"Aluminum - Anodized Blue"}}}
{"command":"config.activate","params":{"row":"Aluminum blue"}}
```

Use the body's actual Browser name in place of `Box1`. `config.table` returns the column keys,
rows, active name, and `matches`: whether the current design still has the active row's values.
Changing an assignment by hand makes `matches` false. Activating another row is undoable.

Material values are names from `material.list` (case insensitive), with `Default` clearing the
body's physical material assignment. Appearance values are names from `appearance.library`,
an empty string or `Default` to clear the body's appearance, or a custom look object:

```json
{"command":"config.row","params":{"name":"Transparent blue","from":"Aluminum blue","values":{"appearance:Box1":{"name":"Custom blue","color":[40,90,186],"opacity":0.4}}}}
```

The table stores custom looks without losing colour or opacity. `color` contains three byte
values, names support up to 128 Unicode characters, and `opacity` must be finite and between
zero and one. Style values are validated when
editing a row and again when activating a saved row. An invalid value leaves the design unchanged.
Clearing a body appearance allows component appearance or material colour to show through;
face appearances keep their existing priority. Physical material changes affect measured mass.

`config.row` supports `from` to copy another row, `rename` to rename an existing row, and
`delete` to remove it. Names must be unique. To remove a style column, use
`config.column` with `material` or `appearance` set to the target body and `delete: true`.

Style columns target body names. Assignments remain stored when a row suppresses the feature
that produces that body, and become visible when it returns. Component and face appearance
columns and conditional configuration rules are not yet supported.

Designs saved with these columns use file format 7. Older designs upgrade on opening without
changing their existing configuration values. Older SolveCraft versions must be updated to
open newly saved designs.
