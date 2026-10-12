# 0.4.0 前端优化验收

日期：2026-10-12。Windows 11 x64 / Rust 1.99.0 / Slint 1.17.1 / Winit + FemtoVG / manifest 锁定的 libmpv。沿用 dev 工作区，保留此前 0.3.1 规则与打包文档修改；基准 c089cf7，不提交 / 推送或合入 master。

## 改动范围

1. 联网搜索宽列采用歌曲44% / 歌手25% / 专辑31%，封面与更多菜单保留；表头和行使用同一公式。窄窗口展开选项时保留两行摘要，不改变76px虚拟列表行高与自动分页。
2. 默认操作字号12→14px，音频 / 歌词 / 解码说明11→13px；底栏歌曲名12→16px、歌手10→12px。音乐库歌曲12→14px、歌手 / 专辑11→13px、时长10→12px，原有列拖动和居中播放控制保持。
3. 快捷键、联网服务、歌词设置均采用现有 GlassSurface，22px内边距；保留外观和字体卡片。全部位于可滚动表单，最低窗口可以继续访问下方设置。
4. LyricLine 用该行实际字号 / 字重 / 可用宽度测量主、副语言；普通行语言间距为设置值70%，当前行为100%。字号改变立即重新排版，颜色和不透明度仍平滑变化；不再把普通行放在大字号预留框中，也不靠两个重启中的字号 / 滚动动画追逐居中位置。
5. 歌手 / 专辑合为16px一行；下方显示 `类型 | 码率 | 采样率 | 源位深 | 输出状态 / API`。本地后台标签读取原有有界通道 / revision，Engine在线程内获取实际 file-format / audio-codec-name / audio-bitrate / current-ao。串流包级码率标≈，未知项明确显示；源位深不能用解码PCM或物理DAC格式反推，完整诊断仍分开显示源 / API / DAC。
6. 封面480ms移动 / 缩放 / 回弹保留。歌词页内容使用同一进度和封面位移，几何宽高固定以防转场期间重新换行；退场也跟随封面返回底栏。固定内容区裁剪避免歌词透过半透明播放栏。浏览时保留隐藏歌词布局，隐藏 / 首次布局 / 封面旅行时直接定位当前行；实际播放切行保留360ms滚动，快速反向和减少动态效果兼容。

## 检查与实机证据

