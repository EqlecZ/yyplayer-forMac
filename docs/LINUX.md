# Ubuntu 26.04 开发与打包

Linux 与 Windows 共用 core、controller、Slint 界面和 libmpv GL presenter，通过目标平台 cfg 选择实现及依赖。Linux 当前验证目标是 Ubuntu 26.04 amd64；实际资格以 [Linux 验收](validation/linux.md) 和 [STATUS](STATUS.md) 为准。Windows 的 WASAPI 名称与确认规则不会用于 Linux。

Linux 适配起于 `dev-linux`，已于 `5390a3d` 合入 `master`；直接在 `master` 构建即可，无需按系统切换分支。0.3.0 的 Windows 自动编译打包见 [打包记录](validation/windows-packaging.md)，此前用户反馈见 [共用源码合并准备记录](validation/shared-source-merge-preparation.md)；本轮版本更新没有新增 Linux 实机验收。

## 构建

需要 Rust >= 1.92（本轮工具链 1.99.0），Slint / slint-build 仍固定 1.17.1。先执行只读审计：

```bash
python3 scripts/check-linux-deps.py
```

审计输出本机缺少的 apt 包及一条完整安装命令。libmpv 是动态绑定，不需要安装 libmpv-dev 的完整编解码器开发依赖树。Ubuntu 26.04 的 `libmpv2` 为 0.41.0，使用系统发行版的安全更新；固定来源与打包策略见 `third_party/mpv/linux.json`。

```bash
cargo build --locked --release -p yyplayer-app --bin yyplayer
./target/release/yyplayer
./target/release/yyplayer '/home/user/视频/示例.mp4'
```

Winit + FemtoVG 同时构建原生 Wayland 和 X11。诊断时可使用 `WINIT_UNIX_BACKEND=wayland` 或 `WINIT_UNIX_BACKEND=x11`；不要用软件渲染器替代生产 GPU 视频路径。

## 桌面与输出

- 设置保存在 `${XDG_CONFIG_HOME:-$HOME/.config}/YYPlayer/settings.json`，音乐与视频索引分开。`YYPLAYER_CONFIG` 仅用于独立开发配置。deb 与 AppImage 使用同一用户配置；卸载程序不会删除它或媒体文件。
- 文件选择使用 XDG desktop portal，在后台对话框服务运行。主题、系统强调色与减少动画通过只读 Settings portal 查询；没有偏好或接口缺失时保留应用回退。字体在后台枚举。
- 共享模式沿用已验证 Ubuntu runtime 的自动 AO 优先顺序（本机为 PipeWire），兼容 PulseAudio / ALSA；不设置会覆盖具体设备后端的固定 `ao` 列表。自动采样率、70% 音量和无额外 DSP 默认保持。设备下拉显示具体桌面设备和按稳定 card ID 枚举的 ALSA `hw:` 直连设备。
- Linux 独占请选择具体设备。PipeWire 独占结合 mpv 0.41 AO 的 exclusive flag、成功流连接与实际 `current-ao`；它是独占流语义，不等同直接占用 DAC。ALSA `hw:` 必须结合实际打开设备、成功 HW 参数协商与实际 AO，`plughw:` / `dmix:` 不能显示硬件独占已确认。
- 优先独占只回退同一个明确设备的共享路径。PulseAudio 不提供这里的独占能力，优先模式可直接透明回退同设备共享，严格模式暂停。ALSA 直连失败不会猜测对应的桌面 sink，也不会切换扬声器。
- 设备失联暂停；输出重试恢复位置并保留暂停。请求独占、API PCM 和物理 DAC 格式分开。源采样率保护比较源与 API 输出；数字位准确仍需真实数字捕获。
- 原始非 UTF-8 文件名以带版本的字节身份保存在 JSON 中；正常 UTF-8 配置仍沿用旧字符串。播放时 mpv file URI 恢复原始字节，FILE_LOADED 用原始 bytes 比较，截图目标也使用原始 bytes。
- 自绘窗口按钮和拖动 / 边缘缩放继续调用 Winit 的原生动作。圆角与阴影由具体 Linux compositor 决定，Windows DWM 角部验收不能用于 Linux。HDR 显示输出、USB DAC / 位准确与多屏睡眠等资格按验收报告区分。

## 回归

