# XI Tinkerer

> [!NOTE]
> This is the Vana360-maintained project fork. Changes target `main`; see
> [CONTRIBUTING.md](CONTRIBUTING.md) for its contribution requirements.
> `upstream/develop` is a read-only intake reference.

Tool for decoding and encoding FFXI DAT files.

It can export DATs into human-readable files (YAML), which can then be edited and re-encoded into DAT files.

The Vana360 branch can inventory one FTABLE/VTABLE layout and test each mapped,
recognized DAT decoder/encoder for exact byte round trips:

```text
cargo run --locked -p xi-tinkerer-cli -- audit-dats <ffxi-path> --out audit.json
```

The JSON report uses retail-relative paths and does not include the supplied
client root.
One DAT file is converted 1:1 with exactly one editable file.

For an Xbox publisher-package runtime root, pass `--xbox-packages`; the
command reads FTABLE/VTABLE from `0001`, resolves each DAT to its publisher
package, and writes a schema-4 package accounting report:

```text
cargo run --locked -p xi-tinkerer-cli -- audit-dats <runtime-root> --xbox-packages --out audit.json
```

Report `package` fields name physical source roots. Slot zero prefers the
decoded `R000100` publisher output and falls back to base root `0001` when no
override exists. This selection is for offline resource extraction; it does
not change how a title runtime registers content packages. Unselected
`R000100` files remain separately reported as unmounted.

Known format IDs are classified from Tinkerer's authoritative DAT mapping
before content probes are attempted. Ambiguous ID aliases are resolved only
when exactly one mapped format probe succeeds.

The `client_format_mappings` section projects that current mapping through the
supplied client's FTABLE/VTABLE and mounted package layout. Each entry is
reported as `selected`, `missing`, or `absent`; this keeps later-client mapping
entries visible without treating them as part of an older client snapshot.

Export the complete client reconciliation contract from one Xbox publisher-
package runtime root:

```text
cargo run --locked -p xi-tinkerer-cli -- export-client-contract <runtime-root> --out-dir <catalog-directory>
```

The command writes an indexed set of global resources, item and key-item
tables, zone entities, events, text, and the schema-4 DAT audit. Global
resources retain mapped-but-absent tables and selected tables that the current
decoder cannot read, so an older client snapshot does not silently inherit
later IDs. Normalized item payloads live in `items.json`; their corresponding
global resource entries reference that catalog instead of duplicating it.
Later item-table mappings remain in the global resource catalog as generic
selected, absent, missing, or decode-failed evidence.
All output paths are retail-relative, and retail-backed output remains private.

For a deterministic neutral export of the proven area-name, title, and status-
name tables, use an Xbox publisher-package runtime root:

```text
cargo run --locked -p xi-tinkerer-cli -- export-client-globals <runtime-root> --out client-globals.json
```

The schema-1 JSON preserves numeric IDs and all decoded values, records only
retail-relative source paths, and does not mutate a server checkout. Retail-
backed output remains private.

Export the English per-zone text catalog separately:

```text
cargo run --locked -p xi-tinkerer-cli -- export-zone-text <runtime-root> --out zone-text.json
```

The schema-1 report keeps all 256 zone slots. Exact four-byte empty-dialog
sentinels become zones with no text entries. The report contains only numeric
zone and text IDs, decoded text, and retail-relative source paths; it does not
infer an expansion cutoff or mutate a server checkout.

Export the matching English entity-name catalog separately:

```text
cargo run --locked -p xi-tinkerer-cli -- export-zone-entities <runtime-root> --out zone-entities.json
```

The schema-1 report keeps all 256 primary zone slots with numeric entity IDs,
decoded names, and retail-relative source paths.
Empty entity tables remain present with no entries.
The report does not generate server entity records or mutate a server checkout.

Export the matching language-neutral event catalog separately:

```text
cargo run --locked -p xi-tinkerer-cli -- export-zone-events <runtime-root> --out zone-events.json
```

The schema-1 report keeps all 256 primary zone slots, records the DAT identity
and retail-relative selected path, and preserves event blocks, event IDs, data,
and byte code as stable hexadecimal. It does not generate server event records
or mutate a server checkout. Retail-backed output remains private.

Export the selected client item tables separately:

```text
cargo run --locked -p xi-tinkerer-cli -- export-items <runtime-root> --out items.json
```

The schema-1 report covers general items, usable items, weapons, armor, puppet
items, and currency. It combines the available English and Japanese text with
neutral item data and records only retail-relative source paths. Retail-backed
output remains private.

Export the selected client key-item tables separately:

```text
cargo run --locked -p xi-tinkerer-cli -- export-key-items <runtime-root> --out key-items.json
```

The schema-1 report decodes the English and Japanese key-item DMSG tables,
preserves each DAT entry index and numeric key-item ID, and records names,
plural names, and descriptions when present. It records only retail-relative
logical and selected source paths. Entries with missing numeric content remain
in the report with a null ID for validation; no server IDs are renumbered.
Retail-backed output remains private.

General editable text exports target English DATs. Item reports also decode the
mapped Japanese text tables. Unknown fields remain in editable exports because
they are required for byte-accurate re-encoding.

Exported YAML is not a stable interchange format. When a release changes field
names or structure, re-export the source DAT before editing and rebuilding it.

## Development setup

The project uses [Rust](https://www.rust-lang.org/),
[Tauri](https://tauri.app/), and [SolidJS](https://www.solidjs.com/).

### Prerequisites

The following software is required to develop and build the application:

- [Rust](https://www.rust-lang.org/learn/get-started) (and cargo) for the backend
- [Prerequisites for building Tauri applications](https://v2.tauri.app/start/prerequisites/)
- [NodeJS](https://nodejs.org/en/download) for the frontend
- [pnpm](https://pnpm.io/installation) as the NodeJS package manager

### Developing

The backend rust crates can be built/tested/etc with the regular `cargo build`, `cargo test`, etc.

To develop on the frontend, navigate to the `client` directory and install the necessary dependencies with `pnpm install`.

Once they're installed, you can develop the frontend application using the following command, which will provide automatic (hot-)reloading whenever the frontend or tauri-backend crate changes:

```sh
pnpm tauri dev
```



## Credits

The starting point for the binary structure of some of the DAT formats, which are used in this project,
were partially derived from the [POLUtils project](https://github.com/Windower/POLUtils) code,
so credit goes to them for their work in reversing these. Their work is licensed under the
[Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) with the following copyright:

    Copyright (c) 2004-2014 Tim Van Holder, Nevin Stepan, Windower Team

This project also uses encoding conversion table files, which were originally from the POLUtils project,
but some have been modified to allow them being used in reverse to allow encoding back to the original symbols.
The full license file and copyright text have been included in the folder that these reside in.


### Events

Event byte-code decoding is based on
[XiEvents](https://github.com/atom0s/XiEvents) by atom0s. Its license is
[provided by that project](https://github.com/atom0s/XiEvents/blob/main/LICENSE.md).
