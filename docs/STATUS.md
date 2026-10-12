# YYPlayer 进度与证据

## 2026-10-12：歌词页完整主题与 Android 柔光背景

状态：Done（Windows 实现、Release 真实转场 / GPU 控件取色与回归）；Linux / macOS 和物理输入待验。沿用 dev 工作区，未提交 / 推送；本地 master / dev HEAD 均为 e99d67c，新增未提交功能不在 master。详见[主题验收](validation/lyrics-theme.md)、[ADR 0010](adr/0010-full-lyrics-theme.md)与[指南](LYRICS.md)。

- 目标与结果：封面主题色覆盖暂停 / 播放、进度 / 音量、图标、倍速下拉、歌词样式、播放选项 / EQ。只读参考 Android MusicAmbientBackground，整窗采用带色底色、两侧径向柔光与底部渐变，旧矢量背景和自定义图叠在同一基底；浅色歌词区去掉平色横条。浏览往返以420ms混合主题，换歌950ms更新，减少动态效果沿用即时切换。
- 改动文件：UI theme / app、now-playing、music-ambient / theme-widgets 与三枚勾选 / 箭头 SVG、controls / player-bar / audio-options / video-options / background-choice；shell 的封面按钮前景投影；lyrics-ui 43阶段例子与 Test-Lyrics 的 Release 入口；README / LYRICS / ADR / CHANGELOG / 本条及报告和自有证据。没有新增依赖、Engine / presenter、runtime 或音频默认策略改动。
- 交互修正：主题控件把控制器值作为输入，以回调提交操作，避免选择后破坏输入绑定；实际1.5x、恢复1x、非预设1.37x均与标签一致。歌词样式表单左上定位、显式首选高度与滚动高度，1000×640可滚到底部重置，再滚回顶部继续编辑；输入草稿保留，勾选 / 增减不依赖符号字体。
- 检查：最终 `cargo fmt --all -- --check` exit0；严格 `cargo clippy --locked --offline --workspace --all-targets --config profile.dev.debug=0 --config profile.dev.incremental=false -- -D warnings` exit0（5.40秒）；相同profile的完整 `cargo test --locked --offline --workspace` exit0（13.15秒），core14 / mpv9 / platform1 / UI1 / app22，HTTPS默认ignored。Test-Lyrics Debug与Release均exit0；Test-UiCustomization22阶段、Test-NavigationHistory24步、navigation-ui11点、ui-feedback17点、audio-ui五图与点击、Test-VideoLibrary真实视频10循环均exit0。Test-Online仅本轮FrontendOnly / SkipHttps / SkipNavigation exit0，后台完整HTTPS / 媒体资格沿用上一轮，导航另验。脚本语法、JSON / 素材hash、diff / 文档链接通过。
- 真机：Windows 11 x64、Winit / FemtoVG / 锁定Slint1.17.1与固定hash libmpv，自有素材 / 独立配置。Release 43阶段、13组控件GPU像素断言，珊瑚 / 蓝 / 绿主题明确覆盖整窗；进入 / 返回各19 / 18种中间色、换歌42种，封面进度峰值1.00533。两尺寸 / 明暗 / 无封面 / 关闭取色 / 双语与参数保存 / 首尾长句居中 / 反向点击 / 减少动画通过。不是帧率、位准确或HDR测量。
- Release：最终 `cargo build --locked --release --offline -p yyplayer-app --bin yyplayer` exit0（7分25秒），普通EXE不启用online-qa。新独立配置 / 清除本进程覆盖变量后4秒正常退出exit0；ready=true，error / render_error为空，libmpv视频renderer初始化0、script_steps=0、音量70% / 倍速1x；文件28,974,080字节，hash见报告。没有写用户APPDATA / 系统偏好 / Android工程。
- 未验与下一步：Debug返回浏览初次布局可能超过动画采样窗口，完整动画证据使用Release；本轮自有Release场景未发现新增缺陷。物理键鼠 / 滚轮、屏幕阅读器、多屏DPI、Linux / macOS、新安装包尚未新增证据。下一步在Linux用同一素材复验完整主题与转场，再补物理输入 / 可访问性；既有USB / 位准确 / HDR / 发行限制保留。

## 2026-10-11：歌词页主题、转场与双语排版

状态：Done（Windows 实现与自有素材 / 真实联网资源验收）；Linux / macOS 新布局和物理输入待验。改动保留在 dev，未提交 / 推送。详见[使用指南](LYRICS.md)、[ADR 0009](adr/0009-lyrics-theme-and-layout.md)和[验收记录](validation/lyrics-page.md)。

- Git：初始工作区干净，dev-api=e99d67c 是 master=5390a3d 与旧 dev=ec55789 的后继；已按用户授权先快进合入本地 master，再签出 dev 并快进到同一 e99d67c。没有改写历史或推送；本轮未提交功能不在 master 中。
- 实现：内嵌 / 本地 / 联网真实封面在现有后台资源服务中限量采样，生成歌词页主题色与柔光渐变，缺失回退应用色。浏览 / 歌词页 420ms 淡入淡出，真实左下 48px 封面 480ms 移动 / 缩放并微回弹；快速反向点击和减少动态效果即时切换均覆盖。
- 排版与配置：页内「歌词样式」与「设置 → 歌词」共用八项独立参数、两个开关及默认恢复；revision 驱动实时投影和既有自动保存。带空格的 ` / ` 拆为上下双语，副语言字重 / 间距独立，点击保留原时间戳；按实际换行高度居中跟随，纯文本共用排版。字体族仍在独立字体设置中。
- 改动文件：core lyrics/settings；app music_assets、online/mod、online_controller、audio_controller、lyric_controller、controller、bootstrap；UI view_model/shell、app、now-playing/settings、lyric-line/lyric-options、music-art/player-bar、背景选择；lyrics-ui/online-search 例子、Test-Lyrics/prepare-lyrics-fixtures、指南 / ADR / CHANGELOG / 本条和报告。没有依赖、Engine、presenter、固定 runtime 或音频默认策略改动。
- 检查：fmt check exit0；严格 clippy workspace/all-targets exit0（32.96秒）；最终完整 workspace 单测 exit0（core14/mpv9/platform1/UI1/app22，HTTPS 默认 ignored 后完整 Test-Online 显式通过40.82秒）；Test-Lyrics 25阶段、Test-UiCustomization 22阶段、Test-NavigationHistory 24步、navigation-ui 11点、ui-feedback 17点、audio-ui 五图 / 实际点击、完整 Test-Online、Test-VideoLibrary 视频10循环均 exit0。普通 Release 最终构建 exit0，4秒独立配置启动 / 正常退出 exit0、ready=true、error/render_error空、视频 renderer 初始化0；EXE 与 SHA 见报告。PowerShell / Python 语法、diff / 文档链接检查通过。
- 实机：Windows x64 / 锁定 Slint 1.17.1 / Winit+FemtoVG / 固定 hash libmpv；自有珊瑚、蓝、绿封面及长句中 / 英 / 日歌词，1000×640 与1240×900、175%缩放截图审查；实际页内参数点击与保存读回一致。首尾、大字号 / 大间距、最小窗口播放选项、无封面 / 纯文本 / 同帧减少动画通过。自有 HTTPS 封面后台取色及串流 / 下载后本地播放通过；不读用户歌曲、不写用户 APPDATA / 系统偏好。
- 已知限制：没有逐字卡拉 OK / 自动语言识别 / 纯文本时间同步；字体可用字重由字面 / 渲染器决定，极长块或大字号可超出视口。Linux / macOS 新布局、物理键鼠 / 触控板、多 DPI 与字体安装变化待验；USB / 位准确 / HDR / 二进制发行资格不扩展。
- 下一具体步骤：Linux 运行同一自有素材复验双语换行、背景与转场，再补物理滚动 / 输入和多 DPI / 字体组合。


## 2026-10-11：A01.5 · 页面历史栈与返回

状态：Done（Windows 实现、验收、当前 dev-api 本地提交及缓存清理）；未推送。详见 [验收记录](validation/navigation-history.md)。

- 实现：统一七个页面的侧栏、设置、歌词 / 媒体与全屏跳转；进入不同页压栈，返回弹栈，重复当前页 / 根页不新增历史。侧栏和视频工具栏有返回入口，歌词大小封面返回来源页；默认 Alt+Left 一次返回一层，保留已有自定义绑定。历史仅本次会话，不保存每次访问的滚动位置。
- 涉及：AppController / navigation、audio_controller / online_controller、UI 投影、侧栏 / 设置 / 歌词 / 视频工具栏；navigation-history 实际 UI 例子与 Test-NavigationHistory、在线 / 视频 / 音频 UI 回归例子；Clean-Workspace 保留可执行文件旁 API config 目录、Test-Cleanup 保护验收；指南 / CHANGELOG / 本条及报告。
- 检查：最终 fmt check exit0；严格 clippy workspace/all-targets exit0（2.60秒）；最终 workspace 单测 exit0（core12/mpv9/platform1/UI1/app20，HTTPS默认ignored后完整Test-Online显式通过38.84秒）；Test-NavigationHistory 24步实际点击、三尺寸 / 透明度 / 宽度 exit0；完整 Test-Online 与导航 exit0，有结果的设置返回补充 FrontendOnly 定向复跑 exit0；Test-VideoLibrary 真实视频10循环含5次设置返回、EOF暂停回归 exit0；Test-UiCustomization -SkipBuild（例子由最终workspace测试构建）、ui-feedback、audio-ui exit0；PowerShell / Python 语法、diff与文档链接通过。Test-Cleanup 的实际隔离删除与 WhatIf / 配置hash保护 exit0，真实工作区 WhatIf 已通过。
- 实机：独立配置 / 自有素材；联网搜索 → 设置 → 返回恢复page6与20条结果，关键词 / API / 来源逐项一致；视频离开实际Idle / renderer释放，返回实际Playing / 帧数增长且位置恢复。普通 Release 无online-qa构建 exit0（3分51秒），4秒正常退出 exit0、ready=true/error空/render初始化0；exe SHA见报告。
- 提交 / 清理：已按用户授权将联网功能 / 前端 / 返回改动一并提交当前 dev-api；实际 Clean-Workspace exit0，移除53,407,759,825字节（49.74GiB）。target内仅剩Release EXE与空API config目录；EXE / 固定DLL / 四个mpv头文件hash未变，源码 / Git / 验收文档保留。清理后同一EXE独立配置再次启动 / 4秒正常退出 exit0，ready=true、error空、render初始化0；最终提交补录了清理与启动证据。
- 已知缺陷 / 未验：本轮自有场景未发现新增缺陷；Linux / macOS运行、物理输入 / 跨屏DPI未新增证据，原USB / 位准确 / HDR及发行限制保留。下一步在Linux复验相同返回路径和视频资源恢复。

