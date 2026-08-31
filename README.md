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

Currently, it only supports conversion of the English DATs, but the plan is to eventually support the other languages as well, once the conversion tables and unique control-structures for those are figured out. See plans for future work below.

## Planned Work

It is planned to eventually support handling of most DAT-related things, like:

- [ ] All languages:
    - [x] English text in DATs
    - [ ] Japanese text in DATs
    - [ ] French text in DATs
    - [ ] German text in DATs
- [ ] All DAT formats (non-exhaustive list):
    - [x] Dialog
    - [x] Entity names
    - [x] Status info
    - [ ] Item info (most are supported)
    - [ ] d_msg (most are supported)
    - [ ] XISTRING (partially done)
    - [ ] Spell info
    - [ ] Ability info
    - [ ] Quest info
    - [x] Events/cutscenes (byte code and data)
- [ ] GUI editor for complex DATs, i.e.:
    - [ ] Items
    - [ ] DATs with images
    - [ ] Events/cutscenes joined with the used dialog text strings and entities

### Breaking changes

There will most likely be breaking changes as new versions of this tool gets updated, since various fields can be added/removed/renamed.

For example, there are currently still unknown fields included in the human-readable files. These fields are necessary data for it to properly generate usable DAT files again, so they have to be included. As understanding of the DAT files progresses, and the meaning of these fields are determined, the field names/content will be changed/renamed. This will currently cause the tool to not be backwards-compatible with the previous versions of human-readable files (since their fields/formats have changed), in which case they will have to re-exported from DATs to be able to generate DATs again.



## Development setup

The project is built using [Rust](https://www.rust-lang.org/) utilizing the [tauri](https://tauri.app/) toolkit for building the application. The frontend UI is built using the [solidjs](https://www.solidjs.com/) framework.

### Prerequisites

The following software is required to develop and build the application:

- [Rust](https://www.rust-lang.org/learn/get-started) (and cargo) for the backend
- [Prerequisites for building tauri applications](https://tauri.app/v1/guides/getting-started/prerequisites)
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

    Copyright © 2004-2014 Tim Van Holder, Nevin Stepan, Windower Team

This project also uses encoding conversion table files, which were originally from the POLUtils project,
but some have been modified to allow them being used in reverse to allow encoding back to the original symbols.
The full license file and copyright text have been included in the folder that these reside in.


### Events

Decoding of events and the byte code is done based on [XiEvents](https://github.com/atom0s/XiEvents) by atom0s. The license associated with the repository [can be found here](https://github.com/atom0s/XiEvents/blob/main/LICENSE.md), which is currently AGPL.