所有脚本使用自有静音音频 / 色条视频与 `target/` 独立配置。不会修改默认用户设置、系统主题或音频服务配置。

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --all-targets
python3 scripts/test-linux.py --backend wayland --suite all --require-hardware
python3 scripts/test-linux.py --backend x11 --headless --suite ui --skip-build
# 选择本机实际 hw: 设备；结果明确区分成功协商与占用拒绝：
python3 scripts/test-linux.py --backend wayland --suite alsa --alsa-device 'alsa/hw:CARD=Generic,DEV=0' --skip-build
```

脚本区分 UI fixture、生产 controller、实际 libmpv 播放、GPU 图片比较和真实 native window handle。`--headless` X11 使用独立 Xvfb；Wayland 使用独立 Weston。headless / XWayland 不替代真实桌面多屏、物理输入和 GPU / 音频设备资格。`--require-hardware` 不允许软解结果伪装成硬解通过。

## 生成 deb 和 AppImage

```bash
python3 scripts/package-linux.py --check-tools
python3 scripts/package-linux.py
# 已缓存工具、已构建 Release 时可完全离线重包：
python3 scripts/package-linux.py --offline --skip-build
```

`--check-tools` 只读检查全部宿主打包命令，不下载或编译。清单集中在 `packaging/linux/tools.json`，本地 apt 审计复用同一清单；缺项一次性列出命令和对应 apt 包。`file` 是固定版 appimagetool 的必需宿主命令，不能依赖开发桌面已安装它；CI 明确安装。正常打包入口也会先检查工具，含 `--skip-build` 模式。固定工具内置的 mksquashfs / desktop-file-validate / zsyncmake 已单独核对，见 [打包工具验证记录](validation/linux-packaging-tools.md)。

脚本在编译前执行 `cargo fetch --locked`，下载锁定依赖图中所有平台的源码，用于完整许可记录与对应源码包；Linux 编译目标保持不变。CI 也在 lint / tests 前完成这一步。不要给这次 fetch 加 `--target`，否则其他平台的条件依赖可能缺失。`--offline` 会改为 `cargo fetch --locked --offline`，要求源码和打包工具缓存齐全；`--skip-build` 也会检查源码缓存。说明依据见 [Cargo fetch 文档](https://doc.rust-lang.org/cargo/commands/cargo-fetch.html)。

输出在 `dist/`：

- `yyplayer_<version>_amd64.deb`：安装 `/usr/bin/yyplayer`、桌面入口 / MIME 和图标，依赖系统 `libmpv2 >= 0.41.0`。不强抢默认文件关联，没有删除用户配置的卸载脚本。
- `YYPlayer-<version>-x86_64.AppImage`：type-2 AppImage，携带经 SHA-256 校验的 libmpv、非驱动 ELF 依赖、PipeWire 客户端模块与 SPA 插件；宿主提供 glibc、GPU 驱动 / GL / Vulkan、字体、portal 和音频服务。在 Ubuntu 26.04 构建不代表能运行于更老的发行版。
- `YYPlayer-linux-source.tar.gz`：编译对应的实际工作树源码和锁定 Rust crate 原始源码归档；`YYPlayer-linux-rust-sources.json` 列出版本、许可和归档摘要。
- `YYPlayer-linux-runtime-manifest.json`：每个播放依赖的原始 / 打包 hash、发行包版本、对应 source package 和精确源码页面；AppImage 包内保留每个发行包完整 copyright 和许可原文。
- `YYPlayer-linux-BUILD-INFO.json` 与 `SHA256SUMS-linux`：构建平台、工具链、dirty 状态、程序 / 源码校验和。

GitHub 容器内 checkout 可能与当前执行用户的所有者不同。打包脚本的 Git 查询对当前仓库路径使用命令级 `safe.directory`，不需要手动更改全局 Git 配置；先检查提交号，再执行 Release 构建。该问题及回归记录见 [CI 打包修复](validation/linux-packaging-git.md)。

AppImage 工具固定 1.9.1 和 SHA-256，缓存到项目 target。使用这个已校验工具本身包含的 runtime，不下载可变的 continuous runtime。源码、Rust 源码清单、runtime 源码与许可清单须与二进制一同提供；上游发行包的精确源码获取页面以清单为准。

安装包验收不需要 root：

```bash
python3 scripts/test-linux-packages.py --backend wayland
```

脚本校验六个产物的 SHA-256，提取 deb / AppImage，分别验证真实音频、4K 视频、捆绑 libmpv / EQ / 输出模式恢复、失联保存设备拒绝、正常 FUSE 启动和损坏 manifest 拒绝。`--skip-fuse` 仅用于无 FUSE 条件，报告会明确未验证正常挂载启动。

源码包解压后，可在其中的 yyplayer 目录执行 `python3 scripts/prepare-linux-source.py`，核验归档并生成 vendor / Cargo 配置，再执行上述 `cargo build --locked --offline --release`。不要在当前开发工作树运行这个仅用于新解压源码包的准备脚本。

包内保留 Rust crate 的原始许可 / notices、Ubuntu 发行包完整 copyright 和 AppImage runtime 原文。上游预构建 runtime 没有公布完整 Alpine 静态包版本 / 可重链接对象清单；本轮产物是经过本机功能验证的开发预览，正式二进制发行的剩余条件见 [runtime 记录](../packaging/linux/runtime-notices/README.md)。

```bash
sudo apt install ./dist/yyplayer_*_amd64.deb
chmod +x ./dist/YYPlayer-*-x86_64.AppImage
./dist/YYPlayer-*-x86_64.AppImage
# 不可用 FUSE 的环境：
./dist/YYPlayer-*-x86_64.AppImage --appimage-extract-and-run
```

包结构、提取启动和正常退出可以不以 root 安装验证。真正安装 / 升级 / 卸载及首次文件关联仍应在独立测试系统执行。发布 workflow 生成 CI artifact，不代替这些验收。`Ubuntu 26.04 packages` 当前自动触发条件仍为 `dev-linux` push；可通过 Run workflow 选择 `master` 手动运行。最低版本矩阵独立且仅手动触发，见 [矩阵指南](LINUX_COMPATIBILITY_CI.md)。
