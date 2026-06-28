# RustDesk 上游核心对齐审计

> 2026-06-28 当前基线。本文记录当前 HarmonyOS 核心相对 RustDesk 上游核心 / Flutter FFI / 桌面被控端能力的实现差距、未实现项、平台限制和验收边界。本项目为第三方非官方 HarmonyOS / OpenHarmony 适配项目，文中的“上游”仅表示源码来源和兼容目标，不表示官方认可、赞助或背书。

## 当前权威结论

- HarmonyOS 被控端画面传输已由真机实测跑通。
- HarmonyOS 被控端远程输入/操控当前按平台不支持处理，不作为发布阻塞项，也不能在 UI 或状态中宣称支持。
- 文件传输、音频、语音、录制、截图、远程光标、完整菜单状态等仍需逐项端到端验证。
- Windows 和 Linux 均已接入 aarch64 + x86_64 双架构构建。
- 当前 Release 标签统一使用 `core-001`、`core-002`、`core-003` 形式，不再使用 `core-linux-*`。
- 构建启动后立即占用版本号；构建失败只保留标签占号，不创建 Release，不上传正式包。
- 只有 arm64 和 x86_64 两个产物都完整构建并校验通过，才创建正式 Release。

## 状态定义

| 状态 | 含义 |
|------|------|
| 已实现 | 当前 HarmonyOS 核心已经走上游 session/core 路径，且至少有一次构建或端到端验证支撑。 |
| 已实现待复验 | 代码路径已补齐，但缺少当前基线下的真实端到端复验。 |
| 部分实现 | 只完成桥接、事件、状态或单方向能力，尚不能等同上游完整能力。 |
| 未实现 | 当前缺少关键 Rust/C++/ArkTS/平台链路，不能按完整能力对外承诺。 |
| 未测 | 不能根据函数名、符号或构建通过推定可用，必须补真实行为测试。 |
| 平台不支持 | 当前 HarmonyOS 普通应用能力不足，不能作为发布阻塞项，也不能在 UI 中宣称支持。 |
| 禁止回退 | 已经接通上游路径的能力，后续同步、生成或升级时不得退回 stub。 |

## 总体结论

当前 HarmonyOS 核心不是空壳，已经接入 RustDesk 上游 session/core，并形成：

```text
ArkTS UI -> NAPI -> C++ bridge -> Rust C ABI -> librustdesk_core.a -> upstream session/core
```

核心差距不在“能不能构建 core”，而在以下几个方面：

1. HarmonyOS 使用 `harmony_bridge/core.rs` 作为专用入口，不是上游 `flutter_ffi.rs` 的一比一复制，因此每次上游升级都需要重新对照 FFI、Session API 和 UI handler 回调。
2. HarmonyOS 被控端画面传输已实测可用，但远程输入/操控受当前平台能力限制，按不支持处理。
3. 文件传输、终端、聊天、剪贴板、菜单命令等已有桥接或事件路径，但仍需要按真实行为做端到端复验。
4. 音频、语音、录制、截图、远程光标、完整菜单状态属于高风险区，不能仅凭符号导出判断完成。
5. `incomingReady` 不能表示“完整被控能力”，必须拆成画面、输入、服务、采集等独立状态。

## 能力矩阵

