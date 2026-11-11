# popamp-plugin-api

# ⚠️ This API is under development

## It can change at any time, with no compatibility with older versions

A plug-in that works today may stop loading in a later Popamp. Until
Popamp 1.0, a change may come without a new API version to warn you.

The types of the calls between [Popamp](https://github.com/popamp/popamp)
and its plug-ins. The player and plug-ins written in Rust both use this
crate, so the compiler checks both sides of every call.

A plug-in is an [Extism](https://extism.org) WebAssembly module with a
`plugin.toml` next to it. How plug-ins are installed, allowed and loaded
is in Popamp's [docs/plugins.md](https://github.com/popamp/popamp/blob/main/docs/plugins.md);
the calls and their JSON are in [src/lib.rs](src/lib.rs).

## Use

```toml
[dependencies]
popamp-plugin-api = { git = "https://github.com/popamp/popamp-plugin-api", branch = "main" }
extism-pdk = "1.4"
```

```rust
use extism_pdk::*;
use popamp_plugin_api::{Play, VERSION};

#[plugin_fn]
pub fn api_version(_: String) -> FnResult<Json<popamp_plugin_api::Version>> {
    Ok(Json(VERSION))
}

#[plugin_fn]
pub fn open(Json(location): Json<String>) -> FnResult<Json<Play>> {
    Ok(Json(Play::Http { url: location }))
}
```

Build with `cargo build --release --target wasm32-unknown-unknown`
(`rustup target add wasm32-unknown-unknown` once).

## Versions

The crate's `MAJOR.MINOR` is the API version, `VERSION` in the crate.
Popamp loads a plug-in built for the same major version and a minor
version no newer than its own. A patch release fixes the crate without
changing the calls.

Version 1.1 added `decoder` plug-ins. Until Popamp 1.0 the API may still
change within a version; depend on `branch = "main"`, and your
`Cargo.lock` pins the commit.

## License

MIT or Apache-2.0, at your option.
