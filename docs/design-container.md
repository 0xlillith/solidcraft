# Design containers, version 1

The `.solvecraft` extension is retained for compatibility. Opening detects the ZIP
local-header signature (`50 4b 03 04`) independently of the name; otherwise it uses
the existing JSON design reader and migrations. Plain JSON remains a fully supported,
self-contained input/output format. `io::write_design` keeps its JSON contract;
`io::write_design_container` is the fallible container writer.

Packages have `container.json` with `{"format":"solvecraft-container/1"}` and
`document.json` with the versioned parametric document (currently `solvecraft/14`).
The independent versions let document migrations evolve without changing blob encodings.
Existing serialized document fields are retained automatically, without a second copy
of the document schema or a deep clone of embedded payloads.

Within `document.json`:

- Import features have empty `step` and a `step_blob` reference.
- Mesh imports retain each mesh's name and optional colour, with empty `positions`
  and `triangles` plus a `mesh_blob` reference.
- Canvases retain placement, format and pixel metadata, with empty `data` plus an
  `image_blob` reference. Raw image bytes replace base64 in the package.

References are `blobs/<lowercase SHA-256>.<step|mesh|png|jpeg>`, hashed over the exact
uncompressed payload. Identical content with the same encoding is written once.
Required parts, malformed references, ambiguous embedded payloads, incorrect hashes,
invalid mesh data and unsupported versions produce errors before session adoption.

## Mesh encoding

Mesh blobs begin with the eight ASCII bytes `SCMESH01`, followed by little-endian
`u64` coordinate count and `u64` index count. These count scalar values, not vertices
or triangles. Next come that many IEEE-754 `f64` coordinates, then `u32` indices.
Both counts must be divisible by three; the byte length must match exactly. Coordinates
must be finite and at most 1e9 mm in magnitude; indices must address an existing vertex.
Names and colours remain in JSON. Signed zero and full coordinate precision survive.

The default `fast` policy stores STEP and binary meshes without Deflate for faster
saving and reading. Small document metadata uses Deflate level one; PNG/JPEG are
stored directly. An explicit `compact` policy compresses STEP and mesh payloads at
level one. Readers support both policies. This deliberately favors interactive file
operations over maximum compression; compressed STEP was slower in the storage benchmark.
Use `"compression":"compact"` with `file.save` or `file.save_as` to favor file size,
or `io::write_design_container_with_compression` from Rust.

## Bounds and recovery

Input and individual expanded parts are capped at 256 MiB. Total declared expanded
parts and resolved payloads are each capped at 512 MiB, including repeated references
in the latter budget. There are at most 10,000 entries. Duplicate/ambiguous entry names,
path traversal, ZIP64 and multi-disk packages are refused. Entries are read in memory;
no archive paths are extracted to the filesystem. Exact part lengths and ZIP CRCs are
checked, along with blob hashes. Hashes detect correspondence and corruption, not trust.

Saving retains the existing temporary-file/flush/rename protocol and `.bak` backup.
Recovery reads both older JSON and new containers. Browser OPFS, downloads and recovery
use the same container reader/writer; JSON export and Save As remain available.

`cache/` entries are currently ignored. No evaluated B-rep is deserialized or adopted,
and the authoritative document always rebuilds. Evaluated cache validation, provisional
display and background rebuilds remain phase two of issue #303.
