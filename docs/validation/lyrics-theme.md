# 完整歌词主题与 Android 背景验收（2026-10-12）

## 范围与基线

用户指出上一轮背景色偏弱，播放栏仍用应用色；本轮在 dev 沿用未提交的歌词实现，按 Android 项目只读参考改进。没有提交 / 推送或改写 master，没有修改 Android 源码。实现依据见 [ADR 0010](../adr/0010-full-lyrics-theme.md)，操作见[指南](../LYRICS.md)；上一轮排版证据保留在[原报告](lyrics-page.md)。

涉及 Theme / AppWindow / NowPlaying、MusicAmbient 与背景预览、主题控件和播放栏 / 歌词样式 / 音频播放面板、shell 的按钮前景投影、lyrics-ui 的真实颜色 / GPU 像素及点击验收。没有 Engine / presenter / 音频策略、依赖或 runtime 改动。

## 方法

Windows 11 专业版 x64（10.0.26300）、AMD Ryzen AI 7 H 350、AMD Radeon 860M（32.0.31041.1004），另有 MuMu 虚拟显示适配器；不是性能 / 功耗测量。锁定 Slint 1.17.1，实际 Winit / FemtoVG 与固定 libmpv（20260928-git-e470f8986e）；自有珊瑚 / 蓝 / 绿封面、静音 WAV 与中 / 英 / 日歌词。`Test-Lyrics.ps1` 使用 target/lyrics-ui 的独立配置与 media/，不读取用户歌曲或写用户 APPDATA。浏览主题固定为 #6875E8，与封面明显不同，测试观察主线程投影后的实际颜色 / 页面混合，并从真实 GPU snapshot 取播放 / 暂停填充、进度拇指、音量已播放轨道和已选取色勾选框像素。

记录实际淡入 / 淡出和换歌中间颜色，移动中点击反向，检查最终恢复的应用色。两尺寸 / 明暗、无封面 / 关闭取色、双语参数 / 保存 / 换行 / seek、减少动画沿用实际 UI 例子。真实点击播放 / 暂停与倍速下拉，核对实际播放状态 / 倍速，不能只看 controller.page。追加最小窗口滚动到底部 / 实际重置 / 滚回顶部继续编辑，以及 1.5x → 1x → 1.37x → 1x 的内核值与控件标签一致性。

OpenGL take_snapshot 读取后缓冲，验收主动请求重绘、限时等待像素一致；转场截图后台编码，避免 PNG 编码阻塞 UI。Debug 的返回浏览初次布局超过转场采样窗口，因此 Release 增加返回时至少两个中间颜色帧断言；Debug 的端点验证不冒充完整返回动画证据。

## 检查结果

| 检查 | 结果 |
| --- | --- |
| fmt / 严格 clippy / workspace tests | 最终 fmt exit0；严格 clippy all-targets exit0（5.40 秒）；workspace exit0（13.15 秒），core14 / mpv9 / platform1 / UI1 / app22，HTTPS 默认 ignored |
| `Test-Lyrics.ps1 -SkipBuild` / `-SkipBuild -Profile release` | 两者 exit0，Release 43 阶段与 13 组 GPU 像素断言；进入 / 返回各至少两个中间颜色帧通过 |
| `Test-NavigationHistory -SkipBuild` / navigation-ui / ui-feedback / audio-ui / `Test-UiCustomization -SkipBuild` | 全部 exit0；历史24步、导航11点、浮窗17点、音乐五图 / 封面 / 点击、字体与窗口22阶段 |
| `Test-VideoLibrary.ps1 -SkipBuild` | exit0，真实视频10次释放 / 重建、续播、快速取消、EOF 前后暂停 / 继续 |
| `Test-Online.ps1 -SkipHttps -SkipNavigation -FrontendOnly` | exit0，真实前端分页 / 错误 / 目录按钮 / 三尺寸；本轮跳过后台 HTTPS 传输与媒体专项，沿用上一轮完整记录；导航已单独复验 |
| 普通 Release 与独立配置正常退出 | 最终 bin 构建 exit0（7分25秒）；新独立配置4秒正常退出exit0，ready=true，error / render_error空，libmpv renderer初始化0，script_steps=0 |
| 脚本语法 / diff / 文档链接 | PowerShell / Python语法、证据JSON / 自有素材hash、git diff --check、变更Markdown相对链接全部通过 |

代码检查命令（锁定依赖 / 离线）：

```powershell
cargo fmt --all -- --check
cargo clippy --locked --offline --workspace --all-targets --config profile.dev.debug=0 --config profile.dev.incremental=false -- -D warnings
cargo test --locked --offline --workspace --config profile.dev.debug=0 --config profile.dev.incremental=false
cargo build --locked --release --offline -p yyplayer-app --bin yyplayer
```

