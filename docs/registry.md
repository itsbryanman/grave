# The Registry

The registry is small on purpose. A grave only has a few sanctioned ways to fail.

## Format Versions

| Version | Status | Notes |
| --- | --- | --- |
| `2` | current | stores original uncompressed length in the header and validates output against it |

## Rot Profiles

| Byte | Name | Meaning |
| --- | --- | --- |
| `0x01` | mold | growth, damp, and overrun |
| `0x02` | static | noise, tearing, and signal rot |
| `0x03` | burnin | visitation scars and retained afterimage |
| `0x04` | dataloss | clustered absence and official silence |

## Flags

| Bit | Name | Meaning |
| --- | --- | --- |
| `0` | `hardcore` | live opens rewrite the payload and exhumation is refused |
| `1` | `mourned-recently` | the weekly mourning window is still closed |

## Exit Codes

| Code | Meaning |
| ---: | --- |
| `0` | success |
| `1` | generic failure |
| `64` | usage error |
| `65` | corrupt, disturbed, or unreadable grave |
| `66` | hardcore refusal |
| `67` | terminal decomposition |
