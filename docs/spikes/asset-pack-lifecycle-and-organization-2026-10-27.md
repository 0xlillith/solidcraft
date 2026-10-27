# SolidCraft extras: persistent libraries, package organization, Drawings and themes

- **Status:** architecture and product spike; proposed behavior, not an implemented installer or an accepted format.
- **Date:** 27 October 2026.
- **Companion:** [Asset pack system architecture spike](asset-pack-system-2026-10-27.md).
- **Scope:** storage across application updates; package boundaries and distribution; replacement of the early development archive layout; parts, finishes, lighting, studios, scene setups, Drawings resources and UI themes.
- **Decision change:** the current development ZIP grouping is disposable. Preserve useful content, provenance and existing document meaning; do not make every early archive layout a permanent compatibility obligation.

## Contents

1. [Recommendation and revised decisions](#1-recommendation-and-revised-decisions)
2. [Repository evidence and boundaries](#2-repository-evidence-and-boundaries)
3. [Where installed packs belong](#3-where-installed-packs-belong)
4. [Storage, profiles and application updates](#4-storage-profiles-and-application-updates)
5. [Backup, relocation and browser persistence](#5-backup-relocation-and-browser-persistence)
6. [What an asset, pack and collection mean](#6-what-an-asset-pack-and-collection-mean)
7. [Choosing package boundaries](#7-choosing-package-boundaries)
8. [A proposed catalog structure](#8-a-proposed-catalog-structure)
9. [Distribution and hosting alternatives](#9-distribution-and-hosting-alternatives)
10. [Rebuilding the development catalog](#10-rebuilding-the-development-catalog)
11. [Drawings packs](#11-drawings-packs)
12. [Theme packs](#12-theme-packs)
13. [UI and command behavior](#13-ui-and-command-behavior)
14. [Draft examples and a sizing experiment](#14-draft-examples-and-a-sizing-experiment)
15. [Implementation sequence and acceptance](#15-implementation-sequence-and-acceptance)
16. [Decisions, validation and limitations](#16-decisions-validation-and-limitations)
17. [Sources](#17-sources)

## 1. Recommendation and revised decisions

**Installed packs should survive an ordinary application update because they are durable user data, stored outside the application installation.** A new executable should reopen the same library. It should not download it again, silently select different pack versions, change a drawing template or activate a different theme.

Use the existing per-user data-root convention as the starting point. Introduce a dedicated asset store beneath it, with a separate mutable profile for each channel or explicit user profile. Keep preferences small, caches disposable and installed content immutable. Let users choose a different data drive through a supported move operation.

For organization, use **independently useful packages with curated collections above them**. Package boundaries should follow use, ownership, licensing, capability and update cadence. A collection can offer a convenient one-click selection of parts, finishes, studios, drawings and an optional theme without forcing all of them into one enormous archive or one version number.

Continue with the first spike's inspectable ZIP plus public JSON contract, while replacing the old development archive structure. The extension remains a proposal; an ordinary compatible ZIP is still inspectable with ordinary tools. Distinguish three products: authoring sources, installable runtime packs and catalog/collection metadata.

### 1.1 Decisions recommended now

| Decision | Recommendation | Reason |
| --- | --- | --- |
| Default storage | Stable per-user application-data directory | Routine executable replacement must not own or delete the library. |
| Settings | Source choices, profile selection and small preferences | Large textures, CAD and fonts are data, not settings. |
| Store identity | Stable identifier plus independently versioned storage protocol | Moving the library or updating the app must not change resource identity. |
| Multiple app channels | Separate mutable profiles; reuse compatible immutable content | A nightly must not migrate stable's preferences or database in place. |
| Runtime unit | Coherent, independently useful pack | Avoid both a universal download and thousands of file-sized packages. |
| User-facing collection | Declarative bundle with optional choices and a resolved lock | Convenience without coupling unrelated content or activation. |
| Current development ZIPs | Rebuild them freely into the new organization | Their temporary layout is not the product contract. |
| Existing documents | Preserve exact applied content and its meaning | Repackaging is not permission to reinterpret saved work. |
| Drawings | Typed paper, border, title-block, style and template assets | Physical drawing output needs semantics beyond a background picture. |
| UI themes | Reuse the existing theme contract through a separate pack adapter | Installation, theme activation and document editing are different actions. |
| Source hosting | Small static catalog; immutable payloads on releases or object storage | Browsing should not download the full library. |
| Complexity | Bounded local packs first; more elaborate sharing/transports when measured | Reliability depends on a small, testable lifecycle. |

### 1.2 What changes from the first spike

The first report's emphasis on retaining historical catalog ZIPs and building explicit legacy importers is no longer the default implementation priority. New runtime packs may have entirely different grouping, names and file layout. A legacy adapter is justified only by an actual saved-document or installed-user migration need.

Historical provenance remains valuable: keep an old asset/source identifier as an alias where useful, retain original source hashes, and document one-to-many transformations. Do not overwrite an already advertised immutable pack identity with different bytes. Creating a new development format/version is different from pretending the old bytes never changed.

The first report's generic statement that applying an asset is a document edit also needs a scope distinction:

| Operation | Destination and transaction |
| --- | --- |
| Install a pack or bundle | Library store/profile transaction; no open document changes |
| Apply a material, finish, part or studio | Appropriate CAD/design transaction |
| Apply a sheet template or drawing style | Appropriate drawing transaction and drawing undo |
| Activate a UI theme | User preference transaction with preview/revert; no CAD undo |
| Create a scene from a template | New-document operation, with explicit source bindings |

These are proposed contracts. The implementation must use the actual host document/session model available at that time.

## 2. Repository evidence and boundaries

### 2.1 Pinned baselines

| Repository/state | Inspected commit | Interpretation |
| --- | --- | --- |
| `0xlillith/solidcraft`, `main` | `807e367bdb03210dba188118fe000acd0a4b3725` | Baseline of the existing spike and personal fork. |
| `tangency-labs/solidcraft`, `main` | `514b0c84f9eb7655e3cdc986f0d44d4cd3416f4c` | Additional current upstream source evidence; not silently present in the personal baseline. |
| `tangency-labs/solidcraft`, `drawings` | `ebafe07487df5cdd108e091db939de765645e5eb` | Feature-branch reference, not main. |
| Personal Drawings preview | `b68fa4f868b8dd5ea312ecb5dfc8a0760dec4c5b` | Inspected paper, border and title-block foundations; some storage structure reflects an earlier design. |
| `0xlillith/solidcraft-render-catalog`, `main` | `df356e2f277b9bd0d1cc62553dac8208f1aae958` | Development inventory and current publication metadata. |

Branches and discussion decisions can move independently. The recommendations below identify which facts come from code, which come from coordination decisions and which are proposals.

### 2.2 Relevant implemented foundations

| Area | Verified source fact | Consequence for the proposal |
| --- | --- | --- |
| User data | `recovery::data_dir()` already selects an unversioned per-user data root on Windows, macOS and Unix. `SOLIDCRAFT_RECOVERY_DIR` affects recovery, not that general root. [R1] | Extend the data-root abstraction; do not invent executable-relative storage as the default. |
| Preferences | Native `settings_dir()` uses `eframe::storage_dir("SolidCraft")`; it is distinct from the engine data root. A newer upstream setting-directory override is not in the pinned personal main. [R2] [R3] | Record an asset-store locator in preferences; do not assume settings and payloads already share one directory. |
| Plugin inventory | The existing plugins page scans `{data}/plugins/<id>/plugin.json`; it is not an executable plugin host or pack installer. [R4] | Keep asset data and future executable extensions explicitly separated. |
| App updater | Upstream updater replaces an AppImage/executable, extracts selected Windows portable executables, launches MSI or opens DMG; Windows executable swaps have their own rollback handling. [R5] | Fits a separate user library, but does not certify pack update/rollback survival. |
| MSI | The inspected installer declares application payloads under Program Files. It does not define a future pack lifecycle. [R6] | Verify actual installer behavior; do not infer it solely from the desired folder. |
| Themes | Personal main supports `solidcraft-theme/1`, bounded tokens/radii/borders/icon palette and copied custom themes in preferences. [R7] | A real early adapter is possible; a generic theme engine is unnecessary. |
| Theme integration | Parsed theme identity is name-derived; same IDs replace one another. Upstream `import_theme` stores and selects immediately. [R7] [R8] | Add source-bound identity and separate installation from activation. |
| Browser | OPFS document loading reads top-level files into memory; persistence is requested without retaining its result. [R9] | Use a dedicated lazy asset backend, not the current document-loading path. |
| Browser updater | Service-worker caches belong to app builds and are swept by their app-cache namespace. [R10] | Asset payloads must not live in a disposable build cache. |
| Flatpak | The inspected manifest deliberately omits network permission. [R11] | Remote sources need a packaging decision; local file installation remains an independent route. |
| Drawings | The inspected feature code has paper/custom sizes, border definitions and typed title blocks; it does not demonstrate a complete installed-template workflow. [R12] [R13] [R14] | Reuse these semantic types only through actual supported consumers. |

### 2.3 Drawings storage requires special care

The inspected personal Drawings preview contains an earlier standalone drawing-file model. The later G3 coordination decision is to embed drawings in the parent `.solidcraft` design. The existence of earlier types does not make the earlier storage arrangement the correct future target. [R15]

The pack layer should resolve a template into a validated, self-contained drawing definition and submit it through the embedded-drawing receiver once that receiver is integrated and stable. It should not mandate `.scdwg`, hard-code an obsolete file layout, or create a second independent drawing store. The receiving Drawings work owns its save format, document embedding, sources and undo boundary. A decision in a discussion is not proof that all of its implementation and acceptance are complete.

### 2.4 The existing catalog is development input

The federated index reports **2,983 resource records**, explicitly including appearance variants and physical profiles rather than 2,983 distinct engineering materials. Its collections include the original catalog and later woods, alloys, printing, additive, earth and elements stages. It explicitly disclaims being a plugin/installer standard. [R16]

The original release manifest describes **23,161 source files and 10,079,019,285 expanded source bytes**. Six material archives are divided by sequential material-ID ranges; a seventh groups catalog, environments, setups, tools and other content. Their individual archive byte counts are recorded separately from source bytes. These facts explain the distribution history, not ideal runtime packaging. [R17]

Publication and integrity verification must remain separate from native acceptance and visual qualification. Repacking a resource cannot convert an existing `not_run` or pending qualification into a passed result.

## 3. Where installed packs belong

### 3.1 Compare the location options

| Location | Useful qualities | Failure modes/tradeoffs | Recommended use |
| --- | --- | --- | --- |
| Beside the executable | Obvious self-contained portable folder | Replacement/deletion of the install directory; read-only installations; duplicate downloads per app version | Explicit portable mode only |
| New top-level home folder | Easy to discover and back up | Clutters home; ignores platform conventions; can be accidentally synced or moved | User-selected location, not universal default |
| Hidden home folder | Familiar to some developer tools | Same cross-platform issues; often mixes settings, caches and durable data | No advantage over existing platform conventions |
| OS application-data root | Stable, per-user, normally independent of app payloads | Sandboxes and installer policies still matter | **Default for installed packs** |
| Configuration/preferences folder | Already available and small | Multi-gigabyte payloads complicate roaming, reset and backup | Store locators and choices only |
| Documents/authoring folder | Visible, editable, easy to version with user work | Often cloud-synced; mutable content should not masquerade as an immutable installation | Authoring and explicit folder sources |
| Downloads | Convenient import entry point | Cleanup routinely deletes it; filenames are unstable | Import from it, then copy into durable storage |
| Project directory/file | Portable and reproducible | Duplicates resources across projects; unsuitable for a global browsing library | Selected embedded resource closure or explicit project source |
| System-wide data directory | One read-only company/lab installation | Requires administrator ownership and clear update authority | Optional managed source/store with per-user overlay |
| External/local data disk | Space for large libraries | Missing drive, changed permissions, different durability guarantees | Supported move/select operation |
| Shared network/cloud-sync folder | Team distribution and backup convenience | Concurrent writes, partial synchronization, locking and offline problems | Read-only source first; no default shared mutable database |
| Browser-managed storage | Available without native filesystem installation | Origin scope, quotas, user clearing, browser profile loss | Dedicated web backend with export/restore |

An application update must not rely on the user remembering to copy folders manually. Portable mode is deliberately different: the data travels with its explicitly selected portable root, and deleting that root deletes the portable library. Other products document that distinction as a supported mode with its own update constraints. [S7]

### 3.2 Recommended platform mapping

Resolve platform directories through the project's central path abstraction and appropriate OS APIs. The following **asset subdirectories are proposals**, not installed paths observed today.

| Platform/distribution | Durable asset root | Configuration/cache policy |
| --- | --- | --- |
| Linux, ordinary native distribution | `$XDG_DATA_HOME/solidcraft/assets`, default `~/.local/share/solidcraft/assets` | Respect XDG config/cache/state distinctions and valid absolute overrides. |
| Windows, ordinary desktop distribution | Known LocalAppData directory, then `SolidCraft/Assets` | Keep bulk assets local; do not put them in roaming settings. |
| macOS, ordinary desktop distribution | User Application Support, then `SolidCraft/Assets` | Use native preference conventions; derived cache belongs under user Caches. |
| Flatpak or other sandboxed distribution | Its permitted application-data location or an explicitly selected accessible root | Do not promise access to another package/channel's private sandbox. |
| Browser | Dedicated OPFS subtree and transactional metadata | Separate application-build caches, document loading and asset payloads. |
| Explicit portable mode | Chosen portable root plus `assets` | Keep it independent of accidental current working directory; expose it clearly in Settings. |

XDG defines distinct data, configuration, state, cache and runtime roots; installed resources and authoritative receipts belong in data. Windows provides Known Folder identifiers and lookup APIs. Apple distinguishes supporting application data from disposable caches. These are platform location principles, not an unconditional uninstall-survival guarantee. [S1] [S2] [S3] [S4]

The existing helper uses environment-based locations. A future asset-root resolver should validate absolute paths and handle unavailable roots explicitly. If the proper user directory cannot be resolved, report the problem or request an explicit location; do not silently write beside the executable or into the process working directory.

### 3.3 Data classification controls deletion and backup

| Class | Examples | Clear cache may delete it? | Offline backup |
| --- | --- | --- | --- |
| Installed content | Descriptors, CAD, textures, fonts, HDR resources | No | Include the retained selection |
| Installation/activation records | Exact identities, receipts, profile generations, removals and pins | No | Include |
| User-created content | Customized finishes, studios, themes, drawing templates | No | Include or list external authoring locations |
| Source trust state | Trusted roots, key continuity, highest-seen signed metadata versions | No | Preserve and revalidate before new online resolution |
| Small profile preferences | Enabled sources, favorites, preferred versions, active theme reference | No ordinary cache cleanup | Include non-secret portable choices |
| Search/thumbnail index | Rebuildable metadata and derived previews | Yes | Optional |
| Renderer derivatives | Decoded textures, tessellation, device-specific caches | Yes | Usually omit |
| Uncommitted transfer/staging | Partial downloads and abandoned work | Under explicit recovery rules | Usually omit |
| Credentials | Tokens, passwords, authenticated-source secrets | No | Use credential-store recovery; exclude from shareable exports |

A source catalog's cached response can be disposable. Trust roots and rollback-protection records are durable security state. Treating both as one cache can turn a harmless cleanup into a trust reset. The first spike's signed-source recommendation still applies. [S8]

## 4. Storage, profiles and application updates

### 4.1 Share immutable bytes; isolate mutable choices

A storage-protocol version is different from a SolidCraft release number. A proposed `assets/store-v1` path describes the on-disk storage contract; `assets/0.21.0` would unnecessarily attach the library to one application release.

The recommended logical layout is:

| Proposed relative location | Responsibility |
| --- | --- |
| `stores/store-v1/store.json` | Stable store ID and supported storage-protocol major |
| `stores/store-v1/packages/<archive-digest>/` | Verified immutable package contents and original manifest |
| `stores/store-v1/receipts/` | Source-bound identities, admission results and provenance |
| `stores/store-v1/generations/` | Committed installed-set generations and removal/pin records |
| `stores/store-v1/trust/` | Durable source authority and update-security state |
| `profiles/stable/` | Stable-channel choices, compatibility cache and selected assets |
| `profiles/beta/` | Independent mutable beta profile |
| `profiles/<explicit-user-profile>/` | Optional named user workflows; not silently created per release |
| `authoring/` or user-selected folders | Editable originals, separate from installed releases |
| `staging/` | Incomplete operations excluded from discovery and use |

Caches may live in the OS cache root rather than inside this durable root. A store ID is independent of its absolute location. A profile stores the locator and expected store identity; filesystem paths are not asset identities and do not enter portable document receipts.

Stable and beta may share immutable content **only when** their storage protocols, permissions and locking are compatible. Sandboxed builds may need separate stores and explicit import/export. Sharing is an optimization, not a reason to bypass platform isolation. [S5]

The existing scalar application/version/capability axes remain separate from these new store/profile schema axes. A newer app can introduce support for another asset type without moving every installed byte.

### 4.2 On-disk representation options

The transport archive and installed representation do not have to match.

| Internal representation | Strength | Cost/risk | Position |
| --- | --- | --- | --- |
| Keep only ZIP and read members lazily | Compact, exact original archive retained | Repeated decompression, random access and decoder limits need careful handling | Valid alternative for an explicitly bounded implementation |
| Immutable extracted directory per archive digest | Simple resource reads; easy inspection and recovery | Duplicate common files across different packages | **Recommended first implementation** |
| Keep ZIP plus extracted files permanently | Easy offline redistribution and quick reads | Potentially almost doubles disk usage | Optional retention policy, not mandatory default |
| Per-file content-addressed blob store | Deduplicates identical resources across packages/versions | More GC, materialization, integrity and multi-profile bookkeeping | Add when measured reuse justifies it |
| One SQLite database containing all payloads | Transactions and indexing in one object | Very large files, backup contention and poor ordinary inspection | Use a database for metadata if useful, not as the public pack format |
| Loose editable resource folders | Excellent authoring experience | Silent mutation invalidates exact identities and approvals | Authoring source that builds/imports an immutable snapshot |

Start with content-addressed **package directories**, keyed by the verified archive digest, while keeping the resolver independent of physical layout. This deduplicates identical complete archives; it does not deduplicate identical files in different archives. A later per-file store can add that optimization without changing the public pack format.

**Exact archive retention:** extracting a ZIP and discarding it loses its exact container bytes. Re-zipping the same members generally produces a different archive digest because metadata and compression can differ. The installed receipt can retain the original archive identity and verified member inventory, but it cannot recreate the original ZIP. Retaining original archives for byte-exact offline redistribution is an explicit storage option; otherwise that operation needs the exact original artifact to be fetched again. A rebuilt archive receives a new release identity/digest.

Read-only flags help prevent accidental edits but are not a security proof against the owner modifying files. Retain hashes and validate at the appropriate read/admission boundary. Never execute authoring scripts found in a pack. A browsable folder should expose **Duplicate to edit**, not encourage modification of installed bytes.

### 4.3 Update behavior matrix

| Event | Required proposed behavior |
| --- | --- |
| Ordinary app update | Preserve store location, installed bytes, pins and profile choices; no unnecessary downloads. |
| App upgrade requiring metadata migration | Create and validate a new committed profile/store generation; retain the recoverable prior generation. |
| App rollback | Open a compatible profile/generation; retain newer unsupported content and explain limitations. |
| Stable and nightly at the same time | Separate mutable profiles; coordinated compatible store access; no in-place migration of the other's profile. |
| Pack update | Install a new immutable release alongside retained versions; never rewrite existing drawings/designs. |
| Change preferred pack version | Affect future selection/application only. Existing references stay exact. |
| Remove one app installation | Do not delete a shared store still used by another profile/installation. |
| Reset preferences | Reset only the selected preference scope; rediscover installed content. |
| Clear caches | Remove derived data; keep installed packs, originals, receipts and trust continuity. |
| Website disappears | Installed assets and properly embedded project content remain available. |
| Capability removed or renderer changed | Recompute availability; do not delete content or silently substitute a different asset. |
| Unsupported store major | Explain incompatibility or offer a separate compatible store; never auto-reset an unreadable library. |

An app rollback does not guarantee it can read a drawing saved in a newer document format. Library retention, adapter compatibility and document-format compatibility are three separate promises.

### 4.4 Migration and crash recovery

For profile/store metadata changes, identify versions first, acquire a migration lease, check space, construct a new generation, validate all references, commit it durably and retain the preceding committed generation. Do not rewrite the only readable copy in place.

Payload semantic conversion creates a derived representation with converter identity and output hash. It does not mutate the original package or make its former hash refer to new bytes.

The implementation needs a coherent transaction boundary for installation, activation, removal and profile selections. Rebuild a derived search index only from receipts reachable from a committed generation. Scanning every apparently valid directory can resurrect uninstalled or cancelled packages.

Garbage collection must account for all compatible profiles, retained generations, explicit pins, active reads, transfer/export jobs and recovery leases. Unknown/unreadable profile state is a reason to preserve potentially reachable data, not to assume it is unused. The system cannot discover every external project on every disconnected disk; explicit pins and embedded project resources cover that limitation.

Security state requires additional care. Rolling a profile back to an old generation must not roll signed-source trust state back below a newer accepted high-water mark. Keep the authoritative monotonic trust record separate from ordinary preference rollback. Restoring an old offline backup requires a documented revalidation policy before new network resolution, not an assertion that historic metadata is fresh.

### 4.5 Uninstall is a separate promise

For an ordinary desktop distribution, the proposed default is to retain user-created content and installed assets unless the user explicitly requests their removal. A clearly scoped **Remove all SolidCraft data** action can show locations and affected stores. It must not delete external authoring directories, project files or another profile's content by association.

However, managed Windows app-data stores are normally preserved through updates and removed at uninstall. MSIX virtualization can change where AppData writes actually live, and flexible-virtualization policy affects persistence. A path containing `LocalAppData` alone cannot establish the uninstall guarantee. [S6] [S24]

Test the actual shipped MSI/portable archive/AppImage/macOS/sandboxed distribution separately. No installer, manifest, capability or removal policy is changed by this spike.

## 5. Backup, relocation and browser persistence

### 5.1 Moving a library

Provide **Settings → Storage → Move asset library…** and **Open library folder**. These are proposed UI actions. A move must preserve the store's identity and exact installed content.

The safe sequence is to select and validate the destination, quiesce every affected compatible writer, pin the source generation, copy to a temporary destination, verify its inventory and receipts, make the destination valid, update a shared locator record or every affected profile locator, and reopen through the normal resolver. Mark the old root relocated/read-only before any cleanup so another compatible profile cannot continue writing a second copy under the same store ID.

If a profile or writer cannot participate safely, do not declare a shared move complete. Keep its old store and offer an explicit copy with a new store identity for the participating profile. Cleanup must respect the remaining references. A restore that intentionally creates an independent second library likewise needs a new store identity; asset identities remain unchanged.

A rename across disks is not an atomic operation. Check destination free space and filesystem behavior, retain the old store through interruption, and distinguish a completed copy from a completed switch. Never merge two existing stores by blindly copying directories over one another.

If the selected disk disappears, show **Library location unavailable** with reconnect/change-location actions. Keep the configured store ID and locator. Do not create an empty default library that makes installed assets appear to have been lost. Properly embedded documents can continue to work while that external library is unavailable.

For portable mode and external disks, record whether the filesystem supports the required locking, case handling and durability semantics. Only claim tested guarantees. A read-only library can remain useful, with a separate local writable store for new installations.

### 5.2 Two different exports

| Action | Contents | Actual guarantee |
| --- | --- | --- |
| Export library configuration | Sources, exact package restore list, profile choices and non-secret settings | Restoration depends on exact resources still being obtainable. |
| Back up library for offline restore | Committed receipts, selected installed bytes, license/provenance closure and user-created assets | The included content can be restored without its original website. |

A URL list is not an offline backup. Include the resolved versions and digests in either form. Mark external authoring folders or restricted resources that were omitted, with an explicit explanation of the resulting restore limitation.

An offline library backup can be a **logical store backup** of installed members and historical receipts. Rehash its included files on restore, and preserve the fact that its provenance came from a previous installation or user-approved backup. Do not claim fresh verification of the original publisher-signed archive when its bytes are absent. A shareable byte-exact pack/collection export requires the retained original archives, exact refetches, or a separately designed independently authenticated member-inventory contract. The first version should use original archives for that export rather than silently invent such a contract.

Capture one committed generation and pin its resource closure until the backup finishes. A live database needs a supported consistent backup method; copying only a SQLite main file while it is being written is not a general backup strategy. SQLite documents an Online Backup API and other supported approaches. [S9]

Restore verifies hashes, reconstructs committed state, rebinds portable source identities deliberately and leaves credentials to the platform credential mechanism. It does not automatically trust every source named in an imported backup. Installed offline content can remain usable while new signed network resolution requires refreshed trust/freshness validation.

### 5.3 Browser-specific behavior

Use the same public package semantics and a different storage backend. The browser store must not be represented as a native home-directory folder the application can always reach.

The web storage model is origin-scoped and quota-managed. Persistent storage can be requested but is not necessarily granted. Clearing site storage removes OPFS content; changing the origin can separate the application from its old data. A different path on the same origin does not by itself create an independent quota or storage boundary. [S10] [S11]

Recommended web implementation:

- A dedicated asset payload subtree in OPFS, with lazy reads and bounded decoding.
- Transactional metadata for committed generations, such as an IndexedDB backend.
- Separate stable/beta profile names when using one origin; explicit export/import when moving origins.
- A visible persisted-storage status and quota estimate in Storage settings.
- Handling of actual write/quota failures; an estimate does not reserve space.
- An exportable offline backup and restore path.
- No multi-gigabyte pack fallback into localStorage.

The audited web app currently loads top-level OPFS files into the document VFS. Keep assets out of that path. It also discards the outcome of `persist()`; a future installer must observe and report the result rather than treating the request as a guarantee. [R9]

OPFS payload writes and IndexedDB metadata do not form one automatic cross-API transaction. Write and verify immutable payloads first, flush/close as supported, then commit the reachable generation pointer. Recovery must distinguish orphan payloads from committed data and refuse incomplete referenced content without losing the preceding installation.

Database upgrades must coordinate with old tabs. IndexedDB has explicit version-change and blocked-upgrade behavior; surface an understandable close/reload request when required. An old build must not erase a newer database after a version error. [S12]

A service-worker update can replace application code while asset storage remains. Never put installed packs in the app-build cache namespace that the current worker sweeps. This is an architectural requirement to test, not demonstrated web-pack support. [R10]

### 5.4 Team, managed and cloud use

For the first version, a company can publish a private static catalog or distribute pack files through an existing file server. Clients install verified immutable copies locally. Authentication and access control remain with the source; credentials do not enter pack manifests or document receipts.

A read-only managed store plus per-user settings avoids granting every employee write access to a common database. Per-user selections must not silently replace a managed package with a same-named user package: authority and exact identities remain visible.

Directly syncing a live database through a consumer sync folder is not the initial team strategy. Sync immutable artifacts or deliberate backups; use an explicit conflict-aware profile synchronization feature only if later requirements justify it. Private company templates should not be published merely because the underlying generic paper pack is public.

## 6. What an asset, pack and collection mean

There are seven distinct concepts. Their separation is the main way to keep the system both understandable and extensible.

| Concept | Meaning | Example |
| --- | --- | --- |
| Resource | Exact bytes used by an asset | Normal map, HDR image, font, logo, geometry payload |
| Asset | Typed reusable item with stable identity and a host contract | Finish, title block, light, fastener family, UI theme |
| Variant | A meaningful supported choice within an asset/family | Screw diameter/length, approved colour, title-block language |
| Representation | Alternative encoding or fidelity for an intended item | 2K/4K maps; display mesh; cosmetic-thread or detailed geometry |
| Package/pack | Independently installable immutable versioned release | Metric fasteners; oak finishes; basic drawing templates |
| Collection/bundle | Curated package selections and optional choices | Workshop kit; company drafting kit; product-presentation kit |
| Catalog/source | Discovery, distribution and source/trust metadata | Curated, community, company or user-added source |

In user-facing text, **pack** can mean the installable unit and **collection** the convenient grouping. Internally, avoid calling a texture file a pack or confusing a bundle with an executable installer.

### 6.1 One archive need not contain only one type

A coherent studio pack may contain lights, a backdrop and a modest environment map. A coherent drawing pack may contain paper definitions, a frame, title blocks, a font and a drawing style. They belong together because they form a useful unit.

However, a finish pack should not require a demonstration vehicle. A fastener family should not require a studio. A drawing template should not require an application theme. Cross-type packages are permitted; unrelated dependency chains are not a desirable default.

### 6.2 Distinguish studio, setup and scene template

| Asset | Intended effect | Content it must not silently introduce |
| --- | --- | --- |
| Studio | Surround existing user geometry with owned lights/environment/backdrop/stage | Showcase vehicle, unrelated product model or changed material assignments |
| Scene setup | Apply declared camera/render/composition settings within supported scope | Unrequested replacement of project geometry or engineering metadata |
| Scene template | Start a document containing explicit objects, props or assemblies | Hidden references to the publisher's local files or accounts |

A demonstration scene can reuse a studio through exact dependencies. Its preview image is not a statement that the installed studio includes every object shown. Catalog cards should make that distinction clear.

### 6.3 Stable identity and edits

Continue using source authority, package ID, exact version, archive digest and asset ID for resolution. A display name, directory, browser tag or theme slug is not identity.

A semantic cross-reference can help the catalog group 2K/4K representations or related variants, but it cannot silently redirect an existing exact reference to another package. For the first contract, different downloadable quality archives retain different package IDs, as recommended in the first spike.

Installed releases are immutable. **Customize** creates an editable derivative in the user's authoring area with a new identity and a `derived_from` provenance relation. Publishing that derivative is an explicit operation. Updating its source pack never overwrites the user's work.

A change in representation may affect engineering behavior: an envelope, cosmetic-thread model and exact threaded model need declared roles. The UI must not promise equal topology, volume, mass or mating behavior merely because they depict the same catalogue item.

## 7. Choosing package boundaries

### 7.1 Alternatives

| Partitioning strategy | Where it helps | Main weakness | Recommendation |
| --- | --- | --- | --- |
| One complete mega-pack | Full offline mirror or archival export | Huge first download and update; unrelated capabilities and licenses | Optional export only |
| One pack for each broad type | Small early catalog | Becomes enormous as all materials/parts/environments accumulate | Use domains as catalog categories, not fixed size boundaries |
| Sequential ID ranges or equal archive sizes | Mechanical shipping workaround | Names are meaningless to users; splits related assets unpredictably | Replace the development layout |
| One pack per individual file/map | Maximum raw granularity | Breaks resource closure and creates a dependency puzzle | Reject |
| One pack per asset | Heavy HDRIs, large models, infrequently shared resources | Too many tiny downloads for scalar presets and family tables | Use selectively |
| One pack per family/subcollection | Users obtain related content together | Requires editorial grouping and measurement | **Default** |
| One pack per vendor/author | Clear ownership and update channel | A large vendor can span unrelated domains and rights | Useful boundary, subdivide by useful family |
| One pack per task/workflow | Convenient installation | Forces unrelated assets to update together if one archive | Prefer a collection of independent packs |
| One pack per resolution/fidelity | Clear download/memory tradeoff | Duplicate metadata/maps across tiers | Good first representation strategy with distinct package IDs |
| Thin metadata plus shared resource packs | Reuses large common payloads | More dependency and offline-resolution complexity | Introduce for demonstrated large reuse |
| Dynamically generated pack per user selection | Minimal selected payload | Server/build/trust/cache complexity; no stable prepublished archive | Possible future service, not the baseline |
| Object/chunk transport | Low repeated-transfer cost at large scale | Request scheduling, authentication, indexing and GC complexity | Measure before adding |

### 7.2 A practical splitting algorithm

Use the following decision order when turning the catalog into products:

1. **Legal/source boundary:** can these resources be redistributed and embedded under compatible, intelligible conditions? Keep private or restricted supplier resources distinct.
2. **Independent usefulness:** can a user apply the promised content without installing an unrelated domain?
3. **Semantic closure:** include all maps, units, profiles and required definitions needed for that item. Do not split one material into arbitrary channel downloads.
4. **Ownership and update cadence:** avoid binding frequently revised company title blocks to stable engineering tables or large textures.
5. **Capability boundary:** a lightweight scalar collection should not require an HDR renderer or a future drawing feature just because it shares an archive.
6. **Representation:** choose a sensible default runtime tier; make unusually heavy tiers explicit alternatives.
7. **Actual size and usage:** measure compressed download, installed bytes, decoded peak memory, number of files and observed selected subsets.
8. **Catalog grouping:** restore convenient broad browsing through tags and collections, not through a bigger mandatory ZIP.

A hard universal byte limit is a poor editorial rule. A 2 MiB package can be incoherent; a 300 MiB studio may be coherent and still need an explicit download choice. The initial targets below are **planning hypotheses**, not measured best values or format ceilings:

| Content class | Initial editorial hypothesis to measure |
| --- | --- |
| Scalar cards, small styles, tables and analytic presets | Group dozens or hundreds when coherent; aim for a few MiB to low tens of MiB. |
| Runtime texture collections | Start around 25–200 MiB compressed per useful subcollection/tier. |
| HDR environments or large exact models | One item or a small family; disclose download and decoded memory before installation. |
| Optional high-resolution/master data | Separate explicit downloads; do not silently select them in starter collections. |
| Browser default | Select against actual quota and decoder limits, with smaller alternatives. |

Changing these editorial targets must not change the public schema or delete existing pack identities.

### 7.3 Dependencies and partial installation

Use exact, source-bound dependency selections in the resolved install plan. A required reference has to resolve to an installed/selected compatible asset. An optional dependency must have a defined useful behavior when absent, not merely be marked optional in metadata.

Initially make the package the installation unit and the asset the application unit. The UI can select packages within a collection, but should not imply that choosing three entries from one ZIP saves most network bytes. Without a supported member-range or object protocol, it may still download the whole ZIP.

Two future approaches to selective contents are viable:

- Split a large collection into separately installable small packs and let the catalog present one collection.
- Define independently addressed optional resource companions, with exact resolved pins and clear completeness states.

Do not introduce an ambiguous "installed" state where an asset is listed as usable but some mandatory maps have never arrived. A browser can show remote metadata without claiming the asset is installed.

### 7.4 Bundles need ownership of installation reasons

A collection has its own version and a selection plan. Resolve the chosen members to exact package releases and digests before installation. Store the user's optional choices and why each package is retained: direct install, collection membership, dependency, explicit pin or managed policy.

Removing a collection removes that retention reason. It must not remove a pack the user installed directly or that another retained selection requires. Collection updates present added, removed and changed members. They do not automatically apply a new studio, create a sheet, change units or activate a theme.

If an optional member fails, report its failure and allow an explicit reduced plan. Do not claim the entire original selection installed successfully. For the first implementation, one approved plan has a coherent commit boundary; partially downloaded members remain uncommitted staging until the user approves a smaller valid plan.

## 8. A proposed catalog structure

The names below are **candidate products**, not a claim that these packs have been built, licensed or qualified. Every entry needs actual content review and a working host adapter before it is labelled usable.

| Catalog family | Example independent packs | Key boundary |
| --- | --- | --- |
| Parts / fasteners | Metric socket-head fasteners; metric hex fasteners; nuts and washers; inch fastener families | Valid dimensional tables, standard/edition metadata, host generator capability |
| Parts / motion | Bearings; shaft collars; spacers; couplings | Envelope/cosmetic versus exact geometry and supported placement |
| Parts / structures | Aluminium profiles; tubing sections; permitted fittings | Cross-section families and length choices; avoid unrelated assemblies |
| Parts / supplier content | A specifically licensed supplier family | Publisher/permission and SKU mapping; never assume download means redistribution |
| Physical materials | Aluminium/alloy cards; polymer/process cards; timber cards | Conditions, units, source values and uncertainty; no mandatory textures |
| Finishes / metals | Machined metals; brushed metals; anodized finishes | Appearance/process semantics independent of engineering properties |
| Finishes / coatings | Paint and powder-coat looks; ceramic looks; supported wear treatments | Cosmetic versus physically modeled coating thickness |
| Finishes / woods | Oak board finishes; walnut board finishes; broader timber palettes | Grain orientation, physical scale and finite-board versus seamless semantics |
| Finishes / manufacturing | FFF surface studies; resin surface studies; additive-metal appearances | Process/direction qualifiers; no implied simulation or certification |
| Lights | Analytic product-light presets; permitted photometric families | Physical units, supported light/profile types |
| Environments | Neutral studio environments; daylight environments; individually heavy HDRIs | Illumination/reflection/camera-background roles and resolution |
| Studios | Neutral product rigs; metal-product rigs; small-object rigs | Owned lighting/stage composition around existing geometry |
| Scene setups/templates | Presentation camera setups; original demonstration scenes | Settings-only versus actual starting geometry |
| Drawings / basics | Paper formats; basic frames; general title blocks | Physical sheet/layout and metadata bindings |
| Drawings / styles | Supported annotation styles; table layouts; drawing symbols | Host semantics and exact print output; standards review status |
| Drawings / company | Private brand/title-block compositions | Private identifiers, logo/font rights and approved notes |
| Themes | A coherent light/dark theme family; high-contrast variants | Presentation preferences and supported token contract |
| Starter collections | Workshop starter; product-render starter; drafting starter | References the above; optional members do not create unrelated dependencies |

### 8.1 Physical material and finish separation

An engineering material card should remain usable without a large appearance library. A finish should apply to a user's existing geometry without installing an engineering database. The catalog can suggest a compatible pairing while letting the user inspect each source and meaning.

A default visual appearance may be a small inline definition in a material/parts pack if needed for a useful standalone experience. Its resolved definition is copied when used, rather than depending indefinitely on a future version of the application's unnamed default palette.

Likewise, a coating's appearance is not evidence of its actual thickness, adhesion, corrosion performance or changed mass. Those require explicit supported semantics and data. This preserves the first spike's distinction between physical properties, surface appearance, geometry and render effects.

### 8.2 Parts families rather than millions of duplicates

For supported built-in generators, distribute reviewed parameter tables and family metadata rather than one redundant model file for every screw length. Each permitted combination needs clear units, bounds, seating/origin conventions, thread designation, length definition and BOM/supplier fields.

Use prebuilt exact geometry where the host cannot generate the required item and the payload has appropriate rights. Keep optional display meshes distinct from authoritative geometry. A pack must not install a new native/WASM generator merely because its descriptors name one. That is a separate executable-plugin decision.

Supplier catalog data and drawings require their actual distribution permissions. The previous McMaster analysis remains applicable; this redesign does not authorize republishing a supplier's restricted files. Use original or appropriately licensed family data and distinguish supplier SKU from SolidCraft asset identity. [S13]

### 8.3 Catalog metadata and discoverability

Useful filters include domain, intended task, supported operation, material/process family, physical units, approximate/exact representation, texture scale/resolution, source, license, language, installation size and qualification status.

Tags improve discovery but do not establish identity, trust, dimensional conformity or compatibility. A "steel" tag cannot assign density. An "ISO" tag cannot certify a fastener or drawing. A "tested" badge must identify the tested app/backend/version and acceptance scope.

Use small preview variants. Show whether a preview includes unbundled demonstration objects. A card can group several sizes/resolutions while its install choices remain explicit and immutable.

### 8.4 Growth beyond the first domains

The same envelope can later carry supported decals, LUTs, lenses, profile sections, sheet-metal presets, stock definitions, fixtures, motion demonstrations, calculation references or documentation examples. New kinds require schemas, validation bounds, host consumers, persistence behavior and UI meaning.

Keyboard shortcut presets, workspaces, automation macros, CAM postprocessors and executable commands deserve separate capability and activation contracts. Do not label them harmless asset data merely because they can be stored as JSON. They can change behavior or produce machine actions and are not required for the proposed first asset-pack release.

## 9. Distribution and hosting alternatives

### 9.1 Keep the contract open and the delivery replaceable

Use the first spike's ordinary ZIP/root-JSON proposal for offline exchange and straightforward installations. Keep public schema versions, explicit file inventory and hashes, no installation scripts, clear license records and the same bounded validation for local and remote sources.

The package resolver should depend on exact identities and resource access, not on a permanent GitHub URL layout. A verified mirror can deliver the same approved bytes; a new source claiming the same name cannot inherit authority automatically.

| Delivery option | Advantages | Disadvantages | Recommended role |
| --- | --- | --- | --- |
| ZIP plus static catalog | Easy to inspect, mirror, host and transfer offline | Whole-archive download; publishers must build releases | **Baseline** |
| Local directory source | Easy personal authoring and company folder workflows | Mutable files need snapshotting and validation | Supported explicit import/build path |
| Direct individual asset files | Minimal workflow for a theme, SVG or model | Missing package provenance/dependency metadata | Keep existing import tools; offer explicit "create/import pack" separately |
| Git checkout | Excellent text history and collaboration | Git/LFS/tooling burden, large histories, unsafe assumption that repository files are installable | Publisher authoring, not normal end-user installation |
| OCI artifact registry | Existing registry/distribution conventions | Registry tooling and authorization complexity for ordinary creators | Optional future adapter, not required ecosystem infrastructure |
| Hosted marketplace/service | Search, accounts, payments, reviews and private entitlements | Operational burden, privacy and service dependence | Optional catalog frontend; preserve local/offline pack installation |
| Content-addressed object service | Fetch exact selected blobs and deduplicate transfer | Request/auth/index complexity; many small objects | Later transport when measurements justify it |
| Binary deltas/chunking | Smaller frequent large updates | More attack/recovery paths and source-version requirements | Later optimization; always allow full exact artifact retrieval |
| Peer-to-peer/local-network sharing | Useful lab/offline transfer | Discovery, permissions, integrity, licensing and support costs | Future explicit transport, same validator and source policy |
| Offline collection export | Air-gapped transfer and complete restore | Larger files, rights restrictions and freshness distinctions | Useful supported export with exact inventory |

A byte-exact collection export uses the original retained/refetched package artifacts. A logical library backup is a different format and must identify its historical receipts as such; neither is permission to rebuild an archive under its former digest.

A portable offline collection export should be an explicitly specified transfer format if it contains several complete packages. The generic pack reader must not recursively discover and install arbitrary nested ZIPs. A transfer reader validates its bounded top-level inventory and named child packages through the ordinary pack validator.

OCI and interchange formats remain alternatives discussed in the first report. They do not replace Drawings field semantics, package identity, update policy or user-data persistence.

### 9.2 Where to host the catalog and binaries

For the current GitHub-oriented project, keep small descriptors, schemas, publisher configuration and authoring code in Git. Publish a small static catalog/previews site and place immutable runtime archives on release assets or appropriate object storage/CDN.

The current official GitHub limit is under **2 GiB per release asset** and up to **1,000 assets per release**. Published GitHub Pages sites have a **1 GB** size limit. These are hosting constraints, not the ZIP profile's theoretical capacity or recommended pack size. [S14] [S15]

Do not serve the entire material corpus from Pages or rely on a single ever-growing release forever. Start with a release grouping that is easy to maintain, and rotate/paginate catalog publication as needed. The catalog can point to objects on more than one approved host without changing package identity.

For browser installation, verify actual CORS and redirect behavior of the chosen download path. A URL that native HTTP can download is not automatically usable through browser fetch. Do not propose a public unauthenticated proxy merely to bypass a provider's policy. Use a supported host/configuration or the explicit download-and-import route.

### 9.3 Publish atomically and preserve trustworthy identities

A publishing pipeline should build and validate immutable payloads, upload them, verify their exact downloadable bytes, and only then publish catalog metadata that advertises them. Signed metadata must bind the catalog/payload identities according to the chosen trust framework.

Keep dependency closure and source authority in the resolved plan. Stage all required content before activation. Do not follow a newly encountered dependency URL as an automatic trust grant.

A pack can be withdrawn from new discovery without erasing already saved document content. Security withdrawal, license withdrawal, ordinary deprecation and replacement are different states with different user messages. Installed content remains inspectable; future application may be restricted by an explicit security policy rather than silently substituted.

### 9.4 Deduplication is not one feature

| Optimization | What it reduces | What it does not automatically reduce |
| --- | --- | --- |
| Same archive digest stored once | Duplicate complete installations | Similar but non-identical packages |
| Per-file blob deduplication | Repeated identical installed resources | Bytes already downloaded in two different ZIPs |
| Shared exact resource package | Repeated downloads if dependency already installed | First download or changes in resource bytes |
| Object transport | Transfer of already held identical objects | Decoding/memory or semantic compatibility |
| Binary delta | Transfer between suitable known versions | Need for recovery, verification and a full fallback |
| Derived GPU cache | Repeated conversion/decoding work | Original source storage or cross-device compatibility |

Begin with meaningful package boundaries and measure both network bytes and installed/decoded cost. Do not claim that adding hashes makes all downloads deduplicate.

## 10. Rebuilding the development catalog

### 10.1 Scope of the redesign

The present archive layout may be replaced. Keep the original authored content and the evidence needed to understand it, while producing a new runtime-oriented catalog. Do not spend the first milestone building a permanent importer for every historical packaging experiment.

This spike does **not** delete, rewrite or republish any existing release. Rebuilding and releasing actual packs is a subsequent implementation/content task, with review of the resulting artifacts.

### 10.2 Proposed authoring organization

Authoring organization and published package grouping should be related through explicit build definitions, not inferred from physical directory boundaries.

| Proposed repository path | Purpose |
| --- | --- |
| `schemas/` | Public envelope, type and catalog schema sources |
| `assets/physical-materials/` | Source-backed material cards and provenance |
| `assets/appearances/` | Appearance definitions and material-scale semantics |
| `assets/finishes/` | Supported finish treatments and masks |
| `assets/parts/` | Reviewed family tables, placement/BOM metadata and original/permitted models |
| `assets/render/` | Lights, environments, backgrounds, studios and scene templates |
| `assets/drawings/` | Paper, frames, title blocks, styles, symbols and template compositions |
| `assets/themes/` | Theme variants and their supported native token files |
| `resources/` | Shared authoring resources, referenced explicitly by build definitions |
| `packages/` | Package membership, default tier, capability and rights policy |
| `collections/` | Curated installation selections and optional groups |
| `provenance/` | Origin mappings, measurements, licences and review evidence |
| `tools/` | Offline authoring/build/validation tools; never installed as runtime hooks |
| `distribution/` | Generated catalog/release metadata, not a second editable source of truth |

This is a proposed publisher layout. Large originals can live in appropriate external storage referenced by exact hashes; committing all master binaries into Git is not required. Different publishers can use different authoring tools while emitting the same public pack contract.

### 10.3 Mapping the existing inventory

| Existing input | Proposed restructuring | Information to retain |
| --- | --- | --- |
| Sequential original material archives | Domain/process/subcollection runtime packs | Old asset IDs as aliases, source hashes and exact per-map semantics |
| Woods stage | Species/board-family runtime appearances, separate physical cards, separate master downloads | Physical board extent, grain direction and finite patch behavior |
| Alloys stage | Engineering cards separated from finish families | Grade/temper/source conditions; visual variants are not extra alloy grades |
| Printing/additive stages | Process-oriented surface packs and independent material/process cards | Print orientation, process conditions, shared-map provenance and limits |
| Earth/elements stages | Coherent material/appearance families after review | Original classification, uncertainty and status |
| Environments/lights/setups mixed with tools | Dedicated environment/light/studio packs; tools remain authoring-side | Lighting/background roles, ownership and supported runtime contracts |
| HTML galleries, reports, source scripts | Documentation/source downloads and catalog previews | Authorship and evidence; no automatic execution in the app |
| Small existing native scalar libraries | Supported direct import or useful new runtime pack | Exact legacy revision semantics when an actual consumer needs migration |

A stable identifier map can be one-to-one, one-to-many or intentionally unmapped. For example, one early "material" record may become a physical card and a separate appearance. Record that split by semantic type; do not redirect a density assignment to the appearance just because it shares the old label.

### 10.4 Repack pipeline

1. Freeze a source inventory revision and compute exact original hashes.
2. Classify each resource by semantic type, ownership, rights, current capability and qualification status.
3. Identify actual consumers: existing documents, native libraries and any real installed packs.
4. Define the new stable asset IDs and explicit origin/alias mapping.
5. Build package membership from useful standalone closures, separating runtime and master data.
6. Generate each quality/fidelity product deliberately, retaining source scale and conversion provenance.
7. Validate structure, dependencies, licenses and bounds; run only applicable native acceptance for adapters that really exist.
8. Produce the immutable runtime archives, previews and catalog entries.
9. Test actual download/install/use/offline-reopen journeys before marking a pack ready.
10. Publish a migration/release note distinguishing redesigned content from unchanged historical artifacts.

Do not preserve accidental filenames merely to avoid a clean redesign. Do preserve meaning and exact historical references needed by saved user work. If no actual user installation depends on an old archive layout, no automatic legacy runtime importer is necessary.

Once a stable public pack format is declared, introduce ordinary compatibility/deprecation rules. Development freedom does not justify changing bytes behind the same stable package/version/digest or silently rewriting documents.

### 10.5 Content quality is a separate release gate

Preserve the distinction among authored, structurally valid, source-verified, published, native-tested, visually reviewed and engineering-qualified. A reorganized pack may be installable while some content remains unsupported for application.

A preview can communicate intended appearance, but it is not runtime acceptance. Source-derived physical data needs conditions and uncertainties; package signatures authenticate distribution, not the physical correctness of a density or the dimensional conformity of a fastener.

## 11. Drawings packs

### 11.1 A drawing template is a composition

Use typed reusable elements so a user can choose a company title block without replacing every paper size or style. Separate semantic types do not imply a separate archive for each cell or line.

| Proposed type | Responsibility | Typical package grouping |
| --- | --- | --- |
| `paper_format` | Physical dimensions and supported orientation | A small paper-format collection |
| `drawing_border` | Margins, frame, zones and marks | Frame family designed for related sheet sizes |
| `title_block` | Typed fields, bounded layout and projection marker | Generic or private company title-block family |
| `drawing_style` | Paper-space line/text/annotation appearance | A coherent supported drafting style |
| `sheet_template` | Paper, border, title block and optional bounded slots | A useful sheet family |
| `drawing_symbol` | Supported semantic/graphical symbol | Domain-specific symbol collection |
| `table_template` | Supported BOM/revision/hole-table layout | Table styles used with real host data |
| `drawing_preset` | Explicit supported drawing defaults | A small workflow preset collection |
| `branding_profile` | Logo, identity assets and explicit metadata defaults | Private company overlay/composition |

A generic template must not depend on a private company package. A private company composition can depend on a public generic base. A drafting starter collection can combine paper, basic title blocks and styles without pulling in a renderer or UI theme.

### 11.2 Reuse existing types without overstating completion

The inspected feature code already represents standard paper sizes and `SheetSize::Custom { sides_mm }`, with a separate orientation and bounded dimensions. A pack can provide a useful name and metadata around a custom physical size; it does not need to extend a compiled enum for every publisher's paper. The feature branch also has a validated `DrawingBorderProfile`. [R12]

The inspected saved-sheet model copies its border profile, which is the right direction for keeping old drawings stable when built-ins change. The typed geometric title-block definition has bounded cells and layout, but the inspected sheet record does not yet provide a full installed title-block/template application path. Geometry types alone do not prove that UI, persistence and output support are complete. [R13] [R14]

G3's embedded-drawing decision and the unmerged receiver/storage work must guide integration. As checked for this spike, #7098 merged the earlier standalone-storage slice into the `drawings` feature branch, not `main`. The embedded-storage implementation in #7490 (`472a330691f89fe58e90e65665bd035927c07ca0`) and its command receiver in #3891 (`36cf48a3f64c530a128f5d794f37b7cfa24cc3ab`) remain open drafts. A future pack adapter should apply through the stabilized drawing receiver of the parent design, retaining the drawing family's own versioning. [R15] [R18] [R20] [R21]

The pack format must not impose a second save format or introduce model paths from a template author's machine. A template applies to the parent design and selected drawing context. External-model drawing references are a separate future capability.

### 11.3 Paper, sheet layout and output are different

Keep sheet size, orientation, border margins, optional filing margin, drawing area and actual printer printable region separate. The last of these is a property of the selected output device, not a permanent property of A4 or a company paper asset.

Define geometry, text height and line widths in explicit paper-space units. A model view's scale is independent of those paper-space dimensions. The user's screen DPI, UI font scaling and dark-mode setting must not resize the printed sheet.

A4 and A3 variants of the same template family need intentional layout or a bounded adaptive layout rule. Scaling the entire A4 title block, lettering and line widths uniformly is not a general conversion policy.

Public ISO descriptions separately cover sheet sizes/layout and title-block/header fields. Record relevant designations and editions as reference metadata; do not claim complete standards conformance from a matching filename or public abstract. [S16] [S17]

Likewise, public paged-media documentation distinguishes page size, margins and orientation, but CSS Paged Media Level 3 is a Working Draft, not a CAD drawing standard. It is useful background, not a reason to adopt a browser layout engine as the drawing contract. [S18]

### 11.4 Title-block bindings must be typed and bounded

A field definition needs a stable ID, supported data source, data type, scope, localization label, formatting policy, required/optional status, explicit override policy and overflow behavior.

| Field | Recommended semantics |
| --- | --- |
| Drawing title/number | Stored document/drawing metadata; required missing values are reported. |
| Author | Explicit stored value; a creation-time suggestion is not a live account lookup. |
| Issue date | Stored value or deliberate export-job input; do not change it on every reopen/render. |
| Revision | Drawing's recorded revision, not the package version. |
| Sheet index/count | Host-derived from the actual drawing set. |
| Scale | Host-derived supported scale summary; mixed-scale sheets need truthful handling. |
| Projection method | Actual drawing setting and supported symbol, not decorative artwork that can disagree. |
| Approval/signature | Actual explicit approval data; blank/unapproved by default. |
| BOM | Host-generated content from the intended model/assembly context. |

Do not let a title block read arbitrary files, environment variables, credentials, network endpoints or another open project's data. A template expression language, if needed, should be a bounded host-defined selector/formatter, not general code.

Long drawing numbers, translated labels and missing glyphs need explicit behavior. Allow bounded wrapping or an alternate layout; do not silently truncate identifiers or shrink critical text until it becomes unreadable. Save the intended locale/date/number formatting policy where it affects issued output.

A template must not manufacture approval by shipping a filled signature or a hard-coded issue stamp. User-authored defaults such as a company address should be clearly distinguishable from a binding to document data.

### 11.5 Template representation options

| Representation | Strength | Limitation | Recommended role |
| --- | --- | --- | --- |
| Native declarative layout/vector data | Typed fields, deterministic geometry, bounded validation | Requires authoring tools or human-readable schema | Canonical runtime contract |
| Restricted SVG plus typed JSON bindings | Familiar editable artwork and interchange | Full SVG includes more capabilities than needed | Optional authoring/import format compiled into supported native data |
| A native drawing used as a template | Reuses existing editor/workflow | Can accidentally include source models, issue history and external/private state | Useful export workflow after explicit sanitization and dependency capture |
| PDF underlay | Reuses fixed forms visually | Weak field semantics/editability; fonts/page interpretation | Later explicit static import, not canonical template semantics |
| Raster image underlay | Simple and widely readable | Poor scaling/editability and no typed metadata | Background artwork only |
| HTML/CSS/JavaScript | Familiar web tooling | Larger execution/layout surface and inconsistent physical-output assumptions | Exclude as the v1 runtime |

SVG is not automatically inert. Its specification defines a secure-static processing mode without scripts, external references, animation or interaction. SolidCraft should additionally define a small accepted graphical subset, resource limits and physical coordinate mapping. [S19]

Reject scripts, event handlers, `foreignObject`, remote images/fonts, unrestricted filters and unsupported references. Bound internal reference expansion, path complexity, text runs and decoded image sizes. Installation does not grant the template permission to fetch missing resources.

### 11.6 Reproducibility and project ownership

When applying a template, preserve the exact layout/style definition, selected asset identity, permitted fonts/artwork, field bindings and values, conversion provenance and rendering-relevant units/locales. Keep those authoritative dependencies with the project according to the receiving design/Drawings storage contract.

Updating or uninstalling the source pack does not restyle an existing drawing. **Update template** is an explicit operation with a semantic diff, field mapping, layout preview and one appropriate drawing undo transaction. User-edited text and overrides must not be silently replaced.

An issued/exported drawing should record enough intent and dependencies for controlled reproducibility. Do not promise byte-identical PDF across arbitrary exporter, shaping-engine or platform versions; some metadata and implementation details can differ. Preserve the authoritative drawing and allow the issued output to be archived separately.

Actual-size print, export at defined physical size and fit-to-page are different operations. If a print job scales the sheet, report the effective output scale; do not let an unchanged scale label imply a physically true-size print. Clipping and printer margins require preflight.

### 11.7 Fonts, logos and private branding

Fonts are both layout dependencies and licensed resources. OpenType has embedding-rights fields, but those flags do not replace the applicable font licence. A font available on the author's desktop is not automatically redistributable in a public pack. [S20]

For public packs, prefer clearly redistributable fonts and retain required notices. The OFL documentation permits relevant bundling uses subject to its conditions, including appropriate licence information and applicable modification requirements. [S21]

If a font cannot be included, offer a deliberate supported substitution with re-layout/review, an explicitly restricted external-font workflow, or another permitted output strategy. Do not silently substitute and claim that the drawing is unchanged. Converting text to outlines also needs consideration of rights, accessibility, searchability and geometry size; it is not an automatic permission bypass.

A private branding pack may include a company logo, approved notes, title-block variant, form identifiers and appropriately licensed fonts. Export must not leak credentials, internal catalog URLs or unrelated company metadata. Keep required provenance in the project/controlled package while offering a deliberate public-output metadata policy.

## 12. Theme packs

### 12.1 Build on the native theme contract

The personal baseline already supports `solidcraft-theme/1`, with a dark/light base and named color overrides, bounded radii, border roles and icon-palette roles. Its documented implementation limits include 256 KiB per theme file, 512 color entries and 64 stored custom themes. Imported themes are copied into preferences, including for the browser. [R7]

Use that payload through a pack adapter initially. Do not require CSS, scripts or a second general-purpose skinning engine. Typography, arbitrary icon replacement, widget layout and keyboard shortcuts are not implied by the existing theme-file contract.

Two specific integration changes are necessary:

1. **Identity:** package source/asset/version/digest must distinguish themes whose human names slug to the same ID. The current name-derived slug and replace-on-ID behavior are not sufficient for multiple publishers.
2. **Activation:** installing a pack makes its themes available; it does not call the current import-and-select path blindly. The installed inventory can contain more themes than are currently materialized/selected in preferences. Respect existing limits until an explicitly designed replacement exists. [R7] [R8]

A theme update creates a new selectable version. It should not replace the currently active theme without an explicit user choice or a separately documented opt-in policy.

### 12.2 Scope and precedence

| Scope | Meaning | Should a UI theme control it? |
| --- | --- | --- |
| Application chrome | Panels, controls, text, focus and supported icon colors | Yes, through documented semantic roles |
| Viewport presentation | Grid/background/selection aids where supported | Only within explicit host roles and visibility rules |
| Engineering meaning | Constraint status, warnings, geometry, materials | No semantic changes; preserve additional cues |
| Drawing output | Paper color, technical line weights, annotation font/size | No; separate saved drawing style |
| Keyboard/workspace behavior | Shortcuts, commands, tool layouts | Separate preset/capability contract |
| Executable features | Macros, scripts, native/WASM modules | Separate plugin system |

Resolve the theme against stable built-in defaults, then apply explicit user overrides and host accessibility policy according to a documented precedence. A pack must not defeat required focus/recovery controls or erase the meaning of a warning. Keep color-vision and font-scaling preferences under user control.

### 12.3 Activation and recovery

Offer **Preview**, **Apply**, **Revert** and **Restore built-in theme**. Keep last-known-good settings and a reliable built-in recovery path at startup. Preview must not become an irreversible persistent choice after a crash or abandoned dialog.

On removal of an active theme, either materialize the supported current preference snapshot with provenance or ask the user to choose a replacement. The first release should use a simple explicit replacement/revert flow. Do not leave a dangling active reference or change an open CAD/drawing document.

Customization produces a user-owned derivative. Preserve its source relation, but do not overwrite it when the original theme updates. Light/dark/high-contrast variants can share one coherent package; their selection is still a preference action.

### 12.4 Interchange and validation

DTCG 2025.10 is a stable community design-token exchange specification and explicitly is not a W3C Standard. Its dimension type uses px/rem, which must not become the unit system for technical drawing templates. It is suitable as an optional bounded theme import/export profile, mapped to SolidCraft's supported roles. [S22]

A converter must state supported token types, aliases, color spaces and unit mapping; detect cycles and unsupported required roles; and produce the existing native payload or a deliberately versioned successor. Parsing a DTCG file is not evidence that every token has a working UI consumer.

Evaluate resolved/composited colors, focus, enabled/disabled states, error/selection cues, high-DPI behavior and fallback. WCAG 2.2 provides useful public contrast targets: 4.5:1 for ordinary text, 3:1 for large text, and 3:1 for relevant non-text interface information, with stated exceptions. These can inform supported theme checks, but color validation alone is not whole-application accessibility certification. [S23]

Keep exported drawing bytes/physical layout independent of theme and UI DPI in acceptance tests. A dark editor can surround a white sheet without changing the actual paper/ink style.

## 13. UI and command behavior

### 13.1 A shared installer with contextual entry points

Provide one proposed **Asset Library** with Browse, Installed, Collections, Sources and Storage actions. Add contextual **Get more…** actions in parts, appearances/finishes, render studios, Drawings templates and theme preferences. The same installer and receipts serve each route.

The user's immediate action should match its context: **Insert part**, **Apply finish**, **Apply studio**, **Create sheet**, **Apply title block**, **Preview theme**. Avoid a generic button whose effect changes application preferences, document geometry or drawing settings unpredictably.

The Installed view should distinguish installed bytes, compatible entries, explicitly unavailable entries, optional resources not selected, preferred version, pins and active preference selections. A package can be retained while a particular asset is unavailable in the current application.

### 13.2 Proposed journeys

| Journey | Expected experience |
| --- | --- |
| First local install | Open/drop a pack, see author/contents/size/compatibility, approve the concrete install plan, then choose an action. |
| Install a collection | Select required/recommended/optional members and tiers; see exact total download and storage plan; installation leaves the open project unchanged. |
| Add a source | Choose native catalog, supported provider or website shortcut; see source identity and policy before trust is recorded. |
| App updated | Existing library is immediately visible; a necessary migration is explained and recoverable. |
| Pack update available | See changes and retained versions; choose whether future selections prefer the new release. |
| Update an applied template | Preview layout/field differences; explicitly apply to the selected drawing and preserve undo. |
| Change theme | Preview and apply in preferences; retain last-known-good recovery. |
| Library too large | Inspect installed versus cache size; remove optional retained versions or move storage. |
| External drive absent | Keep the library identity, explain unavailable location and offer reconnect/reselect. |
| Backup for another computer | Choose offline backup versus configuration-only export and see omitted restricted/external resources. |

Source trust, structural validity, compatibility and content quality need distinct labels. Avoid a single "verified" badge that could mean any of them. Technical hashes belong in Details and troubleshooting; ordinary installation should present useful names and understandable consequences.

### 13.3 File import and developer folders

A compatible `.zip` and proposed `.scpack` use the same manifest-based admission. An ordinary ZIP containing arbitrary images is not silently treated as a pack. Existing single-file import flows for models/themes remain useful; they can offer an explicit later export-to-pack action.

A linked development folder is mutable. Mark it as such and snapshot/validate it before applying content. Do not make opening an authoring folder execute its build tools. A host-controlled **Build pack** workflow can be a separate authoring utility with explicit inputs and output.

For local self-contained packs, no network is necessary. If a pack requires absent dependencies, present those exact dependencies and sources before any download or refuse with an actionable message. Opening a file is not consent to contact every embedded URL.

### 13.4 Commands and destinations

Keep the commands/headless path and interactive UI consistent. Programmatic calls should receive explicit parameters and never open dialogs. New names below are illustrative command families, not claims of registered commands.

| Proposed capability | Destination/behavior |
| --- | --- |
| `assets.inspect`, `assets.plan_install` | Read/validate/plan; no activation |
| `assets.install`, `assets.remove`, `assets.pin` | Store/profile transactions |
| `assets.sources.*` | Source configuration and explicit trust policy |
| `assets.storage.*`, `assets.backup.*` | Location, size, consistent export/restore |
| Domain application commands | Use existing design/parts/render receivers where supported |
| Drawing template application | Use the stabilized drawing receiver/undo boundary |
| Theme availability/activation | UI preference consumer; shared validation without forcing egui into lower layers |

The public data contract can be shared while interpretation remains in the correct layer. In particular, do not move egui token rendering into the geometry/document crates to make a generic asset command work. The first spike's layering, bounded validation and no-script policy remain applicable. [R19]

## 14. Draft examples and a sizing experiment

The following examples are design aids. The first two use explicit `proposal-1` tags and are not accepted runtime schemas or installable packages. They illustrate responsibilities; the first spike remains the place for the complete draft pack manifest and its hashing rules.

### 14.1 An editorial collection definition

This collection references independently versioned package products. At installation time the catalog resolver must bind each chosen product to an exact approved source, version and digest. This editorial definition alone is not a lockfile, trust grant or permission to apply anything.

```json
{
  "format": "solidcraft-collection/proposal-1",
  "id": "io.github.0xlillith.collections.workshop-starter",
  "version": "0.1.0",
  "name": "Workshop starter",
  "members": [
    {
      "package": "io.github.0xlillith.parts.metric-socket-head",
      "role": "required",
      "selected_by_default": true
    },
    {
      "package": "io.github.0xlillith.finishes.machined-metals-2k",
      "role": "recommended",
      "selected_by_default": true
    },
    {
      "package": "io.github.0xlillith.drawings.basic-sheets",
      "role": "recommended",
      "selected_by_default": true
    },
    {
      "package": "io.github.0xlillith.render.neutral-product-studios",
      "role": "optional",
      "selected_by_default": false
    },
    {
      "package": "io.github.0xlillith.themes.workshop",
      "role": "optional",
      "selected_by_default": false
    }
  ]
}
```

The UI should adjust recommended selections when unsupported, explaining that change. A required member that cannot be resolved or used makes this particular collection plan unsuitable; it does not justify silently installing a different family. The resolved installation record includes source authority, exact versions/digests, choices and retention reasons. Package-name lookup must remain within the explicitly approved source/dependency policy.

### 14.2 A paper-space template fragment

This deliberately small layout illustrates physical units and field binding, not a full drafting standard or production template. Its frame uses 10 mm margins on a 297 × 210 mm sheet. The title block and suggested view slot fit within the frame without overlap. A complete asset would also identify its style, permitted fonts and supported rendering contract.

```json
{
  "format": "solidcraft-sheet-template/proposal-1",
  "id": "workshop-landscape-example",
  "units": "mm",
  "coordinate_system": {
    "origin": "sheet_bottom_left",
    "x": "right",
    "y": "up"
  },
  "paper": { "width": 297, "height": 210 },
  "frame": { "left": 10, "right": 10, "top": 10, "bottom": 10 },
  "title_block": {
    "origin": [107, 10],
    "size": [180, 40],
    "fields": [
      {
        "id": "drawing_number",
        "source": "drawing.metadata.number",
        "required": true,
        "overflow": "error"
      },
      {
        "id": "revision",
        "source": "drawing.metadata.revision",
        "required": false,
        "overflow": "error"
      }
    ]
  },
  "view_slots": [
    {
      "id": "main_view",
      "origin": [10, 55],
      "size": [277, 145],
      "purpose": "suggested_placement"
    }
  ]
}
```

The `source` values are proposed typed selectors, not executable expressions. An implementation accepts only supported selectors and validates their context. The suggested slot does not select a model, infer an engineering view scale or change the parent design.

### 14.3 A theme payload using the existing format

This example uses the current native file tag and verified supported keys. It changes only the panel color and control radius over a dark base; it is not a complete accessibility-reviewed theme. A future pack wraps it with source-bound identity and application-capability metadata.

```json
{
  "format": "solidcraft-theme/1",
  "name": "Workshop Slate Example",
  "base": "dark",
  "colors": { "panel": "#1f2937" },
  "radii": { "control_radius": 4 }
}
```

The spike checks JSON structure and compares these keys to the inspected source. It does not claim that the Rust theme importer or a native UI session was executed.

### 14.4 A transparent synthetic sizing model

This calculation is **a fictional workload for comparing granularity**, not a measurement of the existing catalog, a compression benchmark, a network forecast or a proven optimal pack size. Treat all assumed MiB as delivered payload cost and ignore container overhead, compression, retries and caching.

Assumed library:

| Content | Count | Assumed MiB each | Total MiB |
| --- | ---: | ---: | ---: |
| Finishes | 120 | 32 | 3,840 |
| Environments | 12 | 128 | 1,536 |
| Parts families | 12 | 1 | 12 |
| Drawing families | 12 | 1 | 12 |
| Theme families | 4 | 0.25 | 1 |
| **Total** | | | **5,401** |

Assumed user workflow: three finishes from one coherent 12-finish family, one environment, one parts family, one drawing family and one theme. Exactly selected content costs 226.25 MiB under these assumptions.

| Strategy | Assumed initial delivery, MiB | Delivery when one finish changes, MiB | Interpretation |
| --- | ---: | ---: | --- |
| One universal pack | 5,401 | 5,401 | Delivers every unrelated item. |
| One pack per broad type | 5,401 | 3,840 | This particular workflow touches every type. |
| Arbitrary 30-finish chunks; broad packs for other types | 2,521 | 960 | Assumes the three finishes happen to be in one arbitrary chunk. |
| Coherent 12-finish family; other items individually/family packaged | 514.25 | 384 | Compromise between useful collection and overhead. |
| Every finish/environment and small family independently packaged | 226.25 | 32 | Minimum selected payload here, with more publication/index/install units. |

For the same workflow spread across **three different** 12-finish families, the coherent-family delivery becomes **1,282.25 MiB**. That sensitivity is why grouping should be informed by actual co-use, not declared universally optimal from one example.

The model also explains why per-file disk deduplication does not alone improve the "one finish changed" ZIP-download column. The publisher still ships the whole changed archive unless the transport supports another strategy.

Reproduce the arithmetic with this standard-library-only Python fragment:

```python
library = {"finishes": 120 * 32, "environments": 12 * 128,
           "parts": 12, "drawings": 12, "themes": 4 * 0.25}
selected_other = 128 + 1 + 1 + 0.25
rows = {
    "universal": (sum(library.values()), sum(library.values())),
    "broad_types": (sum(library.values()), library["finishes"]),
    "arbitrary_chunks": (30 * 32 + 1536 + 12 + 12 + 1, 30 * 32),
    "coherent_family": (12 * 32 + selected_other, 12 * 32),
    "individual_assets": (3 * 32 + selected_other, 32),
}
assert rows["universal"] == (5401, 5401)
assert rows["arbitrary_chunks"] == (2521, 960)
assert rows["coherent_family"] == (514.25, 384)
assert rows["individual_assets"] == (226.25, 32)
assert 3 * 12 * 32 + selected_other == 1282.25
for strategy, costs in rows.items():
    print(strategy, *costs)
```

Use real catalog measurements next: selected assets per task, compressed artifact bytes, decoded memory, repeated-resource bytes, update frequency and first-use latency. Only then choose whether individual heavy assets, shared companions or object transport add enough value.

## 15. Implementation sequence and acceptance

### 15.1 Proposed slices

This sequence refines the first spike. It does not claim or reassign existing upstream work, alter CI, or provide an unmeasured delivery estimate.

| Slice | Deliverable | Exit evidence |
| --- | --- | --- |
| A. Contracts and inventory | Freeze terminology, type boundaries, source/identity rules and real migration consumers | Reviewed decision set and classified pilot inventory |
| B. Durable local store | OS data-root selection, store/profile generations, bounded local ZIP installation, clear cache separation | Install/restart/update-path tests with exact receipts and interruption recovery |
| C. First vertical asset | One existing material/appearance or parts consumer with persisted exact application | Apply/save/uninstall source/reopen offline, with correct undo |
| D. Theme adapter | Existing native theme payload, source-bound identity, separate availability/preview/activation | Same-name publishers coexist; install leaves current theme unchanged; recovery works |
| E. Catalog/collections | Static sources, resolved selection plans, optional members, signed curated-source lifecycle | Local/remote parity, correct ownership/removal and explicit trust behavior |
| F. Storage operations | Move, offline backup/restore, repair and conservative GC | Interrupted move, missing drive and offline restore preserve known content |
| G. Render/studio adapters | Coherent owned light/environment/stage application and exact persistence | Real supported renderer/native/web journeys; unrelated objects preserved |
| H. Drawings adapter | Paper/border first, then supported title blocks/styles/templates through stabilized receiver | Correct physical output, saved dependencies, explicit template update and drawing undo |
| I. Content repack | New family/tier releases built from the development inventory | Verified hashes, provenance/mappings, licenses and actual consumer acceptance |
| J. Measured optimizations | Per-file deduplication, shared companions/object delivery if justified | Measured storage/transfer benefit with unchanged semantics and recovery |

A public prototype can start with B+C and a few small curated packs. Theme support need not wait for all rendering or Drawings work; Drawings pack application must wait for its actual receiver/capabilities. Do not bundle every category into one all-or-nothing implementation milestone.

### 15.2 Acceptance scenarios

The following are future implementation gates, **not tests run in this documentation spike**.

| ID | Scenario | Required observation |
| --- | --- | --- |
| P01 | Install, replace/update the executable, restart | Same installed identities/bytes/preferences; no needless pack redownload. |
| P02 | Migrate metadata, interrupt before/after each commit boundary | A valid committed generation remains; no reset or phantom installation. |
| P03 | Open stable and nightly concurrently | Isolated mutable profiles; coordinated compatible store changes. |
| P04 | Roll back to an app that cannot read the new store/profile major | Clear incompatibility and preservation; no destructive repair. |
| P05 | Clear caches and reset ordinary preferences separately | Installed packs/originals/trust state remain; preferences reset only as described. |
| P06 | Update, uninstall and reinstall each actually shipped packaging channel | Observed data behavior matches documented guarantees and removal choices. |
| P07 | Remove one installation/profile sharing a store | Other profiles and explicit pins retain their content. |
| P08 | Move across disks, interrupt copy/switch, disconnect destination | Old authority survives until switch; unavailable storage is not replaced by an empty library. |
| P09 | Offline backup, remove a disposable test store, restore without sources | Included content applies correctly with exact identities and rights records. |
| P10 | Configuration-only restore without servers | Missing bytes are reported; no false offline-backup claim. |
| P11 | Restore old state, then perform signed network resolution | Trust rollback protection is not silently reset. |
| W01 | Browser app build update and cache cleanup | Asset store survives app-cache replacement. |
| W02 | Browser quota failure during staging | Prior committed install remains usable; partial bytes never appear installed. |
| W03 | Browser database upgrade with old tab open | Version-change/blocking handled without reset or corruption. |
| W04 | Persistent-storage request refused; site data later cleared | Honest status and working export/restore path; no durability overclaim. |
| W05 | Move web deployment to another origin | Explicit transfer plan; no claim of automatic access to old origin data. |
| O01 | Install only a fastener family | No finish, studio, drawing or theme dependency unless truly referenced. |
| O02 | Apply a finish to existing geometry | No hidden demonstration model or engineering property changes. |
| O03 | Install a collection, omit optional studio/theme | A valid selected workflow; no automatic application or theme activation. |
| O04 | Remove a collection with overlapping direct installs | Only its retention reasons removed; independently retained packs remain. |
| O05 | Install two quality tiers and two same-named publishers | Distinct exact identities; no silent replacement or source-name collision. |
| O06 | Change an installed folder externally | Integrity failure is actionable; no unverified bytes are silently accepted. |
| O07 | Repack a record into separate physical/visual assets | Origin map is explicit; existing applied values retain their meaning. |
| D01 | Create A4/A3 variants and a custom sheet | Physically correct layout with deliberate margins/title-block composition. |
| D02 | Change UI theme, DPI and machine locale | Saved drawing geometry/style and intended issued formatting remain stable. |
| D03 | Missing number, long identifier, missing glyph or wrong binding | Explicit bounded failure/overflow policy; no silent critical truncation. |
| D04 | Apply/update template, cancel, undo, redo | Correct drawing scope; preserved user overrides; parent model not unintentionally edited. |
| D05 | Template package removed or source offline | Saved permitted dependency closure reopens; external restrictions clearly identified. |
| D06 | Active SVG/external font/image reference in a template | Rejected by the declared import subset; no hidden network/code execution. |
| D07 | Restricted font/private branding exported | Deliberate permission-aware choice; no secret/internal-source leakage. |
| D08 | Actual-size versus fit-to-page output | Effective physical scale and clipping are reported truthfully. |
| T01 | Install a theme pack | Current preference and open documents remain unchanged. |
| T02 | Theme names slug identically across publishers | They coexist with distinct source-bound identities. |
| T03 | Bad/unsupported theme activated or startup interrupted | Accessible built-in fallback and last-known-good recovery. |
| T04 | Active theme removed/updated | Explicit replacement/retained choice; no dangling preference or unexpected activation. |
| T05 | Unsupported token, alias cycle, invisible focus or bad contrast | Supported validation reports the actual issue; no blanket certification. |
| X01 | Local file installation while network unavailable/forbidden | Self-contained content works; missing dependencies are explicit. |
| X02 | Remote catalog under native and browser constraints | Same identities; tested CORS/redirect/auth and platform capability behavior. |
| X03 | New optional asset type in a pack | Bounded inert retention only if admission permits; not usable as a missing required reference. |
| X04 | Existing source, license or capability removed | Retained content is not silently substituted; status and allowed operations are explicit. |

## 16. Decisions, validation and limitations

### 16.1 Decisions to settle before a production contract

The strongest defaults are settled by this analysis: per-user durable data, independent mutable profiles, immutable packs, coherent family boundaries, collection-level convenience, separate Drawings/theme semantics and a clean rebuild of temporary archive structure.

The remaining decisions require concrete implementation evidence or maintainer agreement:

- The first supported host adapters and the precise public schema/tag names.
- Whether the initial native store uses the recommended immutable directories or a bounded lazy-archive reader.
- The exact metadata transaction backend and cross-process locking/GC protocol.
- Which shipped packaging channels can promise uninstall retention and shared-store access.
- The storage-location override and portable-mode UI/CLI contract.
- The actual Drawings receiver revision and retained resource representation after G3 work integrates.
- Theme identity mapping and installed-inventory/materialized-preference limits.
- First measured pack sizes/quality tiers and supported decoder budgets.
- Curated source governance, trust bootstrap/rotation and private source behavior.
- The exact migration population: which existing user documents or installations need legacy adapters.

A schema can leave room for future types without advertising unsupported behavior. Every applicable type needs a real consumer, a persistence boundary and a demonstrable user journey.

### 16.2 Work performed for this spike

The work consists of a pinned source audit, primary-documentation research, comparison of storage/packaging/distribution alternatives, semantic design for Drawings/themes, explicit migration and UI contracts, draft examples and a reproducible synthetic sizing calculation.

Separate reviews covered platform/distribution lifecycle, asset/Drawings/theme semantics and repository integration. Identified ambiguities were corrected, including exact-archive retention for redistribution, moving a store shared by several profiles, Windows update/uninstall qualifications and the distinction between earlier Drawings code and unmerged G3 receivers.

Completed document checks:

- All three JSON examples parse; collection member IDs are unique and required members are selected by default.
- The paper-space example fits its frame: the title block and suggested view have a 5 mm gap and no positive-area overlap. This is arithmetic validation, not a rendered drawing test.
- The theme example uses source-verified supported keys and an in-range radius; the Rust importer was not executed.
- The included sizing calculation executes and reproduces every tabulated payload total and its three-family sensitivity case.
- All 17 table-of-contents anchors and 45 source-reference definitions resolve within this document. Whitespace checks are clean.

No SolidCraft application code, installer, CI workflow, source archive or runtime pack is changed by this report. No native/browser installation, migration, rendering or export behavior is claimed as tested. The source audit is not a certification that the proposed guarantees already exist.

### 16.3 Recommended first decision

Adopt the persistent-library and identity contracts, choose a small local-install vertical slice, and reorganize a representative subset of the development catalog around useful independent packs. Include a theme as a separate preference consumer and plan Drawings integration against its actual embedded-drawing receiver. Measure download/storage/application behavior before expanding the transport protocol.

## 17. Sources

Repository links are pinned wherever they describe source code. Discussion/PR links record dated coordination state and may change. External sources support platform or specification facts; package boundaries, workflows, failure policies and implementation recommendations are original design analysis.

### Repository evidence

- [R1] Existing per-user data-root selection.
- [R2] Personal native settings path.
- [R3] Newer upstream settings-directory override.
- [R4] Existing plugin inventory.
- [R5] Upstream application updater.
- [R6] Windows installer definition.
- [R7] Existing native theme file contract and limits.
- [R8] Theme import/selection integration.
- [R9] Browser document/OPFS loading.
- [R10] Service-worker application cache lifecycle.
- [R11] Flatpak permissions.
- [R12] Drawing paper and border foundations.
- [R13] Earlier saved drawing model in the inspected feature branch.
- [R14] Typed title-block foundation.
- [R15] Later G3 embedded-drawing decision.
- [R16] Federated development inventory.
- [R17] Original release archive/source inventory.
- [R18] Unmerged embedded-drawing receiver work.
- [R19] Repository architectural and contribution rules.
- [R20] Pinned draft embedded-drawing storage implementation.
- [R21] Pinned draft drawing command receiver.

### Primary external documentation

- [S1] XDG Base Directory Specification.
- [S2] Windows Known Folder IDs.
- [S3] Windows known-folder lookup API.
- [S4] Apple application support/cache guidance.
- [S5] Apple sandbox and application-group access boundaries.
- [S6] Windows managed app data and MSIX virtualization.
- [S7] Explicit portable-mode example.
- [S8] Signed update metadata and trust roles.
- [S9] Consistent SQLite backup options.
- [S10] WHATWG storage model.
- [S11] OPFS storage/deletion behavior.
- [S12] IndexedDB version-change coordination.
- [S13] Supplier distribution terms relevant to the McMaster example.
- [S14] GitHub release asset limits.
- [S15] GitHub Pages hosting limits.
- [S16] ISO public description of drawing sheet layout.
- [S17] ISO public description of title-block/header fields.
- [S18] CSS Paged Media specification status and concepts.
- [S19] SVG secure-static processing mode.
- [S20] OpenType embedding-rights fields.
- [S21] OFL bundling and licence guidance.
- [S22] DTCG token exchange specification and status.
- [S23] WCAG 2.2 contrast and accessibility criteria.
- [S24] Windows managed app-data update/uninstall policy.

[R1]: https://github.com/0xlillith/solidcraft/blob/807e367bdb03210dba188118fe000acd0a4b3725/crates/engine/src/recovery.rs
[R2]: https://github.com/0xlillith/solidcraft/blob/807e367bdb03210dba188118fe000acd0a4b3725/apps/solidcraft/src/main.rs
[R3]: https://github.com/tangency-labs/solidcraft/blob/514b0c84f9eb7655e3cdc986f0d44d4cd3416f4c/apps/solidcraft/src/startup.rs
[R4]: https://github.com/0xlillith/solidcraft/blob/807e367bdb03210dba188118fe000acd0a4b3725/crates/ui-egui/src/plugins.rs
[R5]: https://github.com/tangency-labs/solidcraft/blob/514b0c84f9eb7655e3cdc986f0d44d4cd3416f4c/crates/ui-egui/src/update/install.rs
[R6]: https://github.com/tangency-labs/solidcraft/blob/514b0c84f9eb7655e3cdc986f0d44d4cd3416f4c/assets/platforms/windows/solidcraft.wxs
[R7]: https://github.com/0xlillith/solidcraft/blob/807e367bdb03210dba188118fe000acd0a4b3725/crates/ui-egui/src/theme_files.rs
[R8]: https://github.com/tangency-labs/solidcraft/blob/514b0c84f9eb7655e3cdc986f0d44d4cd3416f4c/crates/ui-egui/src/prefs.rs
[R9]: https://github.com/tangency-labs/solidcraft/blob/514b0c84f9eb7655e3cdc986f0d44d4cd3416f4c/apps/solidcraft-web/src/files.rs
[R10]: https://github.com/tangency-labs/solidcraft/blob/514b0c84f9eb7655e3cdc986f0d44d4cd3416f4c/apps/solidcraft-web/sw.js
[R11]: https://github.com/tangency-labs/solidcraft/blob/514b0c84f9eb7655e3cdc986f0d44d4cd3416f4c/packaging/linux/flatpak/ai.storyteller.solidcraft.bundle.yml
[R12]: https://github.com/0xlillith/solidcraft/blob/b68fa4f868b8dd5ea312ecb5dfc8a0760dec4c5b/crates/geom/src/drawing_sheet.rs
[R13]: https://github.com/0xlillith/solidcraft/blob/b68fa4f868b8dd5ea312ecb5dfc8a0760dec4c5b/crates/doc/src/drawing.rs
[R14]: https://github.com/0xlillith/solidcraft/blob/b68fa4f868b8dd5ea312ecb5dfc8a0760dec4c5b/crates/geom/src/title_block.rs
[R15]: https://github.com/tangency-labs/solidcraft/issues/3890#issuecomment-6427665746
[R16]: https://github.com/0xlillith/solidcraft-render-catalog/blob/df356e2f277b9bd0d1cc62553dac8208f1aae958/authoring/RESOURCE-INDEX.json
[R17]: https://github.com/0xlillith/solidcraft-render-catalog/blob/df356e2f277b9bd0d1cc62553dac8208f1aae958/distribution/RELEASE-ARTIFACTS.json
[R18]: https://github.com/tangency-labs/solidcraft/pull/3891
[R19]: https://github.com/0xlillith/solidcraft/blob/807e367bdb03210dba188118fe000acd0a4b3725/AGENTS.md
[S1]: https://specifications.freedesktop.org/basedir/latest/
[S2]: https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid
[S3]: https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetknownfolderpath
[S4]: https://developer.apple.com/library/archive/documentation/MacOSX/Conceptual/BPFileSystem/Articles/WhereToPutFiles.html
[S5]: https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox
[S6]: https://learn.microsoft.com/en-us/windows/msix/desktop/flexible-virtualization
[S7]: https://code.visualstudio.com/docs/setup/portable
[S8]: https://theupdateframework.io/docs/metadata/
[S9]: https://sqlite.org/backup.html
[S10]: https://storage.spec.whatwg.org/
[S11]: https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system
[S12]: https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API/Using_IndexedDB
[S13]: https://www.mcmaster.com/termsandconditions/
[S14]: https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases
[S15]: https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits
[S16]: https://www.iso.org/standard/29017.html
[S17]: https://www.iso.org/standard/35446.html
[S18]: https://www.w3.org/TR/css-page-3/
[S19]: https://www.w3.org/TR/SVG2/conform.html
[S20]: https://learn.microsoft.com/en-us/typography/opentype/spec/os2
[S21]: https://openfontlicense.org/ofl-faq/
[S22]: https://www.designtokens.org/TR/2025.10/format/
[S23]: https://www.w3.org/TR/WCAG22/

[R20]: https://github.com/CamWight/solidcraft/blob/472a330691f89fe58e90e65665bd035927c07ca0/crates/io/src/design/drawing.rs
[R21]: https://github.com/CamWight/solidcraft/blob/36cf48a3f64c530a128f5d794f37b7cfa24cc3ab/crates/engine/src/cmd/drawing.rs
[S24]: https://learn.microsoft.com/en-us/windows/apps/develop/data/store-and-retrieve-app-data
