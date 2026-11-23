# Event dispatch benchmark

Measures how long `PluginManager::fire` takes on `ServerTickStartEvent` with native handlers, using [criterion](https://crates.io/crates/criterion).

## How to run it

1. Copy `event_dispatch.rs` into `crates/pumpkin-core/benches/`.
2. Add the following to `crates/pumpkin-core/Cargo.toml`:

   ```toml
   [dev-dependencies]
   criterion = { workspace = true, features = ["async_tokio"] }

   [[bench]]
   name = "event_dispatch"
   harness = false
   ```

3. Run:

   ```
   cargo bench -p pumpkin-core --bench event_dispatch
   ```

To compare versions, run it on each one with the same settings and on the same machine. The published numbers were taken with `CARGO_PROFILE_BENCH_LTO=thin` and `CARGO_PROFILE_BENCH_CODEGEN_UNITS=16`.