## 2026-10-11：A01.4 · 联网搜索布局与自动分页

状态：Done（Windows 前端与自有 HTTPS / 原生目录交互）；Linux / macOS 新按钮及第三方分页待验。沿用 dev-api 原有未提交改动，未提交 / 推送。详见 [验收记录](validation/online-search-ui.md)。

- 目标与结果：搜索框移到页标题右侧；删除「加载下一页」，实际列表到底后自动追加并保留位置；短页自动补充，忙碌 / 历史 / 音质弹层不翻页，末页 / 重复页 / 失败停止。API JSON 设置新增「打开文件夹」，后台打开当前有效原始路径，包含默认目录，失败有可见提示。最低窗口展开播放选项时标题简写与筛选动作分行，不挤入面板。
- 改动文件：player-ui 的 app.slint / online-search.slint / online-options.slint；online_controller / online/mod 的请求保护、重复页停止与有界目录任务；player-platform 的 Cargo features / lib.rs / directory.rs（原生 ShellExecuteW 与平台边界）；online-search 实际 UI 例子、Serve-OnlineFixtures / Test-Online 脚本；ONLINE_API、CHANGELOG、本节及验收报告 / 自有截图。
- 检查：fmt check exit0；严格 clippy workspace/all-targets exit0；workspace 单测 exit0（core12/mpv9/platform1/UI1/app14，默认 ignored 的 HTTPS 在完整 Test-Online 中显式实跑38.70秒）；Test-Online 不传跳过参数 exit0，实际 UI / libmpv / 导航通过；ui-feedback exit0；普通 Release 无 online-qa 构建 exit0（4分08秒）。最终 diff / 验收脚本语法检查通过。
- 实机证据：Windows x64 / 固定 hash libmpv / 独立 target 配置 / 自有素材；20→40→45，追加 visible-first=12，末页 more=false；未滚动的短页补到40条；重复第二页及503失败停止且保留20条。API 自选 / 默认目录实际按钮收到原生成功回复。1000×640、1240×900、1460×920 截图审查通过。串流 position=1.020775秒、下载本地播放0.704930秒；原有歌词 / 暂停 / seek / 离线播放回归通过。
- 新 Release 独立配置启动 / 4秒正常退出 exit0，ready=true、error为空，运行时 mpv v0.41.0-1087-ge470f8986。没有写用户 APPDATA、媒体或系统设置。可执行文件与 SHA 见报告。
- 已知缺陷 / 限制：自有场景未发现新增缺陷；失败页需重新搜索。第三方 API、只读安装目录与特殊文件系统、Linux / macOS 本轮运行、物理滚轮 / 触控板与多 DPI 待验；既有 USB / HDR 资格不扩展。
- 下一步：在 Linux 复验同一 HTTPS / 前端 / 目录场景，用实际 API JSON 核对分页末页协议，并补权限失败与物理输入验收。

## 2026-10-10：A01 · Android 联网音乐/歌词移植到 PC

状态：Done（Windows 实现与自有 HTTPS / 实际 UI 联调）；第三方供给、Linux 新功能及新安装包运行待验。详见 [联网指南](ONLINE_API.md)、[ADR 0008](adr/0008-android-cloud-api-port.md)、[验收记录](validation/online-api.md)。

