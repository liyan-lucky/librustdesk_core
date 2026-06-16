# librustdesk_core

RustDesk HarmonyOS 原生核心静态库构建器。从 RustDesk 1.4.7 上游源码通过 OHOS 交叉编译构建 `librustdesk_core.a`，并生成完整的 C++ NAPI 桥接层，供 HarmonyOS ArkTS 应用使用。

[English](README_EN.md)

## 架构

```
ArkTS UI (11_Rustdesk_harmonyos)
    -> NAPI
librustdesk_bridge.so
    -> C++ 桥接加载器 (cpp/)
    -> Rust C ABI (native_rust_core/)
librustdesk_core.a
    -> rustdesk_harmony_bridge
    -> RustDesk 官方 session/core (rustdesk-master/)
RustDesk 服务器 / 对端
```

## 目录结构

| 目录 | 说明 |
|------|------|
| `native_rust_core/` | Rust 桥接层（bridge_api.rs, bridge_state.rs, lib.rs） |
| `rustdesk-master/` | 上游 RustDesk 源码（1.4.7）含 OHOS 补丁 |
| `patches/` | OHOS 特定 crate 补丁（machine-uid） |
| `rdev-fork/` | rdev 输入库 fork，含 OHOS 支持 |
| `cpp/` | C++ NAPI 桥接层（abi.h, loader.cpp, CMakeLists.txt） |
| `scripts/` | 构建脚本和代码生成器 |

## 关键文件

### Rust 桥接层（`native_rust_core/`）

| 文件 | 说明 |
|------|------|
| `src/bridge_api.rs` | C FFI 导出（约 2872 行），所有 `rustdesk_bridge_*` 函数 |
| `src/bridge_state.rs` | 桥接状态快照管理（BridgeSnapshot, 事件队列） |
| `src/lib.rs` | crate 入口 |
| `Cargo.toml` | `crate-type = ["staticlib"]`，依赖 rustdesk 1.4.7 |
| `build.rs` | 为 OHOS 目标添加 `-Wl,-z,notext` 链接参数 |

### C++ NAPI 桥接层（`cpp/`）

| 文件 | 说明 |
|------|------|
| `rustdesk_bridge_abi.h` | C ABI 头文件，声明所有 `rustdesk_bridge_*` 函数 |
| `rustdesk_bridge_loader.cpp` | NAPI 模块加载器，将 C ABI 封装为 NAPI 导出 |
| `ohos_stubs.cpp` | OHOS 平台桩（xcb, OH_TimeService, qsort_r） |
| `CMakeLists.txt` | 将 `librustdesk_core.a` 链接进 `librustdesk_bridge.so` |
| `types/librustdesk_bridge/index.d.ts` | TypeScript 类型声明 |
| `undefined_symbols.txt` | 未定义符号列表，用于链接调试 |

### 代码生成脚本（`scripts/`）

| 脚本 | 说明 |
|------|------|
| `generate_bridge_api.js` | 从 core.rs 生成 bridge_api.rs |
| `generate_cpp_bridge.js` | 从 core.rs 生成 ABI 头文件和 NAPI loader |
| `generate_ts_bridge.js` | 从 core.rs 生成 TS 类型声明 |
| `regenerate_all.js` | 一键重新生成所有桥接代码 |
| `dedup_abi.js` | ABI 头文件声明去重 |
| `dedup_loader.js` | NAPI 注册去重 |
| `dedup_loader_funcs.js` | NAPI 函数定义去重 |
| `rename_mapping.js` | OHOS 名称到官方 wire_ 名称映射 |
| `build_native_bridge.ps1` | Windows 交叉编译构建脚本 |
| `build_native_bridge.sh` | Linux/macOS 构建脚本 |

## 构建

