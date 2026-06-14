# librustdesk_core 文档索引

> 核心项目文档。所有核心相关的架构、编译、桥接函数、调试文档均在此维护。

> 2026-06-13 经验：如果手机端已 `session-connected` 且 `quality-status` 显示 `codec_format=VP9`，但没有 `video-frame`，先看 `CORE.md` 的 OHOS VP8/VP9 解码修复记录和 `CONNECTION_DEBUG_LOG.md`，不要只查 `session_next_rgba()`。

> 2026-06-13 CI note: if the online Windows runner fails building libvpx with `<cstdint>` not found, do not fall back to MSYS2 libc++ for OHOS clang. Run `27458902852` showed MSYS2 libc++ can be too new for the SDK clang. Build only the `libvpx.a` target and manually install the public headers, because the failing C++ RTC sources are for unused `libvpxrc.a`. Do not disable libvpx VP8/VP9 encoders unless `scrap` bindings and `common/vpxcodec.rs` are changed too.

> 2026-06-14 bridge note: terminal open/input/resize/close must call official `Session` and terminal data must travel through events as base64 `dataBase64`; empty audio frame queues return `[]`. Chat NAPI four-argument calls read content from `args[2]`.

> 2026-06-14 file-transfer note: checking ArkTS/NAPI/C ABI names is not enough. `InvokeUiSession` callbacks must emit the app listener events (`folder-files`, `file-transfer-start`, `job-progress`, `job-done`, `job-error`, `create-remote-dir`, `delete-remote-path`) and transfer start must expose the same `job_id` used by official `send_files()`.

> 2026-06-14 UI-route note: if the app has both a direct bridge function and a generic option helper, check which one the UI actually calls. `switch-sides` is routed through `apply_session_option()` by the RemoteControl menu and must call official `Session::switch_sides()`.

> 2026-06-14 release note: commit `38c837cee0bb28aee795c0fc3895044f1440f96a` was published by run `27483922931` as `core-71`; asset size `131,297,004` bytes, SHA256 `C750A785297AA22A2518B158BF334A1B1415C4E0739E01D0856C8BB5D450E15C`. Build the core from the real `%VSCODE_ROOT%\13_librustdesk_core` path, not from the app project's junction.

> 2026-06-14 release note: commit `275b231e11aefd4a2e51050fc74fbdeba9c566bd` was published by run `27485061967` as `core-73`; asset size `131,471,532` bytes, SHA256 `E444D739EC958CD1485519FE0A712BFC1F074B60EEA65D71552E7E95A909A7B1`. The app downloaded this release and full HAP/package verification passed; runtime launch was blocked only by the phone lock screen.

## 文档列表

| 文件 | 说明 |
|------|------|
| `CORE.md` | 核心架构、可复现编译、桥接函数完整说明（369个函数）、CMake链接、编译问题 |
| `LESSONS_LEARNED.md` | 经验教训和易复发构建问题 |
| `BUILD_ARCHIVE.md` | 历史构建、脚本、Ubuntu路径和早期会话归档 |
| `CONNECTION_DEBUG_LOG.md` | 连接问题逐轮排查记录 |
| `UBUNTU_CROSS_COMPILE_GUIDE.md` | Ubuntu 交叉编译指南 |
| `SESSION3_SUMMARY.md` | 会话3总结 |
| `WINDOWS_SERVICE_OPTIMIZATION.md` | Windows 服务优化 |
| `FUNCTION_LOGIC_AUDIT_2026-06-05.md` | 功能逻辑审计(6月5日) |
| `FUNCTION_LOGIC_AUDIT_2026-06-06.md` | 功能逻辑审计(6月6日) |

## 核心修改流程

1. 在本项目中修改 Rust/C++/TS 桥接代码
2. 运行代码生成脚本（如需要）：`node scripts/regenerate_all.js`
3. 本地验证编译：`powershell -File scripts/build_native_bridge.ps1`
4. Git push 到远端
5. GitHub Actions 自动用 Cargo `release` profile 构建，生成 `librustdesk_core.a`
6. 下载 Release 产物，放入 HAP 项目 `entry/src/main/libs/arm64/`
7. 同步 `cpp/` 文件到 HAP 项目 `entry/src/main/cpp/`（如桥接层有更新）

> 发布前必须检查 `.a` 体积。当前 release 基准约 `132 MiB`；如果 GitHub Actions 产物接近 `568 MiB`，优先检查 workflow 是否误用了 Cargo `dev` profile 或保留了 debug 符号。

## HAP 项目（11_Rustdesk_harmonyos）文档

HAP 项目保留的文档聚焦于应用层：

| 文件 | 说明 |
|------|------|
| `AGENT_MEMORY.md` | AI助手工作规则、经验库、用户偏好 |
| `CORE.md` | 精简版：核心状态、HAP构建安装、运行验证清单 |
| `DESIGN.md` | UI/构建/真机测试设计约束 |
| `UI.md` | UI布局、图标、核心页卡片细节 |
| `FILES.md` | 文件职责和外部依赖目录 |
| `PROGRESS.md` | 功能进度、已完成事项、重点问题 |
| `ISSUES.md` | 问题库和易复发坑 |
| `GIT_PUBLISH.md` | GitHub发布说明 |
