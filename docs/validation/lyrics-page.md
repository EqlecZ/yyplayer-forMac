# 歌词页优化验收（2026-10-11）

## 范围与 Git

用户要求先把当前分支合入 / 变基到 master，再签出 dev 完成封面主题、平滑转场与可调双语歌词。

接手工作区干净，当前 dev-api=`e99d67c`，master=`5390a3d`，dev=`ec55789`；两个目标分支均为当前分支祖先。已在 master 执行 `git merge --ff-only dev-api`，再签出 dev 并 `git merge --ff-only master`。本地 master / dev 均基于 e99d67c，本轮功能改动保留 dev 工作区；没有改写历史、提交本轮改动或推送。

实现涉及 core lyrics / settings；app music_assets / online 的后台取色、lyric_controller 和音频投影；UI view_model / shell、歌词行 / 样式组件、歌词页 / 设置 / 播放栏 / 背景选择和转场；bootstrap 的 Esc 关闭样式面板。没有依赖、Engine、presenter、固定 runtime 或音频默认策略改动。选择依据见 [ADR 0009](../adr/0009-lyrics-theme-and-layout.md)，使用方法见[歌词指南](../LYRICS.md)。

## 环境与方法

Windows x64，Rust / Cargo **1.99.0**，锁定 Slint / slint-build 1.17.1；实际 UI 使用 Winit / FemtoVG。固定 libmpv `e470f8986e`，DLL SHA-256：`0d5b9dbecb73e179ef39dc94314ff8905fab926a80f8d60a2367d824b7254e2e`。原生截图为 2170×1575（1240×900 的 175% 缩放）与 1750×1120（1000×640），软件例子截图单独标记。

`scripts/prepare-lyrics-fixtures.py` 生成自有 65 秒 44.1 kHz stereo 静音 WAV、三套封面、20 行中 / 英 / 日 LRC（包含长句与单语）和三行无时间轴歌词。`scripts/Test-Lyrics.ps1` 在 target/lyrics-ui 设置独立配置，使用生产 controller、资源 worker、实际 libmpv、真实 Slint 指针事件和 GPU 截图；没有读取用户歌曲或修改用户 APPDATA / 系统偏好。素材 hash、动作、颜色、排版字段、滚动几何与逐帧转场记录保存于该目录。

Slint 默认布局定位、字体测量和 GPU snapshot 可能延迟同次 callback。最终验收观察实际透明度 / 转场保留状态，设 10 秒明确超时；不把等待时长或 controller.page 单独当动画完成。样式 SpinBox 和歌词 / 封面按钮使用实际点击；其余参数批量通过相同生产 controller 命令提交，分别注明，不能称为所有控件人工测试。

## 检查

| 命令 / 场景 | 本轮结果 |
| --- | --- |
| `cargo fmt --all --check` | 通过 |
| `cargo clippy --locked --offline --workspace --all-targets … -- -D warnings` | 最终通过，32.96 秒 |
| `cargo test --locked --offline --workspace …` | 最终通过，core14 / mpv9 / platform1 / UI1 / app22，1 项 HTTPS 默认 ignored；文档测试通过 |
| Debug bins / examples | 构建通过 |
| `scripts/Test-Lyrics.ps1 -SkipBuild` | 最终通过，25 阶段，真实连续点击移动封面、样式控件、保存、排版 / 滚动与减少动态效果 |
| navigation-history / navigation-ui / ui-feedback / audio-ui | 全部通过：生产历史 24 步、导航 11 点、浮窗 / 控件 17 点、音频 UI 五图 / 实际点击 |
| `Test-UiCustomization.ps1 -SkipBuild` | 最终通过，22 阶段，真实字体 / SRT / ASS、列宽、原生窗口、封面往返 |
| `Test-Online.ps1`（无跳过参数） | 完整通过，HTTPS 40.82 秒；真实串流 / 下载 / 歌词 / 封面、自动分页 / 目录按钮与导航 |
| `Test-VideoLibrary.ps1 -SkipBuild` | 通过，视频 10 次释放 / 重建、续播、快速取消、EOF 前后暂停 / 继续 |
| 普通 Release / 独立配置启动退出 | 最终构建通过；4 秒正常退出 exit0，ready=true、error / render_error 空、视频 renderer 初始化 0 |
| PowerShell / Python 语法、diff / 文档链接 | 全部通过 |

编译与例子使用 `--locked --offline`；Debug 使用 `--config profile.dev.debug=0 --config profile.dev.incremental=false`，Test-Online 以等价的临时 Cargo profile 环境变量复用构建。受限进程不能读取 Windows 目录元数据，首轮资源加载超时、Clippy 的 CARGO_MANIFEST_DIR 和 rustfmt 出现拒绝访问；授权的本地验收以普通权限重跑。没有关闭用户播放器或修改系统配置。并行 Cargo test 尝试重写正在验收的 EXE 后失败，最终编译和 GUI 执行分开，完整 workspace 测试在全部例子退出后通过。