### Windows（主要）

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build_native_bridge.ps1
```

Windows 脚本在 Cargo 运行前准备目标静态依赖。冷构建时会编译 `libsodium`、下载编译 `libvpx` `1.15.2`、下载编译 `libyuv` 修订版 `0faf8dd0e004520a61a603a4d2996d5ecc80dc3f`，并安装到 `VCPKG_INSTALLED_ROOT\arm64-linux`。对于 `libvpx`，仅构建 `libvpx.a` make 目标并手动安装公共头文件；不要运行默认的 `make && make install`，因为那会同时构建未使用的 `libvpxrc.a`（来自 C++ RTC 速率控制源码 `vp9/ratectrl_rtc.cc`, `vp8/vp8_ratectrl_rtc.cc`）。精简版在线 SDK 可能不包含兼容的 libc++ 头文件，MSYS2 libc++ 也不是 OHOS SDK clang 的安全回退。`libyuv` 在 SDK libc++ include 目录存在时可以使用它，但构建不能依赖 MSYS2 libc++ 头文件。

### Linux

```bash
./scripts/build_native_bridge.sh aarch64-unknown-linux-ohos release
```

### CI/CD

**Windows 在线构建**：`.github/workflows/build-core-windows.yml`
- 运行环境：`windows-2022`
- Rust 工具链：1.88.0
- 使用 Cargo `release` profile 从 `native_rust_core/Cargo.toml` 构建
- 拒绝体积不在 `100,000,000` ~ `250,000,000` bytes 范围内的可疑产物
- 输出：`librustdesk_core.a` 上传为 release asset

**Linux 在线构建**：`.github/workflows/build-core-linux.yml`
- 运行环境：`ubuntu-22.04`
- Rust 工具链：1.88.0
- 仅手动触发（`workflow_dispatch`），不自动触发
- 使用与 Windows 构建相同的依赖和编译逻辑
- 需要设置仓库密钥 `OHOS_SDK_LINUX_ZIP_URL`（Linux 版 OHOS Native SDK 下载地址）

### 产物

- 静态库：`native_rust_core/target/aarch64-unknown-linux-ohos/release/librustdesk_harmony_bridge.a`
- 复制到 HAP 项目时重命名为 `librustdesk_core.a`

## 在 HAP 项目中使用

1. 从 [GitHub Releases](https://github.com/liyan-lucky/librustdesk_core/releases) 下载 `librustdesk_core.a`
2. 复制到 `11_Rustdesk_harmonyos/entry/src/main/libs/arm64/librustdesk_core.a`
3. 复制 `cpp/` 文件到 `11_Rustdesk_harmonyos/entry/src/main/cpp/`（如桥接层有更新）
4. 复制 `cpp/types/` 到 `11_Rustdesk_harmonyos/entry/src/main/cpp/types/`（如 TS 声明有更新）
5. 构建 HAP：`scripts\build_hap.bat`

## 函数名映射

OHOS 对所有 C FFI 函数使用 `rustdesk_bridge_*` 前缀。部分名称与官方 `wire_*` 名称不同：

| OHOS 名称 | 官方名称 | 说明 |
|-----------|---------|------|
| connect_to_peer | session_start | NAPI 保留旧名称，调用新 C 函数 |
| set_incoming_service_enabled | main_start_service | NAPI 保留旧名称 |
| session_alternative_codecs | session_get_alternative_codecs | 重命名以匹配官方 |
| main_use_texture_render | main_get_use_texture_render | 重命名以匹配官方 |

完整映射见 `scripts/rename_mapping.js`。

## 上游兼容性

- 当前版本：RustDesk 1.4.7
- OHOS 目标：`aarch64-unknown-linux-ohos`
- 关键 OHOS 适配：
  - `cfg(target_env = "ohos")` 排除桌面 Linux 依赖
  - `scrap` 不含 wayland/gtk/dbus 特性
  - `arboard` 不含 wayland-data-control 特性
  - 独立的 `rendezvous_mediator_ohos.rs` 用于局域网发现
  - `harmony_bridge/core.rs` 作为会话入口（非 flutter_ffi.rs）

## 当前视频和被控服务状态

- 出站远控会话使用真实 RustDesk 会话路径，通过 `on_rgba -> publish_real_video_frame -> video-frame` 发布视频。
- OHOS 出站观看端视频解码使用 `libvpx` 软解 VP8/VP9 加 `libyuv` YUV 转 RGBA。`codec_ohos.rs` 不得声明 VP9 支持，除非 `handle_video_frame()` 能解码帧并调用 `GoogleImage::to()`。
- 保持 libvpx VP8/VP9 编码器启用，除非重新设计 `scrap` 绑定。`scrap/src/bindings/vpx_ffi.h` 包含 `vp8cx.h` 和 `vpx_encoder.h`，`common/vpxcodec.rs` 引用编码器 API。仅跳过 `libvpxrc.a`；不要禁用 `scrap` 使用的公共 C API 对应的编码器。
- Harmony 入站/被控端屏幕共享尚不可用，因为 desktop server 线程和 Harmony 屏幕采集管线尚未在该目标上接通。
- `main_start_service(true)` 必须在管线缺失时返回 `incomingReady=false` 并给出明确错误。不能仅因为 rendezvous/options 已刷新就标记 incoming ready；那会让远端客户端永远等待一个不存在的视频流。
