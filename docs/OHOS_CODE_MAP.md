# OHOS 专属代码分布说明

> 本文档记录 `rustdesk-master/` 中与官方 RustDesk 1.4.7 不同的 OHOS 专属代码位置，便于后续更新官方源码时快速定位和合并。

> 2026-06-21 23:23 当前映射已集成到 arm64/x86_64 候选库并进入固定 HAP；当前哈希和验证证据以 `CORE.md` 为准。华为被控输入/accessibility 不再是活动实现路径，仅保留 App 侧返回 `201` 的链接兼容 stub。

## 代码分类

### A. 已移入 `harmony_bridge/` 目录的 OHOS 替代文件

这些文件是官方同名模块的 OHOS 替代实现，通过 `#[path = "harmony_bridge/xxx_ohos.rs"]` 在 `cfg(target_env = "ohos")` 时加载。

#### `src/harmony_bridge/` 目录（9 个文件）

| 文件 | 替代的官方模块 | 行数 | 说明 |
|------|---------------|------|------|
| `keyboard_ohos.rs` | `keyboard.rs` | 84 | 键盘输入处理 OHOS stub |
| `platform_ohos.rs` | `platform.rs` | 156 | 平台抽象 OHOS 实现 |
| `server_ohos.rs` | `server/` 目录 | 146 | 服务端 OHOS 精简实现 |
| `rendezvous_mediator_ohos.rs` | `rendezvous_mediator.rs` | 469 | 信令中介 OHOS 实现 |
| `ipc_ohos.rs` | `ipc.rs` | 73 | IPC 通信 OHOS stub |
| `clipboard_ohos.rs` | `clipboard.rs` | 112 | 剪贴板 OHOS stub |
| `clipboard_master_ohos.rs` | `clipboard_master.rs` | 6 | 剪贴板监听 OHOS stub |
| `clipboard_file_ohos.rs` | `clipboard_file.rs` | 21 | 文件剪贴板 OHOS stub |
| `ui_interface_ohos.rs` | `ui_interface.rs` | 80 | UI 接口 OHOS 实现 |

#### `src/harmony_bridge/` 目录（核心桥接，2 个文件）

| 文件 | 行数 | 说明 |
|------|------|------|
| `core.rs` | ~2900 | HarmonyOS 核心桥接：会话管理、事件队列、帧缓存、文件传输等 |
| `mod.rs` | 3 | 模块导出 |

#### `libs/scrap/src/common/harmony_bridge/` 目录（3 个文件）

| 文件 | 替代的官方模块 | 行数 | 说明 |
|------|---------------|------|------|
| `ohos.rs` | `x11.rs` / `dxgi.rs` 等 | 241 | OHOS 屏幕采集和帧缓存 |
| `codec_ohos.rs` | `codec.rs` | 391 | OHOS 视频编解码器 |
| `record_ohos.rs` | `record.rs` | 81 | OHOS 录制 stub |

### B. 散布在官方文件中的 `cfg(target_env = "ohos")` 条件编译（不可转移）

这些代码片段嵌入在官方源文件中，通过 `#[cfg(target_env = "ohos")]` 条件编译启用，无法移到独立目录。

#### `src/lib.rs`（9 处 cfg）

所有 `#[path = "harmony_bridge/xxx_ohos.rs"]` 声明和 `cfg(target_env = "ohos")` 排除块。

#### `src/client.rs`（5 处 cfg）

- 行 5: 排除 OHOS 的 `key_down` 模块导入
- 行 7: OHOS 专用 `key_down` 导入
- 行 1349: OHOS 专用客户端逻辑
- 行 1427: OHOS 专用连接处理

#### `src/lan.rs`（11 处 cfg）

- 行 112: `get_ohos_subnet_broadcasts()` 函数
- 行 209-458: OHOS 专用 LAN 发现逻辑，调用 `harmony_bridge::core::queue_event`

#### `src/hbbs_http.rs`（1 处 cfg）

- 行 9: 排除 OHOS 的 HTTP 模块

#### `src/hbbs_http/http_client.rs`（1 处 cfg）

- 行 22-26: OHOS 专用 HTTP 客户端逻辑

#### `src/lang.rs`（1 处 cfg）

- 行 112: OHOS 语言处理

#### `src/server/video_service.rs`（1 处 cfg）

- 行 1058: OHOS 视频服务逻辑

#### `src/client/file_trait.rs`（4 处 cfg）

- OHOS 文件传输 trait 实现

#### `src/client/io_loop.rs`（3 处 cfg）

- OHOS IO 循环处理

#### `libs/hbb_common/` 下多个文件

- `src/config.rs`: 5 处 cfg（移动端/桌面端配置区分）
- `src/lib.rs`: 1 处 cfg（排除 OHOS 的 Linux 模块）
- `src/proxy.rs`: 8 处 cfg（OHOS 代理处理）
- `src/websocket.rs`: 6 处 cfg（OHOS WebSocket 处理）

#### `libs/scrap/` 下

- `build.rs`: 2 处（OHOS 构建配置）
- `src/common/mod.rs`: 6 处 cfg（OHOS 模块路由）
- `src/common/camera.rs`: 多处 cfg（排除 OHOS 的摄像头功能）

## 更新官方源码流程

1. **替换上游源码**：将新版本 RustDesk 源码解压到 `rustdesk-master/`
2. **恢复 OHOS 条件编译**：在对应官方文件中重新添加 `cfg(target_env = "ohos")` 代码块
3. **恢复 `#[path]` 引用**：在 `lib.rs` 中恢复所有 `#[path = "harmony_bridge/xxx_ohos.rs"]` 声明
4. **恢复 `harmony_bridge/` 目录**：从 git 历史恢复整个 `harmony_bridge/` 目录
5. **恢复 `libs/scrap/src/common/harmony_bridge/` 目录**：同上
6. **更新 `libs/scrap/src/common/mod.rs`**：恢复 OHOS 模块路由
7. **更新 Cargo.toml**：恢复 OHOS 相关的 patch 和 feature 排除
8. **编译验证**：`powershell -File scripts/build_native_bridge.ps1`
