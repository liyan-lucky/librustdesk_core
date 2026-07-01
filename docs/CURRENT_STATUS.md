# 当前仓库状态

更新时间：2026-07-01

## 定位

`librustdesk_core` 是 RustDesk HarmonyOS / OpenHarmony 适配链路中的核心静态库构建仓库。它包含 Rust 桥接层、C++ NAPI 桥接层、上游 RustDesk 源码副本、OHOS 补丁和 CI/CD 构建脚本，用于生成 HarmonyOS App 可链接的 `librustdesk_core.a`。

本项目是第三方非官方适配项目，不代表上游 RustDesk 官方发布、认可、赞助或背书。

## 当前能力边界

- 上游兼容目标：RustDesk 1.4.7。
- 发布标签：统一使用 `core-001`、`core-002`、`core-003` 形式。
- Windows 和 Linux 构建共用同一套版本编号。
- arm64 与 x86_64 两个产物都完整生成并通过校验后，才创建 Release 并上传正式包。
- Linux 构建可由 `main` 更新自动触发，也可手动触发；Windows 构建保持手动触发。
- HarmonyOS 入站/被控端画面传输已由真机实测跑通。
- HarmonyOS 入站/被控端远程输入/操控当前按平台不支持处理，不作为发布阻塞项，也不能在 UI 或状态中宣称支持。
- 文件传输、音频、语音、录制、截图、远程光标、完整菜单状态等仍按未逐项端到端验证处理。

## 当前目录职责

- `native_rust_core/`：Rust 桥接层。
- `rustdesk-master/`：上游 RustDesk 源码副本与 OHOS 适配改动。
- `patches/`：OHOS 特定 crate 补丁。
- `rdev-fork/`：输入库 fork，供核心适配链路使用。
- `cpp/`：C++ NAPI 桥接层。
- `scripts/`：构建脚本和代码生成器。
- `docs/`：架构、差异、合规、构建和验收文档。

## 当前分支和备份

- `main`：当前主工作分支。
- `backup`：`main` 的快照备份分支。
- `.github/workflows/force-backup-main.yml`：手动输入 `YES` 后，把 `main` 当前提交强制覆盖到 `backup`。

## 当前 CI/CD

- `.github/workflows/build-core-windows.yml`：Windows 双架构构建。
- `.github/workflows/build-core-linux.yml`：Linux 双架构构建，需要 `OHOS_SDK_LINUX_ZIP_URL`。
- `.github/workflows/auto-linux-core-build.yml`：`main` 更新后自动触发 Linux 构建。
- `.github/workflows/update-core-release-notes.yml`：构建成功后更新 Release 说明。
- `.github/workflows/cleanup-releases.yml`：手动清理 Release、core 标签和旧 Actions 运行记录。
- `.github/workflows/force-backup-main.yml`：手动强制刷新 `backup` 分支。

## 合规边界

- 不使用“官方项目”“官方发布”“官方授权”等暗示上游背书的表述。
- 上游项目名称仅用于说明源码来源和兼容目标。
- 发布静态库时应保留源码、构建脚本、补丁、第三方声明和许可证说明。
- 许可入口见 `LICENSE_NOTICE.md`、`COPYING`、`NOTICE` 和 `docs/THIRD_PARTY_NOTICES.md`。

当前事实变化时，应同步更新本文件、根 README 和 `docs/README.md`。
