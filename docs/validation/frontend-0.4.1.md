# 0.4.1 歌词收回动画验收

日期：2026-10-12。Windows 11 x64 / Slint 1.17.1 / Winit + FemtoVG / manifest 锁定的 libmpv。按用户要求先将已有0.4.0成果本地提交到dev的 `676a699`，再修改本轮代码；0.4.1保留在dev工作区，未提交 / 推送或合入master。

## 改动与边界

用户截图显示封面即将返回底栏时，歌词与右侧按钮仍覆盖搜索结果。页头按钮、歌曲信息、歌词视口现在分别把自身中心向左下小封面的中心收拢；左侧「正在聆听」也沿同一进度返回。内容尺寸保持，转场不会重新换行或从首行滚动。

封面展开进度为p：文字透明度取 `t = clamp((p - 0.5) * 2, 0, 1)`，再取 `t*t*(3-2*t)`。返回时p≤0.5即完全透明；进入时后半程逐渐显现。往返共用同一曲线，快速反向不会切换透明度规则；几何进度限制在0–1，原480ms封面回弹保留。转场按钮禁用点击但透明度统一跟随页面，不额外应用禁用按钮的瞬间变灰。减少动态效果立即到达目标位置 / 透明度。

改动位于 `app.slint`、`pages/now-playing.slint`、`examples/lyrics-ui.rs`、workspace版本 / 锁文件、两套打包fixture及相关文档。五个workspace crate均0.4.1；未增加依赖或改变Engine线程边界、GPU presenter / prepare_gl、音频默认策略。

## 检查与证据

- `cargo fmt --all -- --check`与 `cargo clippy --locked --offline --workspace --all-targets -- -D warnings`通过，最终严格clippy用时14.20秒。
- `cargo test --locked --offline --workspace --all-targets`通过（2分08秒）：48项独立单测，core14 / mpv9 / platform1 / UI1 / app23；既有HTTPS默认ignored，例子复用controller测试不重复计数。
- 最终Release应用与歌词例子构建通过（4分46秒）；调整验收时间容差后的Release例子单独重建通过（1分25秒）。
- `pwsh -NoProfile -File scripts/Test-Lyrics.ps1 -SkipBuild -Profile release`最终通过59阶段、13组GPU控件取色、544个轨迹点。覆盖真实controller、当前行预定位、晚段39.3秒第13行、快速反向、减少动态效果、大小窗口、主题 / 样式 / seek和无封面 / 纯文本。
- 新返回验收同时检查真实透明度、各区域中心向共同锚点移动、轨迹共线和终点。以下计数来自最终 `target/lyrics-ui/results.json`；不是帧率测量。

| 返回目标 / 逻辑窗口 | 中间进度样本 | 半程后仍旅行的样本 | 这些样本最大文字透明度 | 三个区域共同终点 |
| --- | ---: | ---: | ---: | --- |
| 音乐库 / 1000×640 | 8 | 20 | 0 | (44,600) |
| 联网搜索 / 1840×1040 | 8 | 20 | 0 | (44,1000) |
| 联网搜索 / 1000×640 | 8 | 21 | 0 | (44,600) |

- GPU截图 `target/lyrics-ui/search-return-half-wide.png`与 `search-return-half-minimum.png`经视觉检查：封面仍在返回途中，歌词、歌曲信息和右上按钮已消失。实际图片为3220×1820 / 1750×1120（175%缩放）；搜索页使用网络关闭的自有验收状态，不新增第三方搜索资格。
- 最终重新构建Debug回归例子后串行运行 `scripts/Test-UiCustomization.ps1 -SkipBuild`（22阶段）、`target/debug/examples/navigation-ui.exe`和 `ui-feedback.exe`（17阶段），全部通过，涵盖实际Slint点击、页面几何 / 透明度、封面返回、列拖动、字体、播放控件中心、通知和自动隐藏。
- `scripts/Test-WindowsPackaging.ps1`（7项）与 `scripts/Test-WindowsPackagingLauncher.ps1`（10项）通过，fixture均0.4.1。证据分别在 `target/windows-packaging-qa-2d5f70ed010344648ea6a2691518f4fd/results.json`和 `target/windows-launcher-qa-75534cc858684dfa9a69811c0a1016f9/results.json`。

初轮快速反向验收把真实事件处理当成瞬时操作，1%透明度差断言失败。最终按实际点击耗时与曲线最大变化速度给出容差，保留位置 / 透明度连续性断言，重新构建后59阶段全部通过。初轮失败记录移至 `target/frontend-0.4.1-initial-reversal-failure.json`及对应frame图片，不作为最终结果。

## 本地程序与限制

最终普通应用为 `target/release/yyplayer.exe`，30,330,880字节，FileVersion / ProductVersion均0.4.1，SHA-256：

```text
2b86439ef8fe90a14affbc97923bfb889929b2e87ab8872a85027c2b8b2e7623
```

独立配置启动4秒后正常退出exit0，ready=true、Idle、error / render_error空、视频renderer初始化0；实际runtime为 `mpv v0.41.0-1087-ge470f8986`。证据 `target/frontend-0.4.1-smoke/results.json`与 `diagnostics.json`。本轮没有生成完整0.4.1 ZIP / Setup；根目录Package-Windows.cmd继续自动编译当前版本并生成独立输出目录，旧包保留。

验收使用自有素材、独立配置，未读用户歌曲、改用户APPDATA或系统偏好。未新增Linux / macOS、物理键鼠 / 多DPI / 多屏、屏幕阅读器、USB / 数字捕获 / 位准确 / HDR、安装 / 卸载或正式发行闭包资格。下一步在Linux复验同一素材的转场，再补物理快速反向和多DPI；按后续明确授权提交0.4.1，再由master合入已验证dev改动。