Release 动画验收先以 `cargo build --locked --release --offline -p yyplayer-app --bin yyplayer --example lyrics-ui` 构建（3分08秒），最终普通应用再按上述 bin 命令构建。表格中的 GUI 命令串行执行。普通应用不启用 online-qa；Test-Online 临时启用该 feature，并通过临时 Cargo profile 环境变量复用 Debug 构建。受限进程不能读取本机工具目录元数据，编译 / Python 与真实窗口验收用授权的普通本机权限运行，没有改变全局环境或停止用户播放器。

## 实际颜色与转场

Release 数据见[43 阶段记录](lyrics-theme/results.json)与[自有素材 hash](lyrics-theme/fixtures.json)。默认浏览色 #6875E8；珊瑚 #C6445A、蓝 #2868C0、绿 #247C70 的播放按钮、进度拇指和音量轨道跟随实际封面色，样式面板的选中勾选也同色；RGB 允许 16 级抗锯齿 / 过渡误差。背景左侧采样由浏览 #171A24 变为深色珊瑚 #672A37，浅色蓝 #A4BAD5、浅色绿 #A2C0BA、深色绿 #184642，整窗柔光不再只是弱装饰层。

Release 进入 / 返回分别记录 19 / 18 种颜色，返回首次采样 49ms；换歌记录 42 种颜色。封面进度峰值 1.00533，快速反向操作与最终 48px / 大封面位置通过。上述是此场景的状态 / 图像证据，不是帧率指标。减少动态效果时同帧到端点且停掉保留计时；无封面与关闭取色使用应用色，返回浏览恢复 #6875E8。

截图：[珊瑚深色](lyrics-theme/coral-bilingual.png)、[蓝色浅色](lyrics-theme/blue-light.png)、[进入中间帧](lyrics-theme/theme-enter-middle.png)、[返回中间帧](lyrics-theme/theme-return-middle.png)、[恢复浏览](lyrics-theme/returned-browse.png)、[播放选项](lyrics-theme/playback-panel-themed.png)、[倍速弹层](lyrics-theme/speed-popup-themed.png)、[非预设倍速](lyrics-theme/speed-nonpreset-themed.png)、[最小窗口旧背景](lyrics-theme/legacy-minimum-themed.png)、[滚动到底部](lyrics-theme/styles-minimum-bottom.png)、[重置后顶部](lyrics-theme/styles-minimum-reset.png)。原图来自实际 GPU，不重绘 / 修饰截图。

回归记录：[字体 / 原生窗口 / 列宽](lyrics-theme/customization-results.json)、[页面返回历史](lyrics-theme/navigation-history-results.json)、[导航](lyrics-theme/navigation-results.json)、[浮窗 / 控件](lyrics-theme/feedback-results.json)、[音乐界面](lyrics-theme/audio-ui.txt)、[视频生命周期](lyrics-theme/video-results.json)、[联网前端](lyrics-theme/online-ui-results.json)。导航、浮窗与 audio-ui 使用明确预览状态，只资格化布局 / 点击逻辑；真实歌词 / 播放状态来自上面的 Release live controller 和 libmpv。

## 修正

完整主题由窗口动画值绑定到 Theme，播放栏、歌词样式和播放选项 / EQ 控件读取同一令牌。真实倍速测试发现内部赋值会使显示停留在 1.5x，现改为控制器值输入 / 用户选择回调；真实 1x 恢复与非预设值均通过。数值与勾选同样保留状态输入绑定，编辑中的文本草稿不被重复投影覆盖。

样式表单使用实际首选高度、左上起点与显式滚动高度，避免长内容自动居中裁掉顶部；勾选 / 增减采用 SVG，避免缺字。去掉歌词两端的平色遮罩，浅色柔光背景不再出现横条。参数保存验收等待实际后台写回，在十秒限时内读取完整对象，不以固定延迟假设原子保存已完成。

## 最终普通 Release

`target/release/yyplayer.exe`：28,974,080字节，SHA-256 `8adee6c261a1f8155d3ca59b7c4abd78ba9d683356c139f1100a795168f3037f`。最终构建不启用online-qa。见[文件与检查摘要](lyrics-theme/release-binary.json)和[启动 / 正常退出诊断](lyrics-theme/release-smoke.json)：固定runtime为mpv v0.41.0-1087-ge470f8986，Idle / ready=true、错误为空，libmpv视频renderer初始化0，70%音量 / 1x倍速，没有测试动作（script_steps=0）。空闲启动不替代前面的真实播放验收。

首轮启动检查把覆盖环境变量置为空值，仍被Rust识别为存在，误触发自定义runtime路径 / smoke动作；改为用环境变量Provider真正移除，仅影响当前验收进程，用新配置重新通过并恢复原环境。没有用YYPLAYER_MPV_LIBRARY绕过最终hash验证，没有停止用户播放器。

## 未验证条件

Windows 原生事件注入不代替物理键鼠 / 滚轮、多屏 DPI、屏幕阅读器或 Linux / macOS。静态渐变和截图不证明帧率 / 功耗、DAC 位准确或 HDR。默认布局 / 封面色可读性在自有素材中核对，不承诺任意自定义背景与字体组合。下一步在 Linux 使用相同素材复验，并补物理输入与可访问性。
