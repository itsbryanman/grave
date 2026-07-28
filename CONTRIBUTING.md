# The Groundskeeper's Handbook

Thank you for tending the grounds.

## Working Rules

- Keep the format deterministic. The same grave and the same quantized day must render identically.
- Do not weaken the 512 MiB bound.
- Do not add placeholder behavior, dummy assets, or silent fallbacks.
- Document reality. If the implementation changes, update [spec/RFC-666.md](spec/RFC-666.md) in the same pull request.

## Local Commands

```bash
cargo fmt --all
cargo clippy --workspace --exclude grave-wasm --all-targets -- -D warnings
cargo test --offline
wasm-pack build crates/grave-wasm --target web --out-dir /PROJECTS/grave/viewer/pkg
wasm-pack test --node crates/grave-wasm
```

## Golden Discipline

- Fixtures are generated in code.
- If a render algorithm changes intentionally, regenerate the golden expectations in the same pull request.
- Use the day-quantized render path for goldens. The fractional inspect path is for inspection only.

## Hardcore Changes

Hardcore is the one place where live opens are allowed to replace payload bytes. Treat that path carefully:

- temp-file-then-rename,
- preserve `buried_at`,
- keep the warning paragraph plain and honest,
- prove compounding with tests.

## Viewer Notes

- The viewer is read-only.
- The slider is day-stepped.
- Terminal positions swap to the headstone.
- Keep the preview capped to a reasonable edge length for browsers.

## Lore

Stay in character everywhere except the single honest hardcore warning paragraph. The joke only works if the dangerous part is perfectly clear.
