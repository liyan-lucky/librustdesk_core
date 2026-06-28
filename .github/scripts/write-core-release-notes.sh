#!/usr/bin/env bash
set -euo pipefail

output_path="${1:-release-notes.md}"
release_number="${RELEASE_NUMBER:?RELEASE_NUMBER is required}"
release_tag="${RELEASE_TAG:?RELEASE_TAG is required}"
build_os="${BUILD_OS:?BUILD_OS is required}"

if [ "$release_number" = "001" ]; then
  cat > "$output_path" <<EOF
# RustDesk HarmonyOS 原生核心首发版

## 重要声明

本项目为第三方非官方 HarmonyOS / OpenHarmony 适配项目，不代表上游项目官方发布、认可、赞助或背书。上游项目名称和相关标识仅用于说明源码来源和兼容目标。

本发布包包含基于上游源码构建的原生静态库。对应源码、构建脚本、补丁和桥接层代码可在本仓库对应 tag / commit 中获取。使用、修改或再分发本发布包时，请同时遵守上游许可证以及相关第三方依赖许可证要求。

## 简介

这是 librustdesk_core 的第一个正式发布包，面向 HarmonyOS / OpenHarmony 应用侧集成远程控制核心能力。

本发布包基于上游核心进行 HarmonyOS 适配，输出可被 HAP 工程引用的原生静态库文件。它主要用于 OpenRustDesk / HarmonyOS 客户端中的 C++ NAPI 桥接层，让 ArkTS 应用侧能够调用底层 core 能力。

## 当前定位

本仓库发布的不是完整 App，而是 HarmonyOS 端原生核心库。

- 提供上游核心能力的 HarmonyOS 静态库封装。
- 配合当前仓库的 C++ NAPI、ArkTS 类型定义和 App 侧调用逻辑使用。
- 支持 arm64 真机调试和 x86_64 虚拟设备调试。
- 用于后续 HAP 构建、会话连接、画面传输、文件传输等功能集成。

## 已包含产物

- librustdesk_core.a：arm64-v8a，适用于 HarmonyOS 真机调试和实机集成。
- librustdesk_core_x86_64.a：x86_64，适用于 HarmonyOS / OpenHarmony 虚拟设备调试。

两个产物必须同时存在并通过大小校验后才会创建正式 Release。构建中间任意步骤失败时，只保留版本标签占号，不创建 Release，也不上传正式发布包。

## 当前能力说明

当前核心已围绕 HarmonyOS 适配上游核心路径，重点包含：

- 上游 session/core 连接链路。
- 出站远控会话基础能力。
- HarmonyOS 端原生核心静态库构建。
- arm64 + x86_64 双架构产物输出。
- C++ NAPI / ArkTS 桥接层可集成的核心库形态。
- HarmonyOS 被控端画面传输相关适配基础。
- 文件传输、终端、聊天、剪贴板等能力的核心桥接基础。

部分能力仍需要结合 App 侧、权限、UI 和真实设备继续做端到端验收；具体以仓库文档中的上游核心对齐审计和后续测试记录为准。

## 构建信息

- 发布标签：$release_tag
- 发布编号：$release_number
- 构建环境：$build_os
- Rust 工具链：1.88.0
- 构建配置：Cargo release profile
- 目标平台：aarch64-unknown-linux-ohos + x86_64-unknown-linux-ohos

## 使用方式

下载后按架构放入 HAP 工程对应目录：

- arm64 真机库：entry/src/main/libs/arm64-v8a/librustdesk_core.a
- x86_64 虚拟设备库：entry/src/main/libs/x86_64/librustdesk_core.a 或按项目当前桥接配置命名使用

需要与当前仓库中的 C++ NAPI 桥接层、ArkTS 类型定义和 App 侧调用逻辑配套使用。

## 发布规则

- 版本标签统一使用 core-001、core-002、core-003 形式。
- Windows 和 Linux 构建共用同一套编号。
- 构建启动后立即占用版本号。
- 构建失败会保留标签占号，但不会创建 Release，也不会上传正式包。
- 只有 arm64 和 x86_64 两个产物都完整构建并校验通过，才会发布正式包。

## 备注

这是首个正式编号版本，主要用于建立稳定的发布编号、双架构产物格式和 HarmonyOS 原生核心集成基线。后续版本将根据实际变更、修复内容和功能验收情况更新发布说明。
EOF
else
  cat > "$output_path" <<EOF
# RustDesk HarmonyOS 原生核心更新版

## 重要声明

本项目为第三方非官方 HarmonyOS / OpenHarmony 适配项目，不代表上游项目官方发布、认可、赞助或背书。上游项目名称和相关标识仅用于说明源码来源和兼容目标。

本发布包对应源码、构建脚本、补丁和桥接层代码可在本仓库对应 tag / commit 中获取。使用、修改或再分发本发布包时，请同时遵守上游许可证以及相关第三方依赖许可证要求。

## 简介

本次发布为 librustdesk_core 的后续更新包，用于 HarmonyOS / OpenHarmony 应用侧集成远程控制核心能力。

该版本延续首发版的双架构发布格式，继续输出 arm64 真机调试库和 x86_64 虚拟设备调试库。

## 本次更新

- 保持 HarmonyOS 原生核心静态库发布格式。
- 保持 arm64 + x86_64 双架构产物输出。
- 保持统一版本标签：$release_tag。
- 具体功能变更、问题修复和验收状态请以对应提交记录、仓库文档和测试记录为准。

## 构建信息

- 发布标签：$release_tag
- 发布编号：$release_number
- 构建环境：$build_os
- Rust 工具链：1.88.0
- 构建配置：Cargo release profile
- 目标平台：aarch64-unknown-linux-ohos + x86_64-unknown-linux-ohos

## 产物说明

- librustdesk_core.a：arm64-v8a，适用于 HarmonyOS 真机调试和实机集成。
- librustdesk_core_x86_64.a：x86_64，适用于 HarmonyOS / OpenHarmony 虚拟设备调试。

## 发布规则

- Windows 和 Linux 构建共用同一套 core-XXX 编号。
- 构建启动后立即占用版本号。
- 构建失败只保留标签占号，不创建 Release，不上传正式包。
- 只有 arm64 和 x86_64 两个产物都完整构建并校验通过，才会发布正式包。
EOF
fi
