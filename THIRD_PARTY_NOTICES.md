# 第三方组件与许可记录

YYPlayer 自身源码及原创 UI 资源采用 **GPL-3.0-only**，见 [LICENSE](LICENSE)。该声明不替代第三方组件原有的版权与许可。

| 组件 | 使用方式 | 许可来源 / 当前处理 |
| --- | --- | --- |
| Slint / slint-build 1.17.1 | Rust UI 与构建工具 | 本项目选择 GPL-3.0-only 分支；框架的其他可选许可证不作为本项目的默认授权。见 [锁定版本的上游说明](https://github.com/slint-ui/slint/blob/v1.17.1/LICENSE.md)。 |
| libmpv `20260928-git-e470f8986e` | Windows 开发预览动态库 | 固定构建 manifest 标为 GPL 组合；仅提交获取脚本 / hash / 来源，不提交 DLL。见 [manifest](third_party/mpv/manifest.json) 和 [运行时记录](third_party/mpv/README.md)。 |
| Ubuntu libmpv 0.41.0 / ELF 依赖 | Linux deb 使用系统包；AppImage 携带非驱动依赖 | [Linux 策略](third_party/mpv/linux.json)；打包时记录实际发行版本、原始 / 打包 hash、精确对应源码页面，并保存完整发行包 copyright 与 common-licenses。 |
| type2-runtime / appimagetool 1.9.1 | Linux AppImage 封装 | 工具归档 SHA-256 固定；内嵌 runtime 为 caf24f9，完整原文及静态闭包限制见 [runtime notices](packaging/linux/runtime-notices/README.md)。 |
| 其他 Rust 依赖 | Cargo.lock 锁定版本 | 保留各上游许可证；本轮 Windows normal / build 依赖的元数据清单见 [依赖许可元数据](docs/dependency-licenses.md)。Cargo manifest 的表达式不能代替每个组件的完整 notices。 |
| GD 云端 API 示例 | 与 Android 共用的声明式 JSON | 原文件的 attribution / usage 保留：GD音乐台、CC BY-NC 4.0、学习参考用途；该说明不改变播放器源码许可，见 [配置](config/cloud_api/example-gd.json)。 |
| 图标 / 演示封面 | 编译进 Slint 的本地 SVG | ADR 0000 记录为原创 SVG，随本项目源码授权；验收截图使用自有素材。用户附件、专辑图片、音视频与 APPDATA 配置不入仓库。 |

Windows 固定 mpv 构建的所有 FFmpeg / 编解码组件 notices、对应源码 / 构建说明与 DLL 依赖闭包仍未完成。Linux 开发包另带实际工作树、锁定 Rust crate 源码、原始 Rust notices、发行包对应源码 / 许可清单和固定 AppImage runtime 源码。其上游预构建 runtime 的精确 Alpine 静态版本 / 可重链接安排仍待补证。正式公开二进制发行前按 [开发规划 S00](docs/DEVELOPMENT_PLAN.md) 完成各平台的发行资格；本页不作为完整许可审计结论。

本地已安装的 Cargo 依赖通常含 LICENSE / LICENSES 文件；源仓库中不复制整个 registry 或 target。公开源码的许可证选择不改变依赖各自的授权条件。