| 能力模块 | 当前实现 | 当前状态 | 验收边界 |
|----------|----------|----------|----------|
| 基础连接 / Session 启动 | `connect_to_peer()` 要走上游 `Session`，不能回退 stub。 | 已实现待复验 | 密码错误重试、key、自建服务器、relay、IPv4/IPv6 均需当前基线复验。 |
| 出站远控画面 | `on_rgba -> publish_real_video_frame -> video-frame` 输出，OHOS 侧使用 libvpx/libyuv 软解码。 | 已实现 | 连接后持续获得真实 RGBA 帧，切换质量/编码不崩溃。 |
| 出站远控输入 | 鼠标 mask、键盘和组合键要走 active session。 | 已实现待复验 | 每个输入动作远端真实响应，失败返回 false 或事件。 |
| HarmonyOS 被控端画面 | 用户已实测真机作为被控端，Windows 端可看到持续刷新画面。 | 已实现 | 补 5 分钟稳定性、relay、断线重连、锁屏/后台行为。 |
| HarmonyOS 被控端输入/操控 | 当前按平台不支持处理；输入注入符号仅作兼容和诊断边界。 | 平台不支持 | UI 明确显示 input unsupported；远端不能误以为可控。 |
| 入站服务状态 | `incomingReady/captureRequired` 已有历史演进，但语义需要拆分。 | 部分实现 | 拆成 `serviceReady/screenReady/inputReady/captureRequired`。 |
| 屏幕采集 | App native screen capture 推帧到 core，core 从 incoming frame cache 取帧。 | 已实现待复验 | 分辨率变化、旋转、后台、锁屏、权限撤销需测试。 |
| 编码 / 解码 | 当前重点是 VP8/VP9 + libvpx/libyuv 软解/编码依赖。 | 部分实现 | 只声明真实可用 codec；VP8/VP9 端到端可工作。 |
| 文件传输 | 已接入 job-error/job-done/job-progress/folder-files 等事件。 | 已实现待复验 | 双向文件、目录、覆盖、删除、大文件、中文名、权限错误全部通过。 |
| 终端 | open/input/resize/close 要走上游 Session，输出 base64。 | 已实现待复验 | 输出通过 `dataBase64`，JSON 不被控制字符破坏。 |
| 聊天 | 四参 ABI 对齐 peer_id/message_type/content/timestamp。 | 已实现待复验 | 新四参调用正常，旧一参兼容不破坏内容。 |
| 剪贴板 | active bridge 要构造 Clipboard protobuf 发送。 | 已实现待复验 | 文本、中文、空字符串、双向同步通过。 |
| 远程光标 | 光标形状、位置、显示/隐藏仍是高风险项。 | 待补齐/待复验 | 光标形状和位置随远端变化，开关项真实生效。 |
| 多显示器 | `session_switch_display` 等命令存在并要求返回 bool。 | 已实现待复验 | Windows 多显示器可切换，UI 状态同步。 |
| 菜单命令 | 部分命令已接入 bool 返回和事件回流。 | 部分实现/待复验 | 每个菜单项有成功/失败/不支持状态，UI 不误报。 |
| 语音通话 | 事件有补齐记录。 | 未测 | 双端完整呼叫流程可用，失败有原因。 |
| 音频播放/采集 | `pull_audio_frames_json()` 空队列必须返回 `[]`。 | 未测 | 有声输出/输入，断线恢复，空队列稳定返回 `[]`。 |
| 录制 | record 命令和状态事件有补齐记录。 | 未测 | 能生成录制文件并正确反馈状态。 |
| 截图 | screenshot response 有事件回流记录。 | 未测 | 截图文件生成，UI 收到成功/失败。 |
| LAN 发现 | `rendezvous_mediator_ohos.rs` 有独立适配。 | 已实现待复验 | 同 LAN 可发现，对网络切换有恢复。 |
| IPv4/IPv6/relay | 有混合地址族连接修复记录。 | 已实现待复验 | 地址族不匹配不会卡死，relay 可兜底。 |
| 后台/锁屏/生命周期 | HarmonyOS 侧仍需 App 生命周期配合。 | 部分实现 | 前后台/锁屏场景行为明确，不误报服务 ready。 |
| FFI 覆盖 | Harmony 使用专用 NAPI/C ABI，不是一比一复制。 | 部分实现 | 每次上游升级都有函数级差异清单。 |
| 双架构构建 | Windows/Linux 均接入 arm64 + x86_64，统一 `core-XXX` 标签。 | 已实现待持续验证 | 两个架构产物都成功并校验通过才发布正式包。 |

## 必须保持的边界

1. `harmony_bridge/core.rs` 是 HarmonyOS 专用入口，不是一比一复制上游 Flutter FFI。
2. `incomingReady` 不能表示完整被控能力，应拆分为 `screenReady`、`inputReady`、`captureRequired`、`serviceReady`。
3. HarmonyOS 输入注入按平台不支持，不作为发布阻塞项，也不能在 UI 中宣传可控。
4. 文件传输、音频、语音、录制、截图、远程光标、菜单状态必须用真实端到端行为判定。
5. 命令型 API 必须返回真实执行状态，无 active session 时不能静默 `true`。
6. 旧 mirror 路径不能落后 active bridge。
7. 平台不支持项不能包装成“未测试”。
8. Release 说明不能暗示本项目为上游官方项目或获得官方背书。

## 函数级对齐方法

后续审计按以下顺序进行：

1. 从 RustDesk 1.4.7 上游代码收集 `flutter_ffi.rs`、`Session`、`InvokeUiSession` 和 desktop server 相关能力。
2. 对照 Harmony active bridge：
   - `rustdesk-master/src/harmony_bridge/core.rs`
   - `native_rust_core/src/bridge_api.rs`
   - `cpp/rustdesk_bridge_abi.h`
   - `cpp/rustdesk_bridge_loader.cpp`
   - `cpp/types/librustdesk_bridge/index.d.ts`
3. 每个函数标记 Rust 是否调用上游实现、C ABI 是否暴露、C++ NAPI 参数是否正确、d.ts 是否签名一致、ArkTS UI 是否真实调用并处理返回值/事件。
4. 没有真实验收用例的功能只能标记为“已实现待复验”或“未测”。