- Git：接手 Rust 工作区干净，dev 的 ec55789 已提交在 origin/dev；fetch 后 master 快进到最新 origin/master 5390a3d，再从 master 创建 dev-api。当前 master / origin/master 为0 ahead、0 behind。新增功能保留在 dev-api 工作区，未自动提交/推送；Android 只读参考，未修改。
- A01.1 搜索与界面：独立侧栏「音乐联网搜索」、API/来源/关键词/分页、持久历史与删除/清空、封面缩略图、音质播放/下载弹层；纯歌词 API 单独筛选，播放选项可为本地歌曲联网选择歌词。
- A01.2 数据与播放：Android JSON v1 模板/嵌套 POST/变量/trackFields/headers/apiHosts/音质来源限制、GD三来源五档与限流；现有唯一 libmpv 通过有界本机代理播放HTTPS流，支持Range、完整缓存、自动封面/歌词/翻译、手动歌词持久关联。后台完整下载按真实文件头命名、避免覆盖、附带同名资源、加入音乐库并保留原始元数据，选择下载后播放。歌曲/歌词开关独立，关闭歌曲联网仍可播放下载文件/查歌词。
- A01.3 设置：运行目录 config/cloud_api 默认读取及可选目录/重读、缺省内置同一JSON、坏配置保留旧值；歌曲下载/歌词/专辑图/串流缓存目录独立选择及恢复默认；缓存只清理应用专属哈希文件，未移动/删除用户库。
- 文件：core online/settings/path_serde（旧配置默认与Windows cfg警告修正），app online/* 与 online_controller、controller/audio/library/bootstrap，Slint online-search/online-options/settings/audio-options/app 与 viewmodel/shell；自有 HTTPS 服务/验收脚本和实际UI例子、导航回归；config 示例/指南、打包固定示例复制、Cargo.lock/依赖许可元数据及文档。Android runtime路径没有进入PC源码。
- 检查：严格 clippy workspace/all-targets exit0；workspace/all-targets 单测 exit0（core12/mpv9/UI1/app13，新增缓存隔离边界后app14，HTTPS显式ignored后实跑，例子复用相同controller测试）；Test-Online.ps1 exit0（HTTPS最终39.06秒、真实Slint/libmpv、导航）；fmt最终check exit0；Release无online-qa构建 exit0。最终git diff --check、Python语法/文档链接检查exit0。独立配置的普通Release启动/4秒正常退出exit0，真实内核ready=true。缓存键绑定配置/音质ID/曲目变量，配置调整不误用旧档位；此修正后已再次通过严格clippy、app14单测、HTTPS/实际UI并重建Release。
- 自有素材/设备证据：Windows x64、现有固定hash libmpv；原创12秒44.1kHz双声道 WAV/FLAC/LRC与封面、临时localhost CA、独立target配置。串流实际Url position1.020828秒/video=false；下载Local position0.704263秒，关闭歌曲联网后仍可播放并选择「手动候选版本」持久歌词。截图与SHA见报告，不据此宣称DAC位准确。
- 未执行：GD真实曲目/账号供给、物理目录选择、Linux新功能/特殊文件系统、安装升级卸载、USB/HDR/多屏；打包脚本复制固定模板及指南已改，未生成新包。下载无后台service/断点续传，Linux原子发布要求同目录hard link。原有二进制发行许可与设备资格限制保留。
- 下一动作：使用实际Android JSON与授权曲目试用远端供给；在Linux运行同一数据/播放/目录回归及生成新包。本轮功能无需再接Android工程或更换播放内核。

## 2026-10-03：Windows/Linux 共用源码合并准备

状态：Done（文档与 AGENTS 接续规则更新）；用户准备将 dev-linux 合入 master，实际提交 / 合并 / 推送尚未由代理执行。目标是在相应系统用相同 Cargo 命令分别生成 Windows exe 与 Linux ELF，平台 API / 依赖继续使用 cfg 隔离。

- 新证据：用户明确确认 Windows x64 packages 工作流能够编译打包 dev-linux 的共用代码，记录为用户反馈的编译打包通过。未提供 run URL / 提交号，不虚构；此次未新增 Windows 播放 / WASAPI / GPU / 退出 / 安装回归证据。此前 Linux 本机验收与包验证范围保留。
- 改动：README、AGENTS、CONTRIBUTING、发布 / Linux / 矩阵指南、规划当前状态、ADR 0007 状态、libmpv 运行时说明、CHANGELOG 及 [合并准备记录](validation/shared-source-merge-preparation.md)。说明开发分支不决定平台，去除当前入口中仅 Windows 或 Linux 未编译的过时描述；历史日期记录保留，不能据旧“本轮未编译”否定后续用户验证。
- 检查：git diff --check、变更 Markdown 相对文件链接检查、工作流触发器与文档核对、变更范围检查通过。本轮仅改文档，不重复 Rust / GUI / 硬件 / 打包测试，不修改 workflow 或 Cargo.lock。
- 当前触发器：Windows 在 master push 自动打包；Ubuntu 26.04 仍仅 dev-linux push 自动打包，合并后可以手动选择 master；Linux 最低版本矩阵仍只手动运行，未收到真实报告。
- 下一具体动作：用户提交文档后将 dev-linux 合入 master、推送并检查 Windows 自动构建，手动选择 master 运行 Ubuntu 打包与最低版本矩阵。干净系统安装 / 物理输入 / 多设备 / USB / HDR / 广泛发行版与正式二进制发行条件继续按既有报告补证。

以下条目按当时事实保留；当前平台、分支与验证状态以最新条目和相关报告为准。

## 2026-10-03：Linux CI AppImage 宿主工具依赖修复

状态：Done（针对用户第三次日志的依赖补齐、提前检查及实际 AppImage 工具验证）；干净远端 workflow 待新提交运行。日志已证明 Release / deb 成功，新错误为 appimagetool 必需的 file 在 CI 缺失。本地已安装它，此前完整本机打包通过不能证明原 CI 依赖清单齐全。

- 改动：`.github/workflows/linux-packages.yml` 显式安装 file，编译前运行新的工具回归和 `scripts/package-linux.py --check-tools`。`packaging/linux/tools.json` 集中声明必需宿主命令 / apt 包，打包入口与 `scripts/check-linux-deps.py` 共用；任何下载 / 编译 / 输出前报出全部缺项。新增工具边界测试，既有 Git / 源码测试隔离工具阶段；更新 Linux 指南 / CHANGELOG / STATUS 与 [验证记录](validation/linux-packaging-tools.md)。不改应用、Cargo.lock、固定工具 hash 或 dist。
- 检查：工具 / 源码 / Git 共 8 个测试通过；实际校验、解压固定 appimagetool，以仅 dirname / readlink 的宿主 PATH 复现缺 file，补回 file 后真正生成自有最小 AppImage。核对其内置 mksquashfs / desktop-file-validate / zsyncmake 及当前参数需要的外部命令，未把未启用可选路径加成必需依赖。Python 编译、actionlint 1.7.12、git diff --check、文档链接检查通过。
- 本机：只读工具和 apt 全面审计通过，无需新 apt 安装；没有全局设置改动。本轮不重复 Rust / UI / GPU 或播放器完整打包，实际工具测试及此前完整打包结果范围分开。
- 下一具体动作：用户推送后新运行 `Ubuntu 26.04 packages`，检查新工具预检、AppImage 生成、源码及六产物上传；远端成功前不标为 CI 完整打包通过。

## 2026-10-03：Linux CI 完整条件依赖源码准备修复

状态：Done（针对用户第二次打包日志的修复、本地回归与完整离线打包验证）；真实远端新容器仍待新提交运行。用户日志中 Release 已成功、Git 查询通过，`rust_notices` 的无过滤 offline metadata 因未下载 `accesskit_ios 0.1.2` 返回 101；Linux 构建本身只准备本平台依赖，不能代替完整源码缓存准备。

- 改动：`scripts/package-linux.py` 在 Git 预检后运行无 target 限制的 `cargo fetch --locked`；`--offline` 追加 offline，`--skip-build` 同样检查完整缓存，缺失时不开始编译 / 创建包。`.github/workflows/linux-packages.yml` 在 Rust 安装后 / lint 前预取，并接入新增 `scripts/test-linux-packaging-sources.py`；更新 Linux 指南 / CHANGELOG / STATUS 和 [验证记录](validation/linux-packaging-sources.md)。保留全平台 metadata / notices / 原始源码归档，Cargo.lock / 应用代码不变。
- 检查：新增 2 个边界测试覆盖四种模式及失败路径通过；既有 4 个 Git 测试通过，Python 编译、actionlint 1.7.12 和 git diff --check 通过。独立真实 Cargo / 私有缓存 fixture 复现 Linux build 成功但 iOS 条件依赖源码缺失导致 offline metadata 101，补齐后完整 fetch / metadata 成功且 lock hash 不变。
- 完整本地打包：复用既有 Release，项目 Cargo home、`--offline --skip-build`，独立 `target/linux-source-qualification/dist` 生成 deb / AppImage / source 及 JSON 清单；不覆盖 dist。六产物 hash 通过；614 个原始 registry archive 与锁定图完整一致、各 hash 通过，包含 accesskit_ios archive 及 deb 许可 metadata 条目。原 Release hash 不变。没有 apt 安装、全局设置改动、重新编译应用或播放 / GPU / 安装验收。
- 下一具体动作：用户推送后新运行 `Ubuntu 26.04 packages`，验证干净 CI 缓存完整下载和六产物上传；已有硬件、安装与正式发行资格限制不变。

## 2026-10-03：Linux CI 打包 Git 所有者检查修复

状态：Done（针对用户所贴 Git 所有者错误的修复 / 本地回归）；远端完整打包待新提交重跑。用户日志证明 Release 编译成功，后续 `git rev-parse` 因容器挂载 checkout 的所有者检查返回 128；不能将本日志当成已生成 deb / AppImage。

- 改动：`scripts/package-linux.py` 的提交号、dirty 状态、源码清单统一使用仅当前命令生效的 `safe.directory=<当前仓库>`；预检提交号前移至 Cargo 编译前。新增 `scripts/test-linux-packaging-git.py` 并接入 `.github/workflows/linux-packages.yml`；更新 CHANGELOG / Linux 指南 / 本条 STATUS，新增 [验证记录](validation/linux-packaging-git.md)。没有全局 Git 配置修改、所有权修改或通配仓库信任。
- 检查：4 个真实 Git 测试通过，使用 Git 上游 foreign-owner hook，先复现 128 再验证三处查询 / NUL 路径清单 / 信任范围 / 预检失败不启动编译；Python 编译、actionlint 1.7.12 workflow 检查和 git diff --check 通过。未重新编译 Rust / 打包 dist / 触发远端；没有 apt 安装。
- 下一具体动作：用户推送修复后新运行 `Ubuntu 26.04 packages`，确认 deb / AppImage 和对应源码 artifacts 全部生成；原失败运行的 Re-run 仍使用旧提交，不能拿来验证本修复。最低编译矩阵是另一 workflow，本日志不补充旧发行版兼容结论。

## 2026-10-03：手动 Linux 最低编译版本矩阵

状态：Done（工作流 / 汇总程序实现及本地验证）；真实 GitHub Actions 矩阵待用户手动运行，最低编译版本尚未实测。按明确要求覆盖 Ubuntu 18.04、20.04、22.04、24.04、26.04，以及当期 stable 起点 Debian 9 至 Debian 13；Debian 14 testing 可选且不参与正式版最低结论。

- 改动：新增 `.github/workflows/linux-compatibility.yml`、`scripts/linux-compat/` 四个文件、`docs/LINUX_COMPATIBILITY_CI.md`、`docs/validation/linux-compatibility-ci.md`，更新 README / CHANGELOG / STATUS 和生成目录忽略项。没有应用或 dist 变更，没有推送 / 触发远端。
- 验收设计：现代宿主执行 Node Actions，各发行版独立 Docker 用户空间 / apt / 编译器，统一精确 Rust 1.99.0 与 Cargo.lock，Release 完整编译并检查 ELF。源码只读、target 不跨系统复用，Debian 9 / 10 走官方签名归档；libmpv 动态加载，不用运行时版本拦住纯编译实验。
- 汇总：分别输出 Ubuntu / Debian 最早成功，环境 / 工具链 / 下载 / 镜像 / 资源失败与超时保持未决；缺失、重复、错报版本、旧 commit / Cargo.lock / run attempt 不可成为最低版本证据。Markdown 写入 GitHub Summary，JSON 与逐系统完整日志上传 artifact；仅清理本 job 的唯一容器。
- 本地检查：14 个 Python 边界测试通过、Python 编译 / Bash 语法通过、actionlint 1.7.12 工作流检查通过（未启用外部 shellcheck / pyflakes）；YAML 手动入口 / 10+11 个 case 与容器保护检查通过，git diff --check 通过。只读确认 11 个官方镜像均提供 amd64。没有 apt 安装；本机无 Docker，未运行真实旧系统构建。原 Release hash 不变，报告见 [验证记录](validation/linux-compatibility-ci.md)。
- 下一具体动作：用户提交并推送，在默认分支注册 workflow_dispatch 后选择 dev-linux 手动运行；查看 [指南](LINUX_COMPATIBILITY_CI.md) 和该次报告。失败环境项先复测；最低成功候选后续再验真实播放 / GPU / 安装，不将源码编译下限写成已有 Linux 包通用支持。

## 2026-10-03：最低系统版本与链接说明

状态：Done（文档推测范围）。根据用户要求核对 Rust 链接方式、平台 API、锁定依赖及实际 ELF / deb，将最低条件与已验证版本写入 README。改动 README.md、docs/validation/platform-minimums.md、本条 STATUS；应用代码与 dist 产物不变。

- 实测 file / ldd / readelf：主程序动态链接，acosf / atan2f 引用 GLIBC_2.43；dpkg-deb 的依赖确为 libc6 >= 2.43 / libmpv2 >= 0.41。AppImage 不携带 glibc，不能消除该门槛；deb 不能按格式推断 Debian 系通用或由 pacman 安装。
- 核对官方资料与本地依赖源码：Windows 核心推测 10 1607，圆角为 Windows 11 Build 22000；现有 Ubuntu 包 26.04，Debian 14/forky testing 仅为依赖满足候选，Debian 13 不满足。更旧系统重编译需单独配套 runtime / 验证；没有宣称新的实际支持。
- 检查：链接 / 包字段 / Cargo feature 图只读检查及 git diff --check；纯文档更新不重跑播放、编译或测试，不重打包已有对应源码档。报告见 [核查记录](validation/platform-minimums.md)。下一步只有明确要求降基线时，才在旧系统构建 / 验收完整依赖与独立包。

## 2026-10-03：Ubuntu 26.04 / dev-linux

状态：Done（Ubuntu 26.04 amd64 本机开发预览适配与打包范围）；广泛硬件 / 物理交互 / 正式发行资格保留待验证。按用户要求从 master `6e9e9c42e2d7` 新建并切换 dev-linux；未提交 / 合并 / 推送，Windows 分支与既有实现保持。目标是复用当前功能完成 Ubuntu 26.04 amd64 适配，输出 deb / AppImage，先统一审计 apt。

- Core：配置 / 索引 / 文件级 EQ / 歌词 / 背景原始路径字节无损，普通 version=1 UTF-8 数据兼容；特殊编码用实际路径不能出现的 NUL 标记，防旧合法文件名碰撞。新增身份 / 坏编码 / NUL 边界单测。
- Platform / mpv：系统 libmpv2 0.41.0 绝对路径或包内 manifest + hash；Linux 音频采用具体 PipeWire / Pulse / ALSA hw 设备、实际 AO 与初始化日志确认，严格失败暂停、同设备共享回退透明。修正强制 ao 列表覆盖设备后端；原 Windows WASAPI 方法通过 cfg 原样保留。ALSA 稳定 ID 从 proc 在 Engine 枚举，portal 外观后台只读；无新增全局 runtime，zbus 使用既有锁定版本。
- 启动输出：Linux 首次 Load 前提交保存音量 / 设备，Engine 预先枚举实际设备；失联设备拒绝加载和恢复，无默认 AO 初始化。audio_blocked 独立于媒体身份，使 pending_load 期间仍显示错误；请求 / 实际后端不一致不确认独占。真实失联配置为 Idle、0 tracks、无 API 输出格式、错误可见。
- UI / app：原生 Wayland / X11、XDG app id / 图标 / 桌面 MIME、现有库 / 封面 / 歌词 / EQ / 字体 / 背景 / 控件功能；hide 前清 Slint 借用图片，teardown 只释放 GL，修复 Wayland suspend 时 UI 属性重入。prepare_gl、同上下文、先 render_free 后 core、按需视频租约继续保留。实际诊断新增原生窗口系统与 ready。
- Packaging：新增 Linux apt 审计、播放 / UI / GPU / ALSA / 包验收与源码离线准备脚本、desktop / AppRun / tool manifest / notices、Ubuntu 26.04 容器 CI artifact workflow。deb 依赖系统 runtime；AppImage 携带非驱动依赖 / SPA / PipeWire 模块，保留宿主 glibc / GPU 驱动 / 桌面服务；附源码、原始 crate / notices、运行库精确源码版本 / copyright 与 SHA-256。无 root 安装脚本删除用户配置或媒体。
- 已完成检查：fmt、严格 locked / offline workspace all-targets clippy、workspace all-targets tests（36 个独立测试，例子复用不重复计数）、Debug bins / examples 和 Release 构建通过；Python 编译与 git diff --check 通过。apt 全面复审缺失为“无”，代理没有执行 apt 安装或更改全局设置。
- 本机：原生 GNOME Wayland、RTX 3060 Laptop / 610.57.04，PipeWire 1.6.2；真实 FLAC / 标签 / 封面 / 歌词、共享 / 独占流、Pulse 具体 AO / 回退、EQ 7 步、无效 UTF-8 文件名及视频 11 动作已通过。ALSA hw 真实 busy 拒绝，空闲时实际打开 / HW 参数 / 播放；源 44100 / API 48000 Hz，物理 DAC 未知。Wayland / 独立 X11 界面例子、音乐库 13 阶段、视频库 21 阶段 / 10 循环 / EOF 暂停、字体字幕等 22 阶段通过。4K 软件 / NVDEC / 去色带真实 GPU 图片比较通过，不能由帧数代替。
- 最终顺序 Wayland all 验收通过；4K 软件 / 去色带 / NVDEC 为 169 / 179 / 180 次 render，硬解 RGB 差 0、去色带 0.230387、明显偏差 0。ALSA 严格源率保护实测 44100→48000 Hz 后 Paused、位置 0.000396 秒，未播放。deb 提取版 / AppImage 提取版 / FUSE 直接启动分别真实音频与 4K NVDEC 播放、正常退出通过；损坏 bundle 清单拒绝初始化、不回退系统库通过。六个产物 SHA-256 均通过。
- 源码与发布：打包使用同一 Release 快照，记录 dirty 实际工作树；完整 614 个锁定 crate 原始归档 / 原文 notices，包含 Cargo 解析仍需要的 Windows / macOS 条件依赖。源码包准备脚本核验 hash / 生成 vendor，在独立空 CARGO_HOME 中 locked / offline 解析五 crate 完整图通过；这项不是额外源码包完整编译。产物在 dist，二进制 / archives 不提交 Git；功能 / 包检查、图片及源码解析摘要归档到 docs/validation/linux-results.json。
- 未验证：物理对话框 / 输入法 / 快捷键 / 拖放、拖动 / 边缘缩放、Wayland 最小化 / dock 恢复、多屏 / 睡眠、USB DAC / 数字捕获、HDR / AMD / Intel / 其他 compositor、干净系统真正安装 / 升级 / 卸载。Linux 圆角 / 阴影由 compositor 决定，DWM 值为 null；不宣称位准确或所有平台。Windows 本轮未编译 / 真机回归，GitHub workflow 未执行。
- 发行限制：本轮为开发预览包；AppImage 上游预构建 runtime 的完整 Alpine 静态版本 / 可重链接安排仍待补证，Windows 固定 DLL 发行闭包也保留既有待验证条件。详见 [Linux 指南](LINUX.md)、[ADR 0007](adr/0007-ubuntu-linux-and-packaging.md) 和 [验收报告](validation/linux.md)。
- 下一具体动作：用户运行 dist 的 Linux 包；在独立 Ubuntu 用户 / VM 做真实安装和物理操作，再用 USB DAC 验收格式 / 失联 / 捕获矩阵。正式发布前补齐 AppImage 静态 runtime 的精确发行安排，并在真实 runner 执行新 workflow。

## 2026-10-03：媒体库与歌词页可选背景

状态：Windows 开发预览功能实现，独立配置 / 自有 UI 渲染与解码单测通过；物理文件选择和用户图片组合待人工验收。用户要求先核清两个分支、提交既有改动，再于 dev 实现。本地 master 工作区干净，dev 上既有应用图标改动已单独提交为 `da9941b`；本轮背景改动仅在 dev。

- UI：音乐库 / 视频库共用 4 款主题色 SVG（轨道、丝带、地平线、网格），歌词页独立 4 款（圆环、五线、轻纱、星轨）。原背景为默认第 0 项。外观设置新增两组选择、缩略预览、本地图片、透明度 0–100%、模糊 0–32px；跟随当前明暗和强调色，歌词文字与媒体库列表仍保持前景层。
- Core / app：`Appearance` 两组背景配置使用旧设置默认兼容及范围校验；原始 `PathBuf` 保存自选图片身份。现有有界对话框选择 PNG / JPG，单 worker 的有界请求 / 回复处理图片；限制压缩文件 32 MB、源图 2500 万像素，缩至最多 1920×1080，在后台模糊，过期回复不覆盖新设置。图片读取失败显示状态而不阻塞 UI 或复制图片；UI 只按资源 revision 更新图片。无新依赖，也未改变 mpv / GL / 音频默认。
- 检查：`cargo fmt --all -- --check`、严格 `cargo clippy --locked --workspace --all-targets --offline -- -D warnings`、`cargo test --locked --workspace --all-targets --offline`、Debug 与 Release `yyplayer-app` 构建通过。新增旧设置 / 范围和自有 PNG 解码 / 模糊单测；首次测试发现小图被 `thumbnail` 放大，修复后重跑通过。
- Windows Winit + FemtoVG 自有布局例子 `background-preview` 对 8 款图案、视频库、设置页、浅色青绿主题及自绘示例图片透明度逐帧截图通过；`navigation-ui` 实际点击 / 几何、`ui-feedback` 计时 / 控件回归通过。样图与适用范围见 [背景验收记录](validation/backgrounds.md)。
- 未验证：真实 OS 文件对话框逐项点击、用户选择的不同大小 / 透明度 / 模糊照片与网络盘变化、长期内存 / GPU 占用、macOS / Linux。截图仅证明布局和着色，不证明播放、WASAPI、HDR 或位准确。下一步用独立配置经物理文件选择器检查自定义图片保存 / 重启 / 丢失文件恢复，并采样长时资源占用。

更新：2026-10-02。正式源码分支已统一为 master，dev 保留最新开发版本，原 main 的全部提交已合入 master，重复分支已移除。当前交付为 **Windows 音视频播放开发预览**，原始框架于 2026-09-30 完成。Git 框架 `d10a4e8` 保留在历史，视频 `4eca722` 保留在 master 历史中；音频 `8467ca9` 已合入 master，目录音乐库 / 主题 / UI 修复 / 字体 / 窗口 / 列宽与发布准备均已合入 master。

## 2026-10-02：视频重播后无法暂停

状态：Done（本轮播放状态修复 / 本机回归）。继续 dev，未提交 / 合并 / 推送。旧 Release 自有 2 秒视频复现：EOF 后重播位置前进，但 snapshot 仍 Ended，每次点击均 Seek(0)+Resume，UI 仍播放图标。worker 在内核 active / 非 EOF 时恢复 Playing / Paused，Idle / Loading / Error 不被旧属性复活；保持 Engine / GL 边界。

- 改动 worker.rs 的真实状态投影及两个边界单测、video-library.rs 的实际 Slint 按钮回归、Test-VideoLibrary 提示、CHANGELOG / 接续规则。没有修改 GL / 解码默认 / 音频策略或添加 reload。
- fmt --check、严格 workspace all-targets clippy、workspace all-targets tests（28 个独立单测）、Debug yyplayer / video-library 和 Release 构建通过。Test-VideoLibrary 21 阶段及 10 个暂停 / 重播子步骤通过：普通和 EOF 重播后的暂停位置冻结、继续递增，generation / renderer 创建数保持不变；Test-Video 11 动作含 AAC 音轨、NVDEC、暂停及软件解码重载位置恢复通过。
- 最终 Release 同一 2 秒短视频复验通过：3 秒 Ended，4 秒 Playing，5 秒 Paused；引擎 / 渲染错误为空。自有媒体、独立配置；用户视频仅读取编码 / 时长，没有截图 / 复制或修改 APPDATA。原因、最小轨迹和截图见 [验收记录](validation/video-pause-replay.md)。未新增物理键鼠 / 多屏 / 4K 图片 / HDR / USB 资格。
- 保存证据后运行 Clean-Workspace -WhatIf 核对工作区目标与链接，再清理 12,408,954,418 字节（11.56 GiB），target 仅保留 22,159,360 字节新 Release exe，固定 runtime / 头文件保留。清理后独立配置启动 / 正常退出通过，Idle、render 帧 / 创建 0、错误为空；exe hash 未变、DLL hash 符合 manifest，没有终止用户播放器。
- 下一步：用户运行新 Release 确认实际媒体体验；后续回归保留普通暂停与 EOF 重播后暂停 / 继续场景，继续既有生命周期 / 多设备资格验证。

## 2026-10-02：Windows x64 Actions 构建包

状态：实现完成，等待用户推送到 GitHub 验证真实 runner。当前在 dev，未提交或推送；触发条件是远端 master 收到 push。

- 新增 `.github/workflows/windows-release.yml`、`scripts/Package-Windows.ps1`、`packaging/windows/yyplayer.nsi`。Windows 2022 x64 MSVC 稳定 Rust workflow 下载并校验固定 libmpv，编译 release exe，创建带 runtime DLL / 数据文件、许可记录、manifest / commit 源码链接的 portable ZIP 和 x64 Windows 专用 NSIS 安装器，两文件存为同一 30 天 Actions artifact；也可手动 dispatch。
- 选择 NSIS 而非 Inno Setup：NSIS 上游 zlib/libpng 等许可可以商用 / 非商用，runner 清单含 NSIS / 7-Zip，无需将 CI 构建锁定到商业安装器许可。NSIS setup 按架构检查，只装 64 位程序至 Program Files，快捷方式 / 注册卸载清理。项目设置保持原有 per-user `%APPDATA%`。
- `scripts/Get-Mpv.ps1` hash 验证之后优先用 7-Zip 解压经验证的 `.7z`，没有 7-Zip 才退到 tar；工作流不上传 DLL 到 Git 仓库或使用用户机 PATH。
- 输出是逐提交 Actions artifact，不创建 GitHub Releases 页；需 wait for workflow after master push。完整 libmpv 内部组件 notices / 对应源码安排 / DLL 闭包发行审计仍未完成，所以没有宣称正式二进制发行验收。没有在本机或 GitHub runner 执行 workflow；NSIS 安装/卸载、干净 Windows 首次运行、签名与真实 Actions 上传待后续 runner / 真机验收。
- 下一步：源码 push 至 GitHub master 后，从 Actions 下载两份资产并在干净 Windows 验证安装 / 卸载、便携目录启动与运行时加载；随后再补发行许可闭包与签名决策。

## 2026-10-02：修复 NSIS runner 许可路径假设

状态：已修正，等待将 dev 更新推送到 GitHub 后重跑 Actions。首次 hosted runner 在 `Package-Windows.ps1` 第 18 行失败，因为脚本假定 NSIS 安装器一定在编译器旁安装 `Docs/AppendixI.html`。

- 移除对 runner 私有 NSIS 文档目录的硬依赖；改为从仓库打包固定的 NSIS attribution 文件，含官方许可链接和本项目安装器源码位置。NSIS 版本仍从 `makensis.exe` 文件元数据读取并写入 `BUILD-INFO.txt`。
- 改动 `scripts/Package-Windows.ps1` 与新增 `packaging/windows/NSIS-BUILD-TOOL.md`。尚未在本地复跑完整打包，也未重跑 GitHub Actions；下次 workflow 会验证 hosted runner 路径，并继续检查其余 packaging 步骤。

## 2026-10-02：外观设置控件高度与缓存清理

状态：Done（本轮 UI / 本机布局及缓存清理）。继续 dev 原工作区，未提交 / 合并 / 推送。appearance-options.slint 将用户框出的设计 / 明暗 / 主题色来源下拉、颜色输入及应用按钮固定 34px；标签行 60px，不再按面板剩余高度拉伸。

- fmt --check、严格 workspace all-targets clippy、Debug yyplayer / library-theme / navigation-ui、Release 构建通过。Test-LibraryTheme 13 阶段、navigation-ui 10 点通过，已查看 1240×900 / 1460×920 设置截图；文字 / 控件高度正常，无回调或主题策略变更。低风险布局未新增单测，未重跑解码 / 位准确 / 内存专项；物理键鼠 / 多屏 DPI 尚未新增资格。
- 保存自有截图 / 证据到 docs，并归档此前内存原始样本与测量脚本。Clean-Workspace -WhatIf 核对工作区绝对目标 / 无链接，再清理，移除 18,717,681,722 字节（17.43 GiB）；target 仅留 22,158,848 字节 Release exe，固定 DLL / headers 保留，未处理 APPDATA / 用户媒体或终止用户播放器。
- 清理后独立配置 Release 启动 / 正常退出再验通过，Idle、mpv v0.41.0-1087-ge470f8986、render 帧 / 创建 0，无引擎 / 渲染错误；exe 清理前后 hash 一致，DLL 与 manifest 一致。预览 / 清理输出 / 最小诊断及图片见 [验收记录](validation/appearance-controls-cleanup.md)。git diff --check 与证据链接检查通过。
- 下一步：用户体验紧凑外观控件；后续构建会完整重编，继续此前大库 / 多屏 / USB 资格验证。

## 2026-10-02：精简媒体库界面

状态：Done（本轮 UI / 本机布局与流程验收）。按用户框选截图删除音乐宣传卡片、音乐 / 视频库重复标题和索引说明，数量并入筛选工具栏；侧栏、主标题与空闲播放栏统一命名“视频库”。保留未框出的空库引导和库操作。继续在 dev 原工作区，未提交 / 合并 / 推送。

- 改动 music.slint / video-library.slint / app.slint / shell.rs 的布局与可见属性、controller.rs 空闲标题；更新 ui-customization / video-library 新行位置点击坐标、CHANGELOG 与接续规则。没有改 Engine / presenter / GL / 解码或音频默认。
- fmt --check、严格 workspace all-targets clippy、workspace all-targets tests（26 个独立测试）、Debug bins / examples、Release 与 git diff --check 通过；Test-UiCustomization 22 阶段（新列宽拖动 / 移除）、Test-LibraryTheme 13、Test-VideoLibrary 20、navigation-ui 10、ui-feedback 17、audio-ui 五图 / 实际点击通过。
- 已审查浅 / 深音乐库、视频空页 / 列表和 1000×640 选项面板截图，按钮 / 数量无重叠、列表上移、删除内容无空白占位。使用自有素材 / 独立配置；报告与图片见 [媒体库 UI 精简验收](validation/media-library-ui.md)。未新增物理键鼠 / 对话框 / 多屏 DPI 资格，没有重复 4K 像素 / 位准确 / 内存专项。
- 下一步：在用户真实媒体库下体验精简布局，继续补此前大库 / 长时资源 / 多屏 / USB 验收；库与播放资源生命周期保持上一轮实现。

## 2026-10-02：视频库与按需视频渲染

状态：Done（本轮 Windows 开发预览功能 / 本机验收）。用户要求在 dev 修改；dev 原落后 master 一个文档提交，先快进至 1406840，本轮实现留在 dev 工作区，未提交、未合入 master、未推送。

- 视频剧场改为独立库页，目录 / 单文件导入、递归去重、搜索、筛选、重扫 / 取消、移除和重启恢复；video_library 配置 / videos.json 索引与音乐分开，有界后台规范化 / 扫描，移除不删原文件、不改变已建立队列。只索引名称 / 目录 / 文件大小，不为库扫描解码或虚构时长。
- 同窗口 VideoPlayer / VideoPlayerToolbar 抽为独立组件；进入播放才创建 libmpv render context，音频不等待 render。返回库保存本次位置后 Stop；current-vo 实际关闭后释放 render / FBO / texture，继续观看重建并恢复。修正无音轨视频仅关 vid 会 Ended、旧 VO 标志残留、过期错误取消新加载；VideoGate 合并 / 取消旧等待请求。64 项普通命令 + 1 个 Stop 保留槽，不丢弃排队设置，Shutdown atomic；主线程 / Engine / GL 与 prepare_gl 边界保留。
- 涉及 core 配置 / 协议 / 快照、mpv worker / video_gate、app library_service / video_controller / controller / bootstrap、UI 库页 / 播放器组件 / presenter / 投影；新增 Test-VideoLibrary / 真实流程例子，更新导航与歌词点击测试坐标。更新 README / CHANGELOG / ADR 0006 / 接续规则，无新依赖。
- fmt --check、严格 workspace all-targets clippy、workspace all-targets tests（26 个独立测试）、Debug bins / examples 与 Release 构建通过。视频库 20 阶段含 10 次释放 / 重建、快速取消、损坏视频后音频恢复、索引恢复 / 最小布局通过。Test-Video 11 动作、Test-Render 4K HEVC NVDEC / 软件 / 去色带实际 GPU 图片比较通过，RGB 误差 0 / 0.205388、明显偏差比例均 0；Test-UiCustomization 22 阶段、navigation-ui 10、ui-feedback 17、audio-ui 五图 / 实际点击、Test-LibraryTheme 13、Release Test-Audio 通过。
- 同素材 / 默认设置 / 独立配置的串行 Release 短样本：音乐 Working Set 中位数 227.65 → 110.46 MiB，Private Bytes 384.91 → 163.40 MiB；新版 render 帧 / 创建数均 0。这是自有静音 WAV 的观察，不外推为用户 4K 素材 / PotPlayer 对比或 GPU 专用显存。
- 详细命令、硬件 / 驱动 / 素材 hash、生命周期 / 内存 / GPU 指标与图片见 [视频库验收](validation/video-library.md)，设计见 [ADR 0006](adr/0006-video-library-and-lazy-rendering.md)。使用自有素材 / 独立配置，未读取用户锦城湖视频或改 APPDATA。
- 已知边界：当前运行会话续播，未跨重启保存位置；未做视频缩略图 / 时长索引 / 实时监听 / DB。物理文件对话框 / 长时大库 / 网络盘 / 睡眠 / 多屏 DPI / 其他驱动与跨平台仍待验；USB / HDR / 位准确资格未改变。下一步以自有大库和长时间循环补内存 / 响应 / 退出证据，随后补原音频计划的 USB / 数字捕获。

## 2026-10-02：精简项目首页与整理更新日志

状态：Done（文档整理）。目标为 README 只保留功能、编译、运行与必要入口，产品更新集中到 CHANGELOG；保留完整规划供后续开发。

- 改动 README / CHANGELOG / docs/DEVELOPMENT_PLAN / AGENTS / CONTRIBUTING / THIRD_PARTY_NOTICES / STATUS。完整选型、架构、S00–S14 与验收规格从旧 README 转移，规划目标和实际交付明确区分，开发规则改为引用新位置。
- 验收：内联 Python 检查通过（本地 Markdown 链接、原规划 16 节逐字保留、UTF-8、代码围栏、无 emoji、实际包名与启动路径）；git diff --check 通过。首轮检查误用 player-app 目录，按实际 yyplayer-app 修正后全部通过；仅文档变化，不重复编译、Clippy、播放或硬件验收。
- 未改变程序行为或硬件资格，真机范围沿用此前验收。本轮更改保留在 master 工作区，未提交或推送；下一步由用户通过 VS Code 审查、提交并推送文档。

## 2026-10-02：统一 main / master 主分支

状态：Done（本地分支整理）。用户明确要求合并两个主分支，统一保留 master。

- 操作前工作区干净，没有远端，只有当前一个工作树。main=4eca722、master=dev=f01498f；main 是 master 的祖先，main 没有独有提交，master 比 main 多 6 个提交。
- 在 master 执行 git merge --ff-only main，确认已全部包含；随后 git branch -d main 安全删除已合并的重复分支。提交本轮说明后 dev 快进同步，最终仅 master / dev，当前停留 master。
- 更新 README / AGENTS / PUBLISHING / STATUS 和分支检查记录。检查提交可达性、两个保留分支一致、git diff --check 和最终工作区状态；无代码变化，本轮不重跑编译 / 播放测试。
- 所有原 main 提交（包括 4eca722）仍可从 master 访问，没有改写历史。没有远端分支或 GitHub 默认分支可修改，没有推送。
- 下一步：用户在 VS Code 发布 master；目录仍为 E:\SourceFiles\rust\yyplayer。此前章节描述的是当时的分支状态，当前以本节为准。

## 2026-10-02：GitHub 公开源码发布准备

状态：Done（本地源码仓库准备）。用户明确选择 GPL-3.0-only，GitHub 发布由用户在 VS Code 完成。

- LICENSE 使用锁定 Slint 1.17.1 随附的完整 GPLv3 标准文本；Cargo workspace 与五个 crate 声明 GPL-3.0-only，README 明确源码 / 原创资源授权。新增第三方记录和 383 个 Windows normal / build 可达依赖的许可元数据清单，未把它当作完整二进制许可审计。
- 增加 .gitattributes 源码 LF / PNG binary，补忽略库缓存、损坏配置备份与 VS Code 本地设置；新增 CONTRIBUTING 和具体 VS Code 发布步骤。
- 无冲突快进合并 dev 到 master，准备文档一并提交，dev 保留相同发布准备版本，当前停留 master；旧 main 保留历史视频基线。未创建远端 / GitHub 仓库，未推送。
- 检查：Cargo metadata --locked --offline（五个许可字段及 Windows 依赖图）、fmt --check、git diff --check、git fsck、忽略规则、已跟踪文件 / 历史 blob 体积和常见敏感标记扫描。仅许可 / 元数据 / 文档 / Git 配置变更，本轮不重复编译、Clippy、播放或硬件测试；沿用上轮验收。
- 发布根目录：E:\SourceFiles\rust\yyplayer。具体步骤见 docs/PUBLISHING.md；检查证据见 docs/validation/source-publication.md。
- 下一步：用户在 VS Code Publish to GitHub、选择公开仓库，确认默认分支 master。正式二进制发行仍补 runtime notices / 对应源码 / 依赖闭包，S00 / USB / HDR / 跨平台资格不因公开源码标 Done。

## 2026-10-02：原生窗口外观、音乐库封面与歌词页精简

状态：Done（本轮 Windows 开发预览实现 / 本机验证）。目标在 README 先记录；继续在 dev，未合入主分支、未发布。

- platform / app 新增可选 WindowSurface adapter：窗口化 DWM 圆角 + Winit 原生阴影，最大化 / 全屏方角；借用 HWND，仅状态变化更新。原生角部截图已看到真实圆角与阴影；不改变 GL prepare_gl 或 presenter。
- app 新增后台可见行缩略图服务，UI TrackRow / Cover 显示真实内嵌 / 同名 / 目录封面，损坏 / 无封面回退；64px、8 项队列 / 在途、256 项 LRU、重扫 generation 取消，结果按资源批次更新。
- Persistence 排除 recent 比较实际设置变化，history dirty 与 settings dirty、通知 revision 与磁盘 revision 分离；播放静默保存历史，同值不提示，实际成功修改才显示保存浮窗，失败仍可见。
- 大页面删除收起 / 自动跟随 / 更换歌词；自动歌词与 seek 保留，手动导入在播放选项。左下封面切换浏览 / 大页面，大封面返回音乐库；两种真实点击验证通过。
- fmt / 严格 all-targets clippy / all-targets tests / Release 通过，20 个独立单测。Test-UiCustomization 22 阶段、ui-feedback 17 点、navigation-ui 10 点、Debug 视频 11 动作 / 265 次 render 通过。原生 Release 窗口 result=0、preference=2；最大化 / 还原回读验证通过。
- `.gitignore` 已创建且补充本地 .env / 配置、PDB / dump / 原子临时文件 / 系统杂项。已跟踪文件 / 历史体积 / 常见密钥标记检查未发现不应上传的二进制或匹配；没有 Git 远端、根目录没有 LICENSE。GitHub 准备见 docs/PUBLISHING.md，未自动选择许可或推送。
- 安全清理已移除 5,433,771,005 字节（约 5.06 GiB）构建 / 测试缓存，保留 Release exe 与固定 DLL；自有视频临时素材另清理。提交前工作区约 151 MiB，target 仅余 21,758,976 字节 exe。清理后独立配置启动、原生圆角回读、runtime 与正常退出再次通过。
- 已更新 README / ADR 0005 / 本报告 / 接续规则。详细命令、图片与修正记录见 [窗口与封面验收](validation/window-artwork.md)。Windows 10 / 远程 / 贴靠、多屏 DPI 与 macOS / Linux 仍待验；未追加数字捕获或 HDR 资格。
- 下一步：选定应用许可证和正式主分支、合并 dev，再配置仓库并推送；发行包另补 runtime notices / 对应源码 / 依赖闭包。

## 2026-10-01：字体、集成窗口控制栏与音乐库列宽

状态：本轮 Windows 开发预览实现及验收已完成；物理窗体操作与跨平台资格待验。README 在实现前记录目标，继续在 dev，未合入主分支。

- core 新增 Fonts / LibraryColumns 配置默认与校验；platform 新增后台 Windows GDI 字体族枚举，app 新增有界字体服务 / controller，UI 新增字体设置页。全局 / 歌词 / 字幕分别选择，默认继承与缺失回退可见；列比例和字体原子保存。
- Engine 线程应用 sub-font / ASS 字体覆盖，检查返回并尝试全部旧值回滚；默认尊重作者字体。snapshot 回读实际属性。图片 / 烧录字幕不可更换，复杂 ASS 行内排版待专项。
- Window no-frame，自绘主题控制栏与原生 Winit 最小化 / 最大化 / 还原 / 拖动 / 缩放动作；关闭正常清理。Windows 右置、macOS 左置红黄绿布局边界，视频随顶部自动隐藏，全屏不显示。真实最大化 / 还原 / 最小化 / 关闭断言通过；物理拖动 / 边缘缩放、多屏 DPI 和 macOS / Linux 真机待验，不支持原生 Snap 悬停菜单。
- 音乐库歌曲 / 歌手 / 专辑 / 时长列共用表头与行公式，两条分隔线拖动和双击恢复，比例保存；窄窗口简化封面。移除与提示关闭改为居中 SVG 图标，24px 滚动条留空。实际拖动 0.46 → 0.56、保存读回、80 → 79 行移除通过。
- Test-UiCustomization 20 阶段通过：278 字体、真实 Slint 全局下拉、歌词 / SRT / ASS 字体、持久化、两尺寸布局、左置布局 / 点击及 Windows 窗口动作。已审查自有 GPU 图片；纠正隐式位置、下拉重建、过短测试音频及移除后的索引变化，旧失败不作通过证据。
- fmt / 严格 all-targets clippy / all-targets tests 通过，17 个独立单测。ui-feedback 17 点、navigation-ui 10 点复验通过。最终 Release 构建通过；独立配置重启恢复字体 / 列宽、默认 / 缺失字体回退、原生装饰关闭通过。
- 成品自有视频 11 动作 / 266 次 render 回归通过，真实最大化 / 全屏 / 退出、暂停解码重载保留位置、字体保持和正常退出。清理移除约 4.67 GiB 编译 / 测试缓存，提交前工作区约 161.7 MiB；target 只留 21,572,608 字节新版 exe，固定 DLL 哈希一致，清理后三配置启动再次通过。
- 未改变音频默认、WASAPI / EQ / 解码策略或 GL prepare_gl；未重复数字捕获、4K HEVC / HDR / 性能专项。设计见 [ADR 0004](adr/0004-fonts-window-chrome-and-columns.md)，命令 / 图片 / 限制见 [验收报告](validation/ui-customization.md)。
- 下一步：物理窗口拖动 / 八方向缩放 / 双击 / Snap、多屏 DPI，字体安装 / 卸载 / 缺字 / 复杂 ASS，之后按音频记录补 USB DAC 资格。

## 2026-10-01：主界面空白 / 侧栏修复与工程清理

状态：Done（本轮 Windows 开发预览修复 / 清理范围）。继续在 dev，未合入主分支。

- README 先记录目标。app.slint 显式启动页面计时器；内容改为可伸展 Rectangle，阻断子控件最大宽度向上传递。真实复现旧版音乐 / 设置 opacity=0、1240px 中音乐内容仅 394px；修复后 opacity=1、内容 1038px，选项栏始终靠右。
- music.slint 新增居中空库引导、目录 / 单曲按钮和空库工具精简；controller 无启动文件默认音乐库。删除导航版本小字，时尚上下各 10px 留白。
- 新增 navigation-ui：10 点实际 Slint 鼠标事件 / 三尺寸 / 透明度 / 几何 / 减少动态效果 / 简洁主题通过；已查看空库、最小面板、浅色设置和生产库截图。首版回归例子借用未释放的故障已修正，不计作通过证据。
- fmt、严格 all-targets clippy、16 个独立 tests 通过；ui-feedback 17 点、Test-LibraryTheme 13 阶段真实目录导入 / 播放 / 保存 / 多主题复验通过。未修改普通 mpv / WASAPI / EQ / GL 状态边界，未重复 4K 像素或数字捕获专项。
- Clean-Workspace.ps1 先 WhatIf 核对，限定工作区并拒绝链接；保留 Release exe、runtime DLL / 头文件、源码、Git、docs，不处理 APPDATA / 媒体。编译与验收缓存清理后可通过脚本再生成。
- 最终 Release 构建通过；清理前后独立配置空库启动均 page=0、固定 mpv 成功加载、无渲染 / 引擎错误、正常退出。清理移除 28,873,858,504 字节（26.89 GiB）；工程由 29,040,684,385 缩至约 166,825,881 字节（159.1 MiB，含 Git / docs / exe / DLL）。target 只留 20,774,912 字节的 Release exe，固定 DLL 哈希仍一致。后续文档 / Git 提交会带来小幅体积变化。
- 验收报告和保留图片见 [主界面修复记录](validation/navigation-cleanup.md)。上轮 Timer / 页面正常的结论被本次用户反馈纠正；旧测试漏了进入页面的透明度 / 几何断言。
- 下一步：真实 OS 对话框与多屏 DPI 人工验收，以及此前 USB 格式 / 拔插 / 数字捕获资格。

## 2026-10-01：目录音乐库、简洁 / 时尚主题和动效

状态：Done（本轮本机开发预览范围），全平台 / 大库 / 位准确资格未完成。音频基线 8467ca9 已快进合入 master，main 保留视频 4eca722，新功能在 dev。用户魅蓝 DSP 小尾巴 WASAPI 独占确认记录为用户实测，未推断 bit-perfect。

- core 新增 Appearance / LibrarySettings / Song 与默认兼容；platform 新增只读系统外观后台观察；app 新增目录 / 标签 / 缓存服务、取消 generation、目录和单曲管理及队列 / 搜索 Rc 投影缓存；ui 新增主题令牌、GlassSurface、外观设置、虚拟化音乐库和短时过渡。
- 目录添加 / 单曲添加、递归 / 重叠去重、搜索 / 按目录筛选、更新 / 取消 / 移除，独立于播放队列，不删除磁盘文件。缓存原子保存，离线目录保留记录。当前 20000 首 / 200000 项 / 64 层，未完成完整数据库 / 增量监听音乐库。
- 简洁 / 时尚、系统 / 浅 / 深、系统 / 自选主题色独立保存。Windows 实际读到当前 dark=true 和 DWM 色 316da1；手动切换与四种主题截图通过。时尚为轻量透明 / 高光 / 阴影，未实现 Apple 原生折射或实时模糊；视频 GPU 路径不变。
- Test-LibraryTheme 13 阶段实际生产 controller / Engine / GL presenter：目录导入、筛选、真实播放、移除后继续播放、重扫 / 取消、保存和四主题通过。已查看常规 / 设置 / 最小面板图；首轮布局溢出与软件例子的 GL 前置条件遗漏已纠正，未拿旧失败作通过证据。
- ui-feedback 17 点通过；截图耗时不再压缩下一超时间隔，首帧后显式开启计时。页面 Timer 的首次启动缺陷已在上方本轮修复；减少动态效果停持续 Timer / 短动画时长归零。不宣称已测量功耗或性能提升。
- fmt、严格 all-targets clippy、16 个独立测试、Debug / Release 构建通过；新增颜色 / 旧设置、扫描取消 / 排除和离线缓存 / 过期扫描测试。音乐大页面 audio-ui 五图 / 实际 Slint 点击、Test-Video 11 动作与暂停位置重载通过；4K HEVC 图像回归硬解差 0、去色带差 0.205388、明显偏差均 0%。
- 最终 Release 再次通过 Test-Audio：真实 FLAC / 封面 / LRC、共享 / 严格独占、全局 / 设备 / 单文件 EQ、保存 / 正常退出；768kHz 源协商 192kHz 时暂停。最新自有 FLAC SHA256 为 201ea717873fe7d586755daa5e8cb11a68c60330a1b108dbf1b03975a102875a。成功共享回退 / USB 数字捕获仍未增加证据。
- 当前改动和设计见 [ADR 0003](adr/0003-library-and-themes.md)，检查命令、实测与限制见 [验证报告](validation/library-themes.md)。
- 下一步：OS 对话框、真实系统偏好热切换、大目录 / 网络盘 / 长时性能，macOS / Linux 系统主题适配；USB 格式 / 拔插 / 数字捕获资格仍按音频报告。

## 2026-10-01：音频、WASAPI 独占、分级 EQ 与音乐大页面

状态：完成本轮实现与本机开发预览验证，S01 的完整设备 / 位准确资格尚未 Done。目标先记录于 README；视频版本 4eca722 已快进合入 main，音频代码在 dev。

- core 新增音频策略 / EQ 校验、JSON / APO 导入、规则优先级、LRC；mpv Engine 新增 WASAPI 日志证据、同设备回退、严格拒绝 / 源率检查、设备变化暂停、可恢复重试、命名 AF 链替换 / 回滚。正常视频 GL 边界保留。
- app 新增有界标签 / 封面 / 歌词 / 预设后台服务、带媒体身份的回复、共享音频设置与原子导出；ui 新增可折叠音频 / EQ 面板、大页面和动画。默认共享、无 EQ / ReplayGain、自动协商、70% 音量，不伪造 bit-perfect。
- 真实 FLAC / 内嵌 PNG / 同名 LRC、Realtek 共享 48kHz / 独占 44.1kHz、全局 / Waves SoundGrid 设备 / 单文件 EQ 覆盖及清空 / 保存通过。768kHz 源协商 192kHz 时严格源率策略实际暂停。
- Test-AudioBusy：真实 Realtek 独占持有者 + 严格竞争者 + 优先竞争者，严格拒绝，优先尝试同设备共享后也拒绝，未切换其他设备，正常退出。成功回退到共享的实际设备案例仍待验。
- audio-ui 五张截图已审查，封面展开 / 收起 / 歌词 seek 实际 Slint 点击通过；17 点 ui-feedback 复验通过。布局截图带示例标记；不代替播放或 OS 对话框人工证据。
- fmt、严格 workspace all-targets clippy、workspace all-targets tests、Debug / Release 构建通过；新增 8 个边界测试，含旧设置、极端 offset、输入格式、WASAPI 非致命重试、模式 / 设备身份确认与端点变化。总计 13 个独立测试；测试例子复用部分 services 测试，不重复计独立测试数。最终 Release 成品再次通过 Test-Audio / Test-AudioBusy，最终 Debug 成品再次通过 Test-Video / audio-ui。
- Test-Video 11 动作 / 暂停位置重载 / 保存复验通过；4K HEVC NVDEC / 软件 / 去色带 GPU 比较通过，均无明显偏差，平均 RGB 差 0 / 0.205388。不是音频效果或性能证据。
- 成品 target/release/yyplayer.exe。实现 / 设备边界见 [ADR 0002](adr/0002-audio-hifi-preview.md)、[音频报告](validation/audio-windows.md)。USB DAC、真实拔插、数字捕获对比、成功共享回退、广泛格式 / 多声道、跨平台和性能未验证。
- 下一具体动作：先做实际 USB DAC 格式 / 竞争 / 拔插 / 成功回退矩阵，然后数字捕获验证；并补 S00 工具链 / 发行许可。联网歌词、ASIO / DSD DoP / native DSD、gapless 和完整音乐库仍未实现。

## 2026-10-01：视频顶部浮层与控件居中

状态：Done（本机开发预览范围）。目标先记录于 README。视频顶部工具栏移出占位布局，从内容区 y=0 开始覆盖画面，52px 高、3 秒无操作后隐藏，指针 / 触摸 / 按键唤醒；顶部悬停、拖动、打开选项时保持可用。底部左侧媒体信息、整个窗口正中央播放按钮、右侧倍速 / 音量 / 选项；删除独立 1.00x 文字，实际非预设速度也统一在下拉框显示。

- 修改 app.slint / PlayerBar、指针顶部悬停处理、view_model / shell / controller 的倍速投影；未新增依赖、线程，未改变 GL 交接。
- ui-feedback 扩为 17 个检查点并通过：顶部在窗口模式超时 / 悬停保护、没有上方空白、顶部隐藏不改变视频高度；在 1000 / 1240 / 1460px 窗口正中央实际 Slint 点击均命中播放按钮。已查看最小 / 常规可见 / 隐藏、音乐页面截图，没有控件重叠。物理鼠标 / 触摸 / 多屏 DPI 验收仍未增加。
- fmt、严格 workspace all-targets clippy、workspace all-targets tests、Debug / Release 构建通过；成品位于 target/release/yyplayer.exe。
- Test-Video 11 动作、真实最大化 / 全屏 / 暂停解码重载 / 保存 / 退出再次通过；更新自有素材 GPU 图片与诊断数据。最新截图为 1550 × 1125，桌面为 125% 缩放，测试未改变系统设置。
- Test-Render 实际 NVDEC / 软件 / 去色带 4K HEVC 图片比较通过：119 / 144 / 146 次 render，平均 RGB 差 0 / 0.20539，明显偏差比例均 0%。与此前 175% 结果不做性能比较。
- 实现与验证范围见 [UI 记录](validation/ui-feedback.md)。下一步仍为下方接续计划，不扩大其他 GPU / 平台资格。

## 2026-10-01：浮空提示与全屏控制栏

状态：Done（本机开发预览范围）。目标先记录于 README。设置保存等状态移为浮空卡片，4 秒消失，可点击 × 关闭；文字变化 / 新保存 revision 重新显示，常规状态投影不重置计时。视频全屏控制栏覆盖底部，3 秒空闲后隐藏，指针 / 触摸 / 按键唤醒；悬停控制栏、按住拖动、打开选项时保持可见，失焦释放交互保护。退出全屏恢复常显，显示 / 隐藏不改变视频尺寸。

- 涉及 app.slint、shell / view_model、bootstrap / controller；新增开发例子 ui-feedback，未新增依赖、线程或改动 mpv GL 状态交接。
- ui-feedback 的 12 个检查点通过，包括实际 Slint 关闭按钮点击、相同文字再次提示、关闭后投影、超时和交互保护；已查看软件布局截图，不冒充 OS 物理鼠标 / 触摸 / 多屏 DPI 验收。
- fmt、严格 workspace all-targets clippy、workspace all-targets tests、Debug / Release 构建通过，成品位于 target/release/yyplayer.exe。
- Test-Video 真机复验通过 11 动作、窗口模式、暂停解码重载 / 持久化 / 安全退出，更新自有素材 UI 截图与诊断 JSON。
- Test-Render 再次通过实际 NVDEC / 软件 / 去色带 4K HEVC 图像比较：133 / 138 / 139 次 render，无去色带平均 RGB 差 0，去色带 0.24812，明显偏差像素均 0%。Test-Video / Test-Render 显式 UTF-8 读取 JSON，修复 Windows PowerShell 5.1 默认 ANSI 解码误报。
- 详细实现、检查范围与人工待验场景见 [交互验证记录](validation/ui-feedback.md)。下一步仍按下方接续计划；macOS / Wayland / X11 尚未验证。

## 2026-10-01：修复硬解横向噪点

状态：Done（本机问题修复 / 验证范围）。用户报告 4K HEVC 在关闭去色带时画面损坏。独立配置实际复现 NVDEC + 无去色带异常；软件与去色带正常。原因是 presenter 遗漏 libmpv 所需的共享 GL 默认状态交接。现对 create / update / render / free 恢复标准状态，缩放分配 FBO 后也再次恢复；保留默认硬解与去色带设置，不强制软解 / deband、不增加正常视频 CPU 回读。

- 原视频取样：修复前 25.66795% 像素明显偏离软件参考，修复后 NVDEC / deband=false 同区域误差为 0；原文件未修改，个人图片 / 视频不提交。
- 新 Test-Render / render-compare：自有静态 4K HEVC、实际 NVDEC / 软件 / 去色带 GPU 图片比较通过，补足上一轮只验状态 / 帧数所遗漏的画质检查。
- fmt、严格 all-targets clippy、workspace tests、debug / release 构建通过；Test-Video 的最大化 / 全屏 / 暂停解码重载复验通过，更新自有素材 UI 图片及控制数据。
- 旧 release 窗口保持运行；旧 executable 已在 target/release 同目录改名备份，新 yyplayer.exe 重新构建，用户需关闭旧窗口后重启。
- 详细原因、固定源文件依据、hash / 像素阈值 / 当前资格边界见 [渲染修复报告](validation/render-state-fix.md)。原视频报告里的状态检查仍有效，旧图像不再当作画质通过证据。

## 本轮实际实现

- 五个 crate 沿用：core 新增设置 / 快捷键 / 状态机；mpv 新增固定 runtime manifest、校验脚本、受控 loader、FFI、worker / render 租约；ui 新增 GPU 呈现和真实共用控件；app 新增事件 / controller / 文件对话框 / 原子持久化。
- 本地音视频实际播放，多文件打开 / 拖放 / CLI / 最近文件、队列、暂停 / 停止 / seek、音量 / 静音 / 倍速、输出设备、轨道 / 外挂字幕 / 延迟、章节 / 逐帧 / PNG / 循环 / 比例均已接入。
- 视频页紧凑、普通 / 最大化 / 全屏；音乐与视频共用 PlayerBar。真实运行显示本地队列与实际封面 / 标签 / 歌词；开发例子的样例播放状态有标记，不作为设备输出证据。
- 快捷键可配置；上下音量、左右短按默认 5 秒、长按默认 3x 后恢复；焦点 / 对话框 / 页面 / 媒体变化取消临时加速。实际运行信息包含请求与实际硬解路径。
- 全局 → 最深祖先文件夹 → 单文件完整解码规则（后者优先）；自动 / 软件 / 硬件优先、线程 / 去隔行 / 去色带，保存并重载保留暂停 / 位置。设置与最近 30 个文件后台原子保存。
- 正常视频 libmpv OpenGL → RGBA8 GPU FBO → Slint 借用纹理，不每帧 CPU 回读。开发预览路径决定见 [ADR 0001](adr/0001-libmpv-video.md)；原框架决定见 [ADR 0000](adr/0000-framework-preview.md)。

## 实际验证

本机 Windows 11 / Rust 1.97.0 / Slint 1.17.1，固定 mpv v0.41.0-1087-ge470f8986，RTX 5060 Laptop。详细 runtime / 驱动 / fixture hash 与未测场景见 [Windows 视频报告](validation/video-windows.md)。

| 实际执行 | 结果 |
| --- | --- |
| 获取固定 runtime、archive / DLL SHA256、ABI 核对 | 通过，本地 DLL 可实际加载；二进制不提交。 |
| cargo test --locked --workspace --all-targets --offline | 通过，当前 16 个独立测试；services 在例子 target 的复用执行不重复计数。 |
| cargo clippy --locked --workspace --all-targets --offline -- -D warnings | 通过。 |
| cargo build --locked -p yyplayer-app --offline | Debug 通过。 |
| cargo build --locked --release -p yyplayer-app --offline | Release 通过。 |
| cargo fmt --all -- --check / git diff --check | 通过。 |
| scripts/Test-Video.ps1 -SkipBuild | 真机通过 11 动作 / 暂停软解重载 / 规则持久化 / 独立软解 / 正常退出。 |
| release 成品 7 秒运行 | NVDEC、192 次 render、进度 6.333 秒、无错误、exit 0。 |
| 真实 GPU UI 截图与源帧对照 | 修复方向与 slider fill，对照确认；见 [截图](video-ui.png)。 |

脚本走生产 controller / 内核 / native window，不等于 OS 键盘 / 对话框逐项操作验收。所有功能广泛素材 / 设备矩阵和稳定性仍待补齐。此前探索阶段未通过的 render 日志不作为最终 Pass 证据；本轮固定构建的 ns/us 差异已写 ADR 和单测。

## 阶段状态

| 任务 | 状态 | 本轮证据 / 剩余 |
| --- | --- | --- |
| 完整规划 / 原 UI 框架 | Done（各自限定范围） | DEVELOPMENT_PLAN / AGENTS / ADR 0000；旧音乐布局截图 ui-preview.png。 |
| 本轮：视频功能实现 | Done（开发预览范围） | 实际 engine / GPU / 控件 / 快捷键 / 规则 / 保存；变更见 CHANGELOG，资格限制见验证报告。 |
| S00 | Partially verified | 可追溯 runtime、loader、锁文件、debug / release 已有；具体 toolchain pin、发行许可 / 依赖闭包未完成，MSRV 未测试。 |
| S01 | Partially verified（音频开发预览） | WASAPI 共享 / 独占、分级 EQ、源率拒绝与竞争已验；USB DAC、物理拔插、成功共享回退、数字捕获未验。 |
| S02A | Partially verified | GL 合成本机 NVDEC / 软件播放、方向 / 帧数 / 退出通过；广泛格式、性能、跨平台待验。 |
| S02B | Not started | Windows HWND / D3D11 / HDR 专项仍是候选。 |
| S03 / S04 | In progress（真实实现） | worker、异步 UI、会话队列、导入、设置 / 最近保存已有；复杂竞态、DB、正式队列语义、库待补齐。 |
| S05–S08 | In progress（部分功能） | 音视频控制、独占 / EQ / 本地歌词已接；发行、真实设备失联与全素材矩阵未完成。 |
| S09 | Partially implemented | 目录音乐库 / 标签索引 / 筛选 / 缓存已有；DB / 专辑 / 增量 / 大库资格未完成。 |
| S10–S12 | Not started（专项） | gapless、续播、系统媒体键、HDR、性能定型未完成。 |
| S13 | Not started | 仅 cfg / 库入口 / 设置路径预留，macOS / Wayland / X11 未编译 / 实机验证。 |
| S14 | Not started | 安装 / 卸载、许可、长时稳定性与正式发布未完成。 |

## 接续步骤

1. 按验证报告人工用例补齐 OS 快捷键 / 对话框、多轨字幕、解码优先级、快速连续切文件 / EOF / 重播、窗口与设备变化。
2. 固定 Rust release toolchain 并核对锁定依赖的 MSRV；选择应用 / Slint 许可和合格 runtime 对应源码 / notices，验证 portable 包依赖闭包。
3. S01 的实际 USB DAC 格式 / 独占 / 成功回退、物理设备失联和数字捕获；本机实现与资格边界见音频报告。
4. 性能基准和原生 / HDR 资格，再执行跨平台编译及真机矩阵。

无阻塞本轮开发预览的已知问题。尚未完成的资格条件不能写“支持所有平台 / 格式”“零拷贝”“HDR 输出”“独占已确认”。

## 后续每轮记录

记录日期、具体任务、修改范围、真实执行的检查、runtime / 驱动 / fixture hash、证据路径、未验条件、已知问题、下一具体动作。状态允许 Not started、In progress、Partially verified、Done、Blocked；用户范围优先，但不得把没跑过的检查写为通过。
## 图标方案（2026-10-02）

- 当前任务：提供 10 个供选择的 YYPlayer 程序图标矢量稿，其中 06–10 为第二组；尚未选定正式图标，也未接入安装包资源。
- 改动文件：`assets/icons/yyplayer/option-01-signal.svg` 至 `option-10-duo.svg`。
- 检查：未运行命令行校验；本轮 `exec_command` 启动持续返回 `helper_unknown_error: setup refresh had errors`。SVG 为手写静态矢量，待在目标系统图标尺寸下预览确认。
- 真机场景：未进行；本轮只交付静态图标稿。
- 未验证：Windows 文件图标尺寸 / DPI 下的视觉效果及最终选型。
- 下一步：用户选定方案后，将其接入应用与安装包图标资源，并验证 Windows 构建展示效果。

## G08 多尺寸 PNG 导出（2026-10-02）

- 当前任务：将精修版 G08 SVG 栅格化为多尺寸程序图标 PNG。
- 改动文件：`assets/icons/yyplayer/liquid-orbit-disc/` 下 12 个 PNG 与目录说明。
- 检查：使用本机 Sharp 从 SVG 直接渲染，确认 16 × 16 至 1024 × 1024 的输出尺寸；PNG 保留 Alpha / sRGB。查看 16 px 与 256 px 预览，符号在小尺寸仍可辨识。
- 未完成：尚未接入 Windows EXE / NSIS 图标资源，也未生成 ICO。
- 下一步：将 256 px PNG 纳入 ICO 多尺寸资源并接入 Windows 应用 / 安装器图标配置，再构建检查显示结果。

## G08 程序图标全面接入（2026-10-02）

- 当前任务：将用户选定并精修的 G08 图标接入应用界面、Windows 原生窗口 / 任务栏、EXE 资源及 NSIS 安装器 / 开始菜单快捷方式。
- 改动文件：`assets/icons/yyplayer/liquid-orbit-disc/`（多尺寸 PNG、ICO、说明）；`scripts/Build-AppIcon.ps1`；`crates/yyplayer-app/build.rs`、`src/bootstrap.rs`；`crates/player-ui/ui/theme.slint`、`app.slint`、`components/window-chrome.slint`；`scripts/Package-Windows.ps1`；`packaging/windows/yyplayer.nsi`；Windows Release workflow、`.gitattributes`、CHANGELOG 与本报告。
- 检查：`cargo fmt --all --check`、`cargo clippy --locked --offline --workspace --all-targets -- -D warnings`、`cargo test --locked --offline --workspace --all-targets`、`cargo build --locked --offline --release -p yyplayer-app --bin yyplayer` 均通过；UI preview 已生成并人工检查；图标生成脚本通过，ICO 解码并确认包含 10 个尺寸；Release 构建生成 Windows `.res` 图标资源；`git diff --check` 通过。
- 真机场景：本机 UI preview 确认侧栏与自绘标题栏显示 G08；未启动真实播放器窗口 / 任务栏验收，也未实际安装 NSIS 包。
- 未验证条件：本环境未安装 `makensis.exe`，不能在本机编译或安装 NSIS 包；待 Windows Release CI 的 NSIS 阶段验证安装器产物。Windows 原生窗口 / 任务栏图标仍需实机视觉确认。
- 已知限制：构建脚本仅对 Windows MSVC 嵌入 EXE ICO；非 MSVC Windows 构建会跳过该资源。Slint 中的品牌位图内嵌于应用，运行时窗口图标从同一 256 px PNG 设置。
- 下一步：在具备 NSIS 的 Windows Release runner 上验证 Setup 与卸载项图标，并在实际 Windows 窗口 / 任务栏、资源管理器与开始菜单检查显示效果；无需改变程序功能逻辑。

## G08 侧栏品牌对齐修正（2026-10-02）

- 当前任务：修复用户 Windows 截图中侧栏图标高于“YYPlayer”文字的错位；用户反馈 Windows 系统图标本身已显示正常。
- 改动文件：`crates/player-ui/ui/app.slint`、`CHANGELOG.md`、`docs/STATUS.md`、`docs/validation/2026-10-02-icon-concepts.md`。
- 实现：品牌行仍为 76px，40px 图标按行高度居中，文字占整行高度并垂直居中；保持原有 10px 水平间距。
- 检查：`cargo run --locked --offline -p yyplayer-app --example ui-preview -- --output target/icon-alignment-preview.png` 通过，人工查看图标与文字中心对齐；`cargo test --locked --offline -p player-ui`、`cargo fmt --all --check`、`cargo build --locked --offline --release -p yyplayer-app --bin yyplayer`、`git diff --check` 通过，Release EXE 已更新。
- 真机场景与未验证条件：本轮通过软件渲染界面预览检查布局；未重新启动用户的真实播放器窗口验证不同 DPI / 字体设置。图标资源与 Windows 系统图标配置未改动。
- 下一步：用户下次运行新构建时可对照原截图确认实际 DPI 下的对齐效果。