- `lyric-layout` 实际 Slint 测量：同句普通行主语言高26px / 间距7px；选中换行为74px / 间距10px；窄列普通行换行为51px / 间距仍7px。短句具有相同语言间距，不受另一句换行高度影响。证据 `target/lyric-layout/results.json` 与三张图。
- Test-Lyrics Debug 53阶段通过：真实controller / libmpv、自有WAV / 三色封面 / 双语素材、样式点击 / 保存、首尾 / 长句居中、最小窗口选项、纯文本 / 无封面、反向 / 减少动画、后半段39.3秒第13行重新进入即居中、WAV / 1411kbps / 44.1kHz / 16bit / WASAPI摘要、设置卡片截图。
- Test-Online FrontendOnly / SkipHttps / SkipNavigation通过：自有HTTPS分页20→40→45，追加保留visible_first=13；短页填充、重复 / 503末页停止、设置返回保留搜索与20条结果、API目录实际按钮、宽 / 窄 / 选项布局。增加QA重绘以防静态GPU背缓冲截图过期；不是重新验证全部媒体 / HTTPS契约。
- navigation-ui、ui-feedback、audio-ui、navigation-history通过：真实Slint点击 / 计时 / 几何 / 返回；预览控制器的布局证据不等于媒体资格。
- Test-UiCustomization 22阶段通过：79首保留记录、可见缩略图、列拖动保存比例0.56、字体枚举278项、封面返回 / 静默历史保存、SRT / ASS字体、原生最大化 / 还原 / 关闭和最小化。本轮排版资格不等于DAC / HDR资格。
- Windows PowerShell5.1的7项隔离打包回归通过，fixture版本同步0.4.0；本轮未生成新的完整ZIP / Setup，此前0.3.1包保留。
- 格式、严格workspace/all-targets clippy、完整workspace/all-targets tests通过（48项独立单测：core14 / mpv9 / platform1 / UI1 / app23，既有HTTPS默认ignored；例子复用的app测试不重复算独立测试）。
- 最终普通Release应用 + lyrics-ui构建通过（4分18秒），最终QA暂停等待修正后单独Release例子构建通过（1分38秒）。裁剪后的Debug / Release均53阶段通过，13组GPU控制像素断言；两组尺寸1240×900 / 1000×640各三张设置卡片图。最终严格clippy通过（2.07秒），不把之前失败的取色采样写为通过。
- 进入 / 返回 / 后半段进入 / 返回的Cover进度0.05–0.95有效采样分别8 / 7 / 9 / 8帧；这些样本的当前行中心偏移均为0px。四张转场中间图重新生成，最终退场图确认歌词被内容区裁剪，未透过底栏；不是帧率测量。
- 普通EXE `target/release/yyplayer.exe`：30,297,600字节，FileVersion / ProductVersion均0.4.0，SHA-256 `4defd6091ed6fe0e7e43e9f0ccaafd43128624b51dadf4f4f094f60a1e50c7e4`。独立配置4秒正常退出exit0、ready=true、Idle、error / render_error空、视频renderer初始化0，实际runtime `mpv v0.41.0-1087-ge470f8986`；未使用开发路径覆盖。证据 `target/frontend-release-info.json` 与 `target/frontend-smoke/diagnostics.json`。

可复验命令：

```powershell
cargo fmt --all -- --check
cargo clippy --locked --offline --workspace --all-targets --config profile.dev.debug=0 --config profile.dev.incremental=false -- -D warnings
cargo test --locked --offline --workspace --all-targets --config profile.dev.debug=0 --config profile.dev.incremental=false
cargo run --locked --offline -p yyplayer-app --example lyric-layout --config profile.dev.debug=0 --config profile.dev.incremental=false
pwsh -NoProfile -File scripts/Test-Lyrics.ps1 -Profile release
pwsh -NoProfile -File scripts/Test-UiCustomization.ps1
pwsh -NoProfile -File scripts/Test-Online.ps1 -SkipHttps -SkipNavigation -FrontendOnly
```

## 修正与限制

初轮Debug首次呈现未采到完整480ms封面旅行，动画证据使用Release；功能项继续Debug验收。实际字号逐帧动画与滚动同时重启曾使当前行漂移，已改为实际字号立即排版并复验居中。新增卡片使API目录按钮移动，回归更新为新按钮中心，未绕过真实点击。静态GPU取色前先暂停新素材，避免上一帧快照与运动进度圆点的位置差；实际播放 / 暂停点击仍单独验证。独立启动通过Env提供器移除进程级内核覆盖变量，避免空字符串被识别为无效开发路径。

截图来自自有测试素材，未复制用户歌曲或封面；全部使用独立配置，未改用户APPDATA或系统偏好。未新增Linux / macOS、多屏DPI / 物理输入 / 屏幕阅读器、USB / 数字捕获 / 位准确 / HDR、安装 / 卸载或正式发行闭包资格。极长双语块仍可能高于视口，可手动滚动；长技术摘要在窄列换行。没有改Engine命令边界、音频输出默认策略、GPU presenter或prepare_gl。

mpv属性语义核对自[官方0.41手册](https://mpv.io/manual/stable/#properties)：audio-bitrate为包级bits/s、current-ao为实际音频输出驱动；audio-device仍是请求设备，不能当作实际API。

下一步：在Linux用同一素材复验排版 / 转场 / 源格式摘要，再补物理键鼠和多DPI；按明确授权在dev提交并验证后由master合入。
