# Golden-vector fixtures

These JSON files are the single source of truth for Vindue's geometry and
config semantics — **the same files are consumed by both test suites**:

- **TypeScript** — `src/fixtures.test.ts` imports them directly (vitest)
- **Rust** — `src-tauri/src/config.rs` embeds them with `include_str!` (cargo test)

That parity is the contract (see CONTRIBUTING.md): the TS front end and the
Rust core must agree on selection→rect math, config validation, and
defaulting, byte-for-byte against these vectors. Changing behavior? Update
the fixture first, then make both sides pass it. Adding behavior? Add a
fixture for it before shipping.
