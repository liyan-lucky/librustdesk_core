# librustdesk_core

> 免责声明：本项目为第三方非官方 HarmonyOS / OpenHarmony 适配项目，不属于上游项目官方发布、认可、赞助或背书的项目。上游项目名称和相关标识仅用于说明源码来源和兼容目标，相关权利归其各自权利人所有。
>
> 许可证与源码说明：本仓库包含基于上游源码的 HarmonyOS 适配、桥接和构建脚本。使用、修改、分发本仓库源码或发布产物时，请同时遵守上游许可证以及相关第三方依赖许可证要求。许可入口见 `LICENSE_NOTICE.md`、`COPYING`、`NOTICE` 与 `docs/THIRD_PARTY_NOTICES.md`。

RustDesk HarmonyOS 原生核心静态库构建器。从 RustDesk 1.4.7 上游源码通过 OHOS 交叉编译构建 `librustdesk_core.a`，并生成 C++ NAPI 桥接层，供 HarmonyOS ArkTS 应用集成使用。

[English](README_EN.md)

## 当前状态

- 当前发布标签统一使用 `core-001`、`core-002`、`core-003` 形式。
- Windows 和 Linux 构建共用同一套版本编号。
- 构建启动后立即预留版本号；失败也会保留标签占号。
- 只有 arm64 和 x86_64 两个产物都完整生成并通过校验，才创建 Release 并上传正式包。
- Release 说明只更新介绍内容，不改标签和 Release 名称。
- Linux 可由 main 分支更新自动触发，也可手动触发；Windows 保持手动触发。

## 架构

```
ArkTS UI (11_Rustdesk_harmonyos)
    -> NAPI
librustdesk_bridge.so
    -> C++ 桥接加载器 (cpp/)
    -> Rust C ABI (native_rust_core/)
librustdesk_core.a
    -> rustdesk_harmony_bridge
    -> RustDesk 上游 session/core (rustdesk-master/)
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
| `docs/` | 架构、差异、合规和验收文档 |

## 构建

### Windows

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build_native_bridge.ps1
```

### Linux

```bash
./scripts/build_native_bridge.sh aarch64-unknown-linux-ohos release
./scripts/build_native_bridge.sh x86_64-unknown-linux-ohos release
```

## CI/CD

### Windows 在线构建

- Workflow：`.github/workflows/build-core-windows.yml`
- 运行环境：`windows-2022`
- 触发方式：手动触发（`workflow_dispatch`）
- 目标平台：`aarch64-unknown-linux-ohos` + `x86_64-unknown-linux-ohos`
- 输出：`librustdesk_core.a` + `librustdesk_core_x86_64.a`

### Linux 在线构建

- Workflow：`.github/workflows/build-core-linux.yml`
- 自动触发器：`.github/workflows/auto-linux-core-build.yml`
- 运行环境：`ubuntu-22.04`
- 触发方式：main 更新后自动触发，也支持手动触发
- 目标平台：`aarch64-unknown-linux-ohos` + `x86_64-unknown-linux-ohos`
- 输出：`librustdesk_core.a` + `librustdesk_core_x86_64.a`
- 需要仓库密钥：`OHOS_SDK_LINUX_ZIP_URL`

### Release 说明

- 模板脚本：`.github/scripts/write-core-release-notes.sh`
- 自动更新：`.github/workflows/update-core-release-notes.yml`
- `core-001` 使用首版详细说明。
- `core-002` 以后使用更新说明模板。
- 仅更新 Release 介绍内容，不修改标签和 Release 名称。

## 发布产物

| 文件 | 架构 | 用途 |
|------|------|------|
| `librustdesk_core.a` | arm64-v8a | HarmonyOS 真机调试和实机集成 |
| `librustdesk_core_x86_64.a` | x86_64 | HarmonyOS / OpenHarmony 虚拟设备调试 |

两个产物必须同时存在并通过体积校验后才会创建正式 Release。构建中间任意步骤失败时，只保留版本标签占号，不创建 Release，也不上传正式发布包。

## 在 HAP 项目中使用

1. 从 GitHub Releases 下载对应架构静态库。
2. 复制到 HAP 工程对应 ABI 目录，例如 `entry/src/main/libs/<ABI>/`。
3. 与当前仓库中的 C++ NAPI 桥接层、ArkTS 类型定义和 App 侧调用逻辑配套使用。
4. 如桥接层有更新，同步复制 `cpp/` 和 `cpp/types/`。

## 当前能力边界

- 出站远控会话使用真实上游会话路径，通过 `on_rgba -> publish_real_video_frame -> video-frame` 发布视频。
- OHOS 出站观看端视频解码使用 `libvpx` 软解 VP8/VP9 加 `libyuv` YUV 转 RGBA。
- HarmonyOS 入站/被控端画面传输已由真机实测跑通。
- HarmonyOS 入站/被控端远程输入/操控当前按平台不支持处理：不要把输入注入作为发布阻塞项，也不要在 UI 或状态中宣称可控。
- 文件传输、音频、语音、录制、截图、远程光标、完整菜单状态等非画面能力仍按“未完成端到端验证”处理。

详细能力差异和验收边界见：`docs/OFFICIAL_CORE_GAP.md`。

## 合规说明

- 本项目不是上游官方项目。
- 上游项目名称仅用于说明源码来源和兼容目标。
- 发布静态库时，应同时保留对应源码、构建脚本、补丁、第三方声明和许可证说明。
- 许可入口见：`LICENSE_NOTICE.md`、`COPYING`。
- 第三方声明见：`NOTICE`、`docs/THIRD_PARTY_NOTICES.md`。
- 上游许可证文本保留在：`rustdesk-master/LICENCE`。

## 历史记录

旧的测试包编号、历史 SHA、历史包大小和历史 Release 信息不再作为当前主说明展示。当前有效发布规则以本 README 的 `core-XXX` 统一编号规则和 CI/CD 说明为准。
