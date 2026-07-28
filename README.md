# GRAVE

GRAVE is a professional-grade volatile retention format for data you have chosen to let go.

[![CI](https://github.com/itsbryanman/grave/actions/workflows/ci.yml/badge.svg)](https://github.com/itsbryanman/grave/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/grave?style=flat-square)](https://crates.io/crates/grave)
[![License](https://img.shields.io/badge/license-MIT-c9a76c?style=flat-square)](LICENSE)
[![rustc](https://img.shields.io/badge/rustc-1.87%2B-c07843?style=flat-square)](https://www.rust-lang.org/)
[![decay](https://img.shields.io/badge/decay-inevitable-black?style=flat-square)](spec/RFC-666.md)
[![graves disturbed](https://img.shields.io/badge/graves%20disturbed-0-brightgreen?style=flat-square)](spec/RFC-666.md)
[![mourning](https://img.shields.io/badge/mourning-accepted-purple?style=flat-square)](docs/registry.md)

```text
$ grave bury photo.jpg --profile mold --epitaph "she was beautiful once"
photo.jpg was laid to rest on 2026-07-28.
  Profile: mold | Half-life: 30 days | Epitaph: "she was beautiful once"
  Estimated terminal decomposition: 2027-03-12.
  Grave file: ./photo.jpg.grave

$ grave inspect photo.jpg.grave
+--------------------------------------------------------------+
| photo.jpg.grave                                              |
+==============================================================+
| Interred      photo.jpg (image/jpeg)                         |
| Buried        2026-07-28 (18 days ago)                       |
| Last visited  2026-08-02 (13 days ago)                       |
| Visits        4                                              |
| Profile       mold                                           |
| Decay         ########....  61.3%                            |
| Prognosis     terminal by 2027-03-12                         |
| Epitaph       "she was beautiful once"                       |
+--------------------------------------------------------------+
```

> Note: Deterministic graves do not destroy the original payload. Hardcore graves do, and they say so loudly before the oath is taken.

## Interment Quickstart

```bash
cargo run --bin grave -- bury photo.jpg --profile mold --epitaph "she was beautiful once"
cargo run --bin grave -- inspect photo.jpg.grave
cargo run --bin grave -- open photo.jpg.grave
cargo run --bin grave -- graveyard .
```

The browser viewer lives in [viewer/index.html](viewer/index.html). The wasm bundle can be rebuilt with:

```bash
wasm-pack build crates/grave-wasm --target web --out-dir /PROJECTS/grave/viewer/pkg
```

## The Four Rots

- `mold`: block-grid growth that creeps day by day across images and over text.
- `static`: scan tears, noise, and signal collapse, as if the medium itself is arguing.
- `burnin`: the file remembers each visit and smears itself around the places it has been seen.
- `dataloss`: clustered dead regions and vanished sentences, orderly enough to feel official.

## Visiting Hours

The viewer is read-only by design. Drag a `.grave` file into [viewer/index.html](viewer/index.html), scrub by whole days, and watch the decay panel update without touching `last_opened`. The future side of the scrubber is tinted, the terminal position swaps to a headstone, and the canvas preview is downscaled to 1600px on its longest edge for sanity.

## The Format

The specification is written down in [spec/RFC-666.md](spec/RFC-666.md). It documents the v2 header layout, fixed offsets, trailer CRCs, day-quantized rendering, the 512 MiB safety bound, and the exact behavior of terminal files, mourning, and disturbance recovery.

## Docker

```bash
docker build -t grave .
docker run --rm -v "$(pwd)":/graves grave bury /graves/photo.jpg
```

Live visits and mourning patch graves in place, so bind mounts are the expected mode. The container clock supplies `now`, which means container time and host time should agree if you care about exact decay dates.

## Hardcore

Hardcore burial is opt-in and irreversible. On every live open, the rendered output becomes the new payload. Exhumation is refused with exit code `66`. Terminal decomposition is final. The warning paragraph in `grave bury --hardcore` is the only unfunny paragraph in the project on purpose.

## Frequently Whispered Questions

### Does `grave open` destroy my file?

Not unless you buried it with `--hardcore`.

### Why does the viewer not count as a visit?

The viewer never writes. That is the Amber Clause: watching through glass is observation, not disturbance.

### Why does `--at 2026-12-01` not touch the grave?

Because it is a vigil, not a visitation. The date is interpreted as `2026-12-01 23:59:59 UTC`, rendered with day-quantized decay, and written nowhere.

### Why can a disturbed grave still be exhumed?

The payload CRC and stored original length are checked separately from the header CRC. Disturbance ruins trust in the record, not necessarily the remains.

### How large can a grave be?

The stored uncompressed length must not exceed 512 MiB. Implementations must enforce that bound during exhumation and rendering.

MIT, in this life or the next.
