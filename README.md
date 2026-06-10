# librustdesk_core

RustDesk HarmonyOS native core static library builder.

## Structure

- `native_rust_core/` - Bridge layer (bridge_api.rs, bridge_state.rs, lib.rs)
- `rustdesk-master/` - Upstream RustDesk source (1.4.7)
- `patches/` - OHOS-specific patches

## Build

```bash
cd native_rust_core
cargo build --release --target aarch64-unknown-linux-ohos
```

Output: `librustdesk_core.a`
