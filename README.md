<div align="center">

```text
           .       *
       .-----------------------.
       |      +  R I P  +      |
   *   |                       |   .
       |   here lies a file    |
       |   you chose to keep   |
       |   until you didn't    |
       '-----------------------'
     _______/_________\_______
    ///////////////////////////

  ____ ____      _    __     _______
 / ___|  _ \    / \   \ \   / / ____|
| |  _| |_) |  / _ \   \ \ / /|  _|
| |_| |  _ <  / ___ \   \ V / | |___
 \____|_| \_\/_/   \_\   \_/  |_____|
```

### a file format with a lifespan

**GRAVE** is a professional-grade volatile retention format  
for data you have chosen to let go.

[![CI](https://github.com/itsbryanman/grave/actions/workflows/ci.yml/badge.svg)](https://github.com/itsbryanman/grave/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-c9a76c?style=flat-square)](LICENSE)
[![rustc](https://img.shields.io/badge/rustc-1.87%2B-c07843?style=flat-square)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/docker-buildable-2b5797?style=flat-square)](Dockerfile)
[![decay](https://img.shields.io/badge/decay-inevitable-black?style=flat-square)](spec/RFC-666.md)
[![graves disturbed](https://img.shields.io/badge/graves%20disturbed-0-brightgreen?style=flat-square)](spec/RFC-666.md)
[![mourning](https://img.shields.io/badge/mourning-accepted-6b4c8a?style=flat-square)](docs/registry.md)
[![wasm](https://img.shields.io/badge/wasm-consecrated-4a5d43?style=flat-square)](https://itsbryanman.github.io/grave/)

</div>

---

**Live viewer:** [itsbryanman.github.io/grave](https://itsbryanman.github.io/grave/)

Every `.grave` file decays. Deterministically. On a schedule you set at burial, accelerated
by neglect, worn slightly by every visit, and slowed — never stopped — by mourning. Two
strangers opening the same grave on the same day witness the same rot, byte for byte.

The original remains untouched inside the coffin, recoverable at any time.

Unless you buried it in **consecrated ground**. Then it is simply gone, a little more each
time you look.

```text
                        .---------------.
                    .'                 '.
                  /                     \
                /                       \
              |           RIP           |
              |                         |
              |   here lies a file      |
              |   you chose to keep     |
              |   until you didn't      |
              |                         |
              |_________________________|
            ////////////////////////////\
          //////////////////////////////\
```

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

> **Note:** Deterministic graves never destroy the original payload. Hardcore graves do,
> and they say so loudly before the oath is taken.

## Interment Quickstart

```bash
cargo install --path crates/grave-cli        # or: docker build -t grave .

grave bury secrets.txt --epitaph "kept too long"
grave inspect secrets.txt.grave              # reads the stone; leaves no trace
grave open secrets.txt.grave                 # a visitation; the grave remembers
grave open secrets.txt.grave --at 2027-01-01 # a vigil; sees the future, touches nothing
grave mourn secrets.txt.grave                # slows the rot; once a week, no more
grave graveyard ~/graves                     # walk the grounds
grave exhume secrets.txt.grave               # the deceased returns, remembering nothing
```

## The Four Rots

| Profile | Byte | What takes the body |
|---|---|---|
| `mold` | `0x01` | Colonies seed, spread, and mature day by day across images and over text. Block-grid cellular growth. Patient. |
| `static` | `0x02` | Scanline tears, salt-and-pepper noise, block corruption, and — near the end — the slow rolling bands of a signal that has given up. |
| `burnin` | `0x03` | The file remembers being looked at. Every visit stacks another ghost, contrast craters, and a hot spot brightens where everyone stared. |
| `dataloss` | `0x04` | Clustered dead regions and vanished sentences, labeled with plausible offsets. Orderly enough to feel official. `[LOST: 47 bytes]` |

All four are pure functions of the burial seed, the day, and the visitation record.
Rot is computed, never stored. See [RFC-666 §4](spec/RFC-666.md).

## Visiting Hours

The browser viewer is live at [itsbryanman.github.io/grave](https://itsbryanman.github.io/grave/) —
drag a `.grave` onto it, read the stone, and scrub the slider from burial day to the
terminal horizon to watch the decay unfold. The future side of the scrubber is tinted.
The terminal position is a headstone. The same files also live locally in
[`viewer/`](viewer/index.html).

The viewer never writes. That is the **Amber Clause**: watching through glass is
observation, not disturbance. `last_opened` is not updated, visits are not counted.
The prebuilt wasm bundle is checked in under [`viewer/pkg`](viewer/pkg) for local use, but
the Pages workflow rebuilds that bundle fresh before publishing.

Rebuild the wasm bundle with:

```bash
wasm-pack build crates/grave-wasm --target web --out-dir viewer/pkg
```

## The Format

The full specification is [RFC-666](spec/RFC-666.md): the v2 header layout with fixed
mutable offsets, dual trailer CRCs, day-quantized decay computation, the 512 MiB
exhumation bound, mourning semantics, and Disturbance Recovery.

The short version:

```text
[ magic "GRAVE" ][ v2 ][ burial_id 32B ][ buried_at ][ last_opened ][ visits ]
[ profile ][ flags ][ half-life ][ mourn credit ][ epitaph ][ name ][ mime ]
[ original length ][ zstd payload ..................... ][ CRC ✝ ][ CRC ✝ ]
```

Tamper with the burial record and the header CRC will not match. The grave is not
angry; it simply renders at **maximum decay** from then on. The remains, verified
separately, can still be exhumed. Disturbance ruins trust in the record, not
necessarily the deceased.

## Consecrated Ground (`--hardcore`)

Hardcore burial is opt-in, confirmed by typing the filename, and irreversible. On every
live open the rendered rot becomes the new payload — each visit genuinely loses
information, and the loss compounds. Exhumation is refused with exit code `66`.
Terminal decomposition is final.

The warning shown at hardcore burial is the only unfunny paragraph in this project.
That is on purpose.

## Frequently Whispered Questions

**Does `grave open` destroy my file?**
Not unless you buried it with `--hardcore`.

**Why does the viewer not count as a visit?**
The viewer never writes. Observation through glass does not disturb the grave.

**Why does a grave I never visit rot *faster*?**
Neglect is an accelerant. You must visit your dead. `grave mourn` helps, once a week.

**Why can a disturbed grave still be exhumed?**
The payload CRC and stored original length are checked separately from the header CRC.

**What happens at 100%?**
The viewer shows only the headstone: name, dates, epitaph, visits, mournings.
Deterministic graves can still be exhumed afterward — the RFC calls this
*disturbing the grave*, and it is legal, if frowned upon.

**Can I bury a `.grave` file?**
Recursive interment is legal in most jurisdictions.

## Exit Codes

`0` peace · `1` misfortune · `64` malformed rites · `65` disturbance ·
`66` consecrated ground · `67` terminal decomposition

Scripts may grieve programmatically.

---

<div align="center">

```text
      ✝            ✝            ✝            ✝            ✝
   ________     ________     ________     ________     ________
```

MIT — in this life or the next.

*Maintained by the groundskeepers. See the [Groundskeeper's Handbook](CONTRIBUTING.md).*

</div>