## 实机结果与截图

- 背景真实色：珊瑚封面 `#c6445a` → 蓝色封面 `#2868c0` → 纯文本绿色封面 `#247c70`；切歌时立即撤销旧封面颜色，无封面回退。截图：[珊瑚 / 深色](lyrics-page/coral-bilingual.png)、[蓝色 / 浅色](lyrics-page/blue-light.png)、[纯文本双语](lyrics-page/plain-bilingual.png)。
- 默认双语长句、日语 / 中文、单语都按实际高度排版，当前行居中；自有 LRC 行数 20，原始 0ms 点击回调保留。1000×640、播放选项打开时仍可滚动；64px 当前字号 / 80px 间距下首尾几何断言通过。此时长块可以超过视口，见边界说明。[最小窗口选项](lyrics-page/minimum-options.png)。
- 实际样式按钮 / 增加间距按钮收到回调，24 → 25；普通行 24px / 35% / 300、当前行 36px / 800、副语言 500 / 16px，保存后读回完整对象一致。[页内设置](lyrics-page/style-panel.png)、[设置页预览入口](lyrics-page/settings-lyrics.png)。
- 480ms 封面轨迹经过中间位置，采样最大进度约 1.00544，包含轻微回弹；返回后恢复真实 48px 左下封面。移动中真实点击反向 / 再反向，最终位置和透明度正常。减少动态效果时无保留计时，进度直接为 0 / 1；视频页的过渡封面禁止点击。[恢复浏览](lyrics-page/returned-browse.png)。
- 自有 HTTPS 实际串流位置 1.020139 秒、联网封面后台色 `#486b91`，下载后本地位置 0.704191 秒；暂停 / seek / 手动候选和离线播放保留。见[完整联网记录](lyrics-page/online-results.json)。

歌词 / 实际点击与轨迹见[25 阶段数据](lyrics-page/results.json)，输入素材见[hash 清单](lyrics-page/fixtures.json)。回归证据：[字体 / 窗口](lyrics-page/customization-results.json)、[页面历史](lyrics-page/navigation-history-results.json)、[导航](lyrics-page/navigation-results.json)、[浮窗 / 控件](lyrics-page/feedback-results.json)、[视频生命周期](lyrics-page/video-results.json)。截图来自自有素材，不证明位准确或 HDR 输出。

最终普通 Release：`target/release/yyplayer.exe`，28,188,160 字节，SHA-256 `35779712d2665dc2b39d050a8db046981e35ef2ed812c2a66f017ee8cb5dc2d8`。最终增量构建 0.95 秒 exit0；独立配置启动后 4 秒经正常资源清理退出 exit0，运行时 mpv v0.41.0-1087-ge470f8986，Idle / ready=true / error 与 render_error 为空。见[启动与退出诊断](lyrics-page/release-smoke.json)。此空闲启动没有媒体，不替代上面的真实资源 / 播放验收。

## 修正与边界

开发轮次先发现渐隐层默认居中并覆盖译文，改为显式 x/y 贴边；将柔光渐变限于透明外缘，避免明显矩形色块。覆盖歌词页后，原主布局空内容会把播放选项挤到零高度，已增加显式可伸展空间，并以最小窗口截图复验。

精确 seek 到时间戳边界可能落在前一帧 / 样本；本例在 9.3 / 57.3 秒确认长句行（index=3）/ 最后行（index=19），独立的真实歌词点击仍断言原始 0ms 时间戳。第一次 SpinBox 点击命中减少按钮，按真实横排增加 / 减少位置修正坐标，未把 23 当作预期的 25。

快速反向切页从最后操作重新启动保留计时；零帧后测透明度 / Timer 状态，不能以单次 snapshot 后固定 850ms 推断计时已经结束。同帧修改减少动态效果时，转场直接读取当前窗口输入，取消在途动画和计时；不依赖稍后才投影的 Theme 副本。

原 Test-UiCustomization 发现收回尚未结束时的保留封面会拦住左下点击，现保留封面在浏览状态也可立即展开；新增连续点击的是移动中的实际封面，最终字体 / 封面回归通过。重复运行的无封面场景曾把上一轮同名截图误当自动封面；fixture 已隔离至 media/，截图留在测试根目录，复验通过。

Windows 检查不扩展为 Linux / macOS、真实鼠标 / 触控板 / 键盘、多屏 DPI 或字体安装变化资格。字体不具备某个字重时由渲染器选择 / 合成；没有逐字卡拉 OK、语言自动识别或纯文本时间同步。极长歌词块 / 用户选择的大字号可超过视口，需要手动滚动。低不透明度由用户决定，不承诺任意封面 / 背景 / 自定义字体的对比度。

下一步：在 Linux 使用同一自有素材复验双语换行、背景和转场；补物理滚动 / 输入、多 DPI 与实际字体组合。USB / 位准确 / HDR 和二进制发行资格沿用既有边界。
