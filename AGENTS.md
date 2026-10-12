# YYPlayer 开发执行规则

本文件面向后续人类开发者和代码代理。项目概览与启动方法见 [README.md](README.md)，产品目标、选型依据、模块接口与逐阶段任务以 [开发规划](docs/DEVELOPMENT_PLAN.md) 为准，已交付更新见 [CHANGELOG.md](CHANGELOG.md)。当前为 Windows x64 / Ubuntu 26.04 amd64 共用源码的 libmpv 开发预览：真实播放、GPU 合成、分级解码设置、快捷键、媒体库、EQ 与本地歌词已接入；Windows 使用 WASAPI，Linux 使用 PipeWire / PulseAudio / ALSA。用户已确认 dev-linux 的 Windows Actions 编译打包通过；USB DAC / 位准确、系统集成和广泛平台资格仍未完成。不能把规划描述或示例 UI 当成已实现功能。

## 0. 顶层分支规则（用户明确要求，2026-10-12）

- **唯一的顶层开发分支与开发起点是 `dev`。`master` 只允许合入 `dev` 的已验证改动，不能直接开发、编辑或提交修改。** 本规则覆盖源码、UI、脚本、配置、版本号、文档和 AGENTS.md 本身。
- 修改前先运行 `git branch --show-current`。当前为 master 时，先检查工作区与分支进度、切换到 dev，再编辑；不能在 master 修改完成后才改分支名称。下级任务 / 独立工作树也必须以 dev 为开发起点，改动先整合到 dev，不能从 master 开始开发来绕过此规则。
- master 合入有冲突时，先在 dev 合并最新 master 并处理冲突、验证，再让 master 快进合入 dev；不得在 master 手工修代码或解决内容冲突。合入结束后回到 dev 继续工作。
- 提交、合入 master、推送和发布各自按当轮用户授权执行；本地修改 / 打包不代表自动授权合入 master 或推送。不得使用强制推送或重写历史来同步分支。
- 后文“源码主分支为 master”指集成 / 发布分支，不能解释为允许在 master 修改代码。历史 STATUS / ADR 的旧分支操作仅为当时记录，不是当前执行规则。

## 1. 接手时的执行顺序

1. 读 README 的项目概览与启动方法，再读 docs/DEVELOPMENT_PLAN.md 中选型结论、视频呈现、音频策略与当前任务对应章节。
2. 读 [docs/STATUS.md](docs/STATUS.md)，检查实际文件、工作区修改、runtime manifest、上一任务的验收证据。
3. 用户没有另行指定范围时，从 S00 起按前置依赖选取第一个未完成任务。当前 UI 优先交付来自用户明确要求，范围见 `docs/adr/0000-framework-preview.md`；本轮视频优先交付见 ADR 0001；复用现有实现并按 STATUS 补齐剩余资格验证，不重新创建工程。
4. 写清本轮目标、涉及模块、验收方式；实现一个可审查的任务或其明确子步骤。
5. 完成适当检查与运行验证，把结果、限制和下一具体步骤写入 STATUS。环境缺失记录待验证，完成其他独立工作。

用户的明确要求优先于本文件；用户只要求规划时，不擅自推进应用实现、安装工具或改变全局系统设置。本文件不要求在常规实现、修复和可逆文档更新前再次索要批准。

## 2. 已选定的架构

- Rust + Slint + libmpv，音视频使用统一内核和 AppController。
- 同一 Cargo workspace / Cargo.lock 支持 Windows x64 MSVC 与 Linux amd64 GNU；平台实现和依赖用 cfg 隔离，Linux 已接入 Wayland / X11。macOS 仍保留边界，未编译 / 真机验收。
- 技术关卡必须先验证：共享 / 独占音频、GL Render API 合成、Windows HWND / D3D11 呈现。
- 当前 Windows 开发预览采用 GL Render API + Slint 借用纹理，事实与限制见 ADR 0001；HWND / D3D11 和 HDR 输出专项仍需独立验证。
- 不默认引入第二套音频引擎、自建时钟、插件系统、网络服务或全局异步 runtime。

必要选型变化写入 `docs/adr/`：背景、事实证据、候选、决定、影响和验证。不能仅因为 wrapper API 不方便换内核。

## 3. 不得违反的线程 / 生命周期规则

- Slint 组件在主线程创建与修改，worker 使用合并投递与弱引用。
- 普通 libmpv 调用属于 Engine 线程；UI 回调只提交命令，不能等待 Engine。
- 已有 MpvEngine worker、64 项普通命令 + 1 项 Stop 保留槽的有界通道、最新快照和 render 租约。controller 仅提交命令；禁止把普通同步 mpv 调用搬到 UI / GL notifier。初始化 / 退出也必须保持该边界。
- GL notifier / render 线程只操作允许的 render API；不能同时发普通同步 mpv 命令，不能持有 Engine 需要的锁。
- mpv 回调只做非阻塞唤醒，不能 render、更新 UI、等待或执行重工作。
- OpenGL render 调用串行，使用创建 render context 时相同的当前上下文。
- 借用纹理不能跨 Slint 窗口复用；释放前保证 UI 不再使用；旧 GL ID 不能跨 context 重建复用。
- render context 先于 mpv core 销毁。原生 presenter 先停止 / 清理 mpv 输出，再销毁宿主窗口。
- callback context 的 lifetime 必须覆盖解绑与在途回调；FFI 指针数据跨线程前复制。
- `unsafe` 只在边界模块，注明 `SAFETY` 条件；不能凭经验添加 `Send` / `Sync`。
- 通道有界；高频值合并；Stop / Shutdown 始终可达。退出不能以 `process::exit` 掩盖资源清理缺陷。

## 4. 音频与视频语义

- 请求独占不等于已确认独占；独占不等于 bit-perfect。当前设备、API 输出格式、物理硬件格式分开表示，未知就显示未知。
- 不默认强制 192 kHz / 32 bit、音量超过 100%、EQ、响度归一化或额外音效。
- 严格模式失败不回退；优先独占按保存策略回退并保持可见状态。设备意外失联默认暂停，不突然切到扬声器出声。
- DSP 与独占可并存；未处理候选模式与会改样本的功能互斥；压缩直通与 PCM DSP / 变速互斥。
- EQ 链有名称、有验证、有更新回滚；不持续叠加节点，全部为零时移除节点。
- 同格式 gapless 依赖同一 mpv playlist；不同采样率保持源格式与绝对无缝之间的取舍必须透明。
- HDR 素材、HDR → SDR、合格 HDR 显示输出分别标记；普通 RGBA Image 不能证明 HDR 输出。
- Render API 不当作可直接使用 D3D11 / Vulkan / WGPU 的接口；Wayland 不使用 `wid` 冒充原生合成。
- 硬件解码与零拷贝分别验证；有 copy fallback 时记录实际路径。
- 不用每帧 CPU RGBA 回读 / 上传作为正常视频架构。
- 自动 next 只由一个明确的实际执行方触发；队列 ID、mpv playlist ID 与媒体 generation 分开。

## 5. 实现与验证要求

- `player-core` 无 Slint、mpv、OS API；平台差异限制在 platform / presenter。
- 绑定、crate、选项和构建版本依据锁定版本官方资料核实；禁止编造 Slint / mpv API。
- 所有初始化参数检查返回值，按 Required / Optional 分级；缺 P0 能力不能继续展示假成功。
- 文件路径保留平台原始表示，显示字符串不作为文件身份；不拼接 shell 命令处理媒体路径。
- 本地打开、标签读取、DB、设备选择、视频呈现出错都应返回可恢复的分类错误。
- 配置版本化、迁移可测、坏数据可恢复；原子替换，不先删除唯一文件。
- 新依赖最少 features、提交应用 Cargo.lock，运行时二进制用 manifest + hash 锁定。
- 不依赖开发者 PATH 上偶然存在的 DLL；最终包自带合格 runtime 与必要 notices / 源码安排。
- 功能实现按 docs/DEVELOPMENT_PLAN.md 第 14 节做 fmt、clippy、相关 tests 与 release build。用户限定只做框架与少量检验时，按该范围做必要格式 / 编译 / UI 预览检查，在 STATUS 明确省略项；不要把没有跑过的检查写为通过。
- `demo.rs` 与 `MediaPreview` 仅为展示，不转换成真实 PlaybackSnapshot，不伪造播放、设备、独占、EQ 或硬解成功。接入媒体后使用真实状态源替换 demo。
- Slint 与 slint-build 当前固定 1.17.1；升级两者一起改并验证对应 API。`ui-preview` 的软件截图只用于布局审查，普通应用默认 Winit + FemtoVG。
- 测试覆盖边界、竞态和数学 / 数据风险；不为低风险文字修改造无意义测试。
- 真实 DAC、HDR、多 GPU、macOS / Wayland / X11 验收需要真实环境；CI 编译不能代替。
- 性能测试用 release，记录硬件 / 驱动 / 参数 / 素材 hash 与口径。不同默认值的独立 mpv 数据不能直接拿来做对比。
- UI 日常流程不要求用户理解 mpv 选项；底层字段进入媒体信息 / 诊断页。

## 6. 每次交付与进度更新

STATUS 中填写：当前任务 / 子步骤、改动文件、检查命令及结果、真机场景结果、未验证条件、已知缺陷、下一步明确动作。相关报告放 `docs/validation/`。

任务只在代码和要求的证据都满足时标 Done。样机结论写 ADR；改变默认路径或质量配置必须有新证据。

交付说明用中文，简述实现结果、验证、限制。不能以“已支持所有平台”“完美音质”“零拷贝”“无缝”“HDR”替代明确的适用组合与测量。

## 7. 接续当前实现

先查 STATUS 与 ADR 0001 / 0002 / 0003。此前用户目标已在各轮实现前记录，已交付变更现汇总于 CHANGELOG，完整规划保留于 docs/DEVELOPMENT_PLAN.md；音频版本已合入 `master` 的 `8467ca9`，视频 `4eca722` 已保留在 master 的提交历史中；旧框架 `d10a4e8` 保留在 Git 历史。用户没有明确新任务时，优先补齐当前实现的格式 / 多设备 / 生命周期资格验证和 S00 工具链 / 发行许可；再补齐 S01 的 USB DAC / 位准确资格。不要把视频预览重新降为假状态或占位引擎。

- 固定运行时见 `third_party/mpv/manifest.json`，获取脚本 `scripts/Get-Mpv.ps1`，不要提交 DLL / archive。
- `YYPLAYER_MPV_LIBRARY` 明确绕过 hash，仅供开发；发行路径必须受控。
- 新增解码选项时同步更新 validate、分级规则、controller、UI、重载恢复和验证；规则是完整对象覆盖。
- 快捷键需同时考虑输入焦点、repeat、modifier 释放、失焦、文件对话框和退出的临时速度恢复。
- 生产 GPU 合成不得改为 CPU 图片循环；debug 单次截图与实际视频路径分开记录。
- Slint / FemtoVG 会留下 blend / active texture 等 GL 状态；进入所有 libmpv render 调用前使用 presenter 的 prepare_gl 恢复默认状态。不得删除这个边界而用默认开启去色带 / 软解掩盖画面损坏。升级渲染器后核对 render_gl.h，并重跑 Test-Render 的图片比较。
- 播放状态、实际硬解名称和帧数只能证明运行，不能证明画面正确；涉及 GL 状态 / 纹理 / NVDEC 的修改必须比较真实 GPU 图像。Test-Render 使用自有静态 4K HEVC，不提交用户视频或其截图。
- 固定 git runtime 的 render target 使用 ns，头文件注释为 us；修改 presenter / 运行时先核实 source 和 clock ABI，并跑时序测试 / 实机帧数，不能凭注释盲改。
- 不在每次投影重建相同模型或覆盖用户正在编辑的表单；设置字段由 revision 驱动。
- 字体族在后台枚举，主线程只投影结果；目录未变化时不重建字体下拉模型，否则选择可能被控件初始化重置。字幕字体只通过 Engine 命令设置，检查与回滚 sub-font / sub-ass-style-overrides / sub-ass-override；默认保留 ASS 作者字体。全局 / 歌词 / 字幕字段独立，缺失字体回退有可见提示，不抹掉保存选择。
- 自绘窗口控制区使用 UI 线程 Winit 原生动作，关闭走正常资源清理；左置交通灯布局演示不是 macOS 资格。视频窗口标题区覆盖画面并随顶部控件隐藏，全屏不显示；布局诊断必须区分内容局部坐标和窗口坐标。音乐库表头与行使用同一列宽公式，拖动松开后保存比例，为滚动条和固定操作区留空间。修改这些交互后运行 scripts/Test-UiCustomization.ps1，另外回归 ui-feedback / navigation-ui；物理拖动、边缘缩放和多屏另记待验。
- 页面进入计时器从停止状态必须调用 start，不能对从未启动的 Timer 只调用 restart 后把页面透明度留在 0。内容容器显式可伸展，避免子控件 max-width 向上限制页面宽度、把选项面板挤到中间。修改导航后运行 navigation-ui 的实际 Slint 点击 / 透明度 / 几何断言，而不只检查 controller.page。
- 状态提示是 UI 层临时浮空信息，重复状态投影不得重置超时 / 恢复手动关闭；新保存 revision 可以再次通知。全屏控制栏覆盖视频，隐藏不能改变视频 / FBO 尺寸，按住拖动、悬停控件和打开选项时保留可操作性。修改此交互后运行开发例子 ui-feedback；它只证明 Slint 计时 / 点击逻辑，不代替物理设备验收。
- 视频顶部工具栏也覆盖画面并自动隐藏，不恢复原顶部空白占位；播放按钮组相对整个窗口居中，不依赖左右内容宽度分配。实际倍速统一由下拉显示，非预设值也必须正确选中。布局修改后核对 ui-feedback 的最小 / 常规 / 较宽窗口中心点击与截图。
- `scripts/Test-Video.ps1` 使用独立配置；新增验收不能改用户默认 APPDATA 设置。
- 当前媒体位置重载已验证，快速连续打开 / EOF / 设备失联 / 睡眠 / 多屏仍需增加证据。

启动命令与操作见 README。Ubuntu 26.04 amd64 已有编译 / 本机播放 / GPU 与包提取启动证据，Linux 接续以第 12 节为准；其他发行版与 macOS 仍需各自验收。运行报告里区分生产 controller 脚本、状态机单测和实际键盘 / 对话框人工验收。

## 8. 音频开发预览接续

- WASAPI 独占必须结合固定 runtime 的格式接受、初始化完成和 actual AO；缓冲区对齐错误可能由内核重试恢复，不因单条日志提前判失败。更换 runtime 后重新核对日志协议，未知状态不显示已确认。
- 默认共享 / 无额外 DSP / 自动采样率协商 / 70% 音量；不改为默认 100% 或强制 192kHz。源 PCM、API PCM、物理 DAC 格式分开；bit-perfect 需要数字捕获。
- 优先独占只回退同设备共享；明确设备 / 端点失联暂停，严格源采样率不匹配暂停。重试可恢复媒体位置且保持暂停，不能突然自动换设备出声。
- EQ 单文件 > 具体设备绑定 > 全局，完整对象覆盖；有名称、有限参数、固定链替换和回滚，平直移除链。JSON / APO 参数导入不接受任意 filter / Include / 不等价 GraphicEQ。
- 标签 / 图片 / 歌词 / 预设文件在有界后台服务读取；过期媒体 revision 不覆盖当前歌曲，歌词模型和图片仅资源 revision 更新。动画只在相关大页面运行；Slint 主线程不读整首音频或解码图片。
- Test-Audio / Test-AudioBusy 使用自有 fixture / 独立配置；audio-ui 是布局样例。不得把设备竞争双失败测试当作成功共享回退，或把 PCM 位宽当成 DAC 实际格式。USB 拔插 / 成功回退 / 数字捕获仍按报告补证据。

## 9. 工程缓存清理

`scripts/Clean-Workspace.ps1 -WhatIf` 先预览；不带参数清理 target 的编译 / 测试产物和 runtime 下载 archive / 未使用导入库，保留 target/release/yyplayer.exe、可执行文件旁 runtime（若存在）、固定开发 DLL / 头文件、源码、Git 与 docs。脚本校验工作区路径及 junction / symlink，不处理用户 APPDATA 或媒体目录。先完成验证并保存必要证据，再清理；下一次 cargo build 会完整重编译。锁定文件不得通过终止用户播放器来强行删除。

## 9. 目录音乐库与主题接续

- 库、播放队列分开；移除目录 / 曲目不删除磁盘文件，不停止已开始播放。目录 / 原始路径身份留在 core，后台规范化路径、读标签和保存缓存；禁止 Slint 回调读整目录。
- 扫描 revision / 取消 generation 与媒体 generation 分开；过期扫描不能覆盖新配置，待提交请求合并最新项；ListView 虚拟化，库模型只按 revision 更新。失联目录缓存保留，手动重扫可恢复。
- 当前上限：20000 首 / 200000 项 / 64 层，配置 2MB、索引 32MB。没有实时目录监听 / 大库 DB；更改上限先验证内存、响应和退出。OS 对话框 / 正在进行的标签读取返回后才能结束相应服务。
- 主题遵循 Appearance → platform Observer → view model → Theme 语义令牌，设计 / 明暗 / 主题色 / 减少动态效果独立。不要向每个页面加入 OS 查询或固定绿色，新增设计集中扩展 profile / 选择项并保留旧配置默认。
- 时尚模式为轻量玻璃印象；不能为折射 / 模糊截图每帧视频，不删除 prepare_gl。动态效果立即响应操作，减少动画同时停掉持续 Timer；隐藏视频控件不改 FBO 尺寸。
- Windows 外观观察只读注册表 / DWM / 动画偏好；测试不得改系统主题来证明同步。Linux 使用只读 Settings portal，缺失接口时保持回退；macOS 系统外观尚未验证。
- Test-LibraryTheme 使用独立配置、自有 WAV 和真实 controller / GL presenter；截图不证明位准确或功耗。ui-feedback / audio-ui 的真实 Slint 点击与计时需继续回归。用户魅蓝 DSP 独占确认是用户实测，不能推断格式矩阵 / 数字捕获。

## 源码发布与共用主分支接续（2026-10-03）

- 用户已确定 GPL-3.0-only；workspace 与五个 crate 继承该字段，根 LICENSE 保留标准原文。第三方记录不能替代 runtime 二进制完整 notices / 对应源码安排。
- 正式集成 / 发布分支为 master，唯一顶层开发分支为 dev，所有修改遵循第 0 节；dev-linux 共用代码已在 `5390a3d` 合入，联网搜索与页面返回已在 `e99d67c` 中。dev-linux / dev-api 为历史功能分支；分支名不决定编译平台。原 main 提交保留在 master 历史中；当前分支状态按 git 检查，进度以 STATUS 和 docs/PUBLISHING.md 为准。
- 用户在自己的完整仓库根目录操作，Windows 工作区为 E:\SourceFiles\rust\yyplayer，Linux 工作区为 /home/yydaniel/sourcefiles/yyplayer；路径不决定平台。2026-10-03 的文档合并准备未执行提交 / 合并 / 推送；后续 Git 操作以当轮用户授权为准。发布源码不代表 USB / HDR / 广泛跨平台资格或二进制发行已完成。
- 用户于 2026-10-03 确认 Windows x64 packages 可以编译打包 dev-linux 的共用代码。记录为用户反馈的编译打包证据，不虚构 run URL / 提交号，也不扩展为播放、安装 / 卸载、WASAPI / GPU 或退出回归通过。修改共用逻辑时继续回归受影响的 Windows / Linux 路径，环境缺失记待验证。
- `.github/workflows/windows-release.yml` 在 master push 生成短期 Actions CI artifact（portable ZIP + NSIS x64 Setup）；没有自动创建 GitHub Release。NSIS 安装目标限定 x64 Windows。完整 libmpv build notices / corresponding-source arrangement 与 DLL 闭包完成后，才能把 CI 包作为正式二进制发行，不能以成功构建代替发行资格。

## 10. 窗口表面、缩略图与保存通知接续

- Windows Surface adapter 使用借用句柄和可选 DWM 提示；不以透明分层窗、window region 或 CPU 视频循环实现圆角。窗口化启用原生阴影，最大化 / 全屏关闭；Linux 圆角 / 阴影由 compositor 决定，不能照搬 DWM 资格；macOS 保留平台边界，未验收。原生截图与 Slint 截图区分，跨屏 DPI 不由角部截图证明。
- 音乐库可见行信息在 Slint 内更新，由 app tick 投递有界后台服务；不得在 repeater init 回调重新借用 controller。缩略图 64px / 256 项 LRU / 8 项队列与在途，缺失也缓存，重扫取消旧 generation，图片资源 revision 驱动投影。
- recent 历史保存必须静默，与真实设置 dirty 分离；成功提示需实际设置不同且保存成功。通知 revision 独立于持久化 revision，静默写入不得复活已手动关闭的浮窗。
- 歌词页通过左下 / 大封面返回音乐库，导入歌词位于播放选项。修改后复验 Test-UiCustomization 的封面点击 / 静默播放、ui-feedback 和导航；截图不代替 WASAPI / 位准确资格。

## 11. 视频库与按需渲染接续

- 音乐 / 视频库仅保留页主标题，数量并入筛选工具栏，不恢复宣传卡片、重复库标题和索引说明占位；界面名称统一“视频库”。修改列表高度后同步真实行点击 / 列宽拖动验收。

- 视频库导航进入独立视频库（page=5），播放器为同窗口 VideoPlayer（page=1）；不能把导航恢复成空播放器或令浏览库初始化 render context。
- video_library 配置 / settings.videos.json 缓存独立于音乐库；后台规范化 / 扫描 / 文件大小，索引不启动 decoder / 不虚构时长或封面。移除不删磁盘文件；20000 个媒体 / 200000 项 / 64 层、有界通道 / revision 保留。
- Load.video 意图与 snapshot.video 实际轨道分开；vid=no 音频 Load 不等待 render，视频 Load 等待租约 ready。新的音频 / Stop / 关闭视频必须取消旧等待请求；相关竞态由 VideoGate 测试覆盖。
- 返回库保存当前媒体 / 位置后 Stop；Engine 的 current-vo 已退出才确认释放条件，GL notifier 再清 UI 图片并 render_free / 删除 FBO / texture。不能只 Pause + vid=no，无音轨视频会变 Ended。继续观看重建并恢复位置，仅本次会话有效。
- EOF 重播不会重发 FILE_LOADED；快照需在真实 active / 非 EOF 时从 Ended 恢复 Playing / Paused，不在 UI 假设成功。播放回归覆盖普通暂停和 EOF 重播后再次暂停 / 继续，核对位置冻结、generation / renderer 不重建。
- 保留 prepare_gl / 同上下文 render / 先 render_free 后 core；Slint UI 仍用 OpenGL，音频 0 次 libmpv renderer 不等于 UI 不用 GPU。修改后回归 Test-VideoLibrary、Test-Video / Test-Render 真实 GPU 图片、导航 / ui-feedback / 音乐 UI。测试独立配置 / 自有素材；内存以 Release 同素材 / 同设置 / 同口径比较。

## 12. Ubuntu Linux 接续（2026-10-03）

- Linux 适配起于 dev-linux，当前共用源码已在 master；包含当前共用代码的分支在对应系统都应能使用相同 Cargo 命令构建，禁止用分支名 / 开发者绝对路径选择平台。平台 API / 依赖维持 target cfg，Linux apt / Python 打包工具不得变成 Windows 编译前置。
- 用户明确要求已创建 dev-linux，Ubuntu 26.04 amd64 本机开发预览和 deb / AppImage 适配以当前 STATUS、docs/LINUX.md、ADR 0007 为准；旧章节 Linux 未编译描述为历史边界。其他发行版、AMD / Intel / Xorg 真机、USB / HDR / 物理输入与多屏仍未资格化，不把 Wayland / Xvfb 结果泛化。
- Linux deb / 开发使用绝对系统 libmpv2 路径和发行版安全更新，策略在 third_party/mpv/linux.json；AppImage 包内 runtime 必须通过自身 manifest / SHA-256，损坏不得静默回退系统库。Windows 固定 hash 策略保持。
- Linux 不强制 ao 列表，它会覆盖 audio-device 中的具体后端。具体 PipeWire / Pulse / ALSA 设备必须核对实际 AO；ALSA hw 成功打开 / Final HW params 才确认，PipeWire exclusive stream 不等于硬件 DAC 独占 / 位准确。自动设备独占拒绝，失联暂停，ALSA hw 失败不猜测桌面 sink；源率保护比较源与 API。
- Linux 启动文件在首次 UI tick 前处理；保存的输出策略必须先于首次 Load，Engine 先获得真实设备列表。不可用请求保留 blocked、禁止 Load / Resume，audio_blocked 错误不能因 pending_load 的媒体身份过滤被隐藏。验收要求无默认 AO 初始化，不能只看位置 / Paused。
- 配置 / 索引 / EQ / 歌词 / 背景保留原始 Unix 字节，普通 UTF-8 旧字符串兼容。特殊 JSON 标记以路径不可能含有的 NUL 起始，不能改成会碰撞合法文件名的普通前缀。FILE_LOADED 比较复制的原始 C bytes；file URI 已解码，不再用显示字符串比身份。
- UiShell 在 event loop 返回后、hide 前清除 Slint 借用视频图片。RenderingTeardown 只释放 render / GL，不修改 UI 属性，否则 Wayland suspend 的 backend 可变 borrow 会重入；继续保持同上下文 / prepare_gl / 先 render_free 后 core。生命周期修改回归真实视频 10 循环、EOF 暂停、正常退出与 4K GPU 图片。
- 外观 portal 只读后台查询、字体后台枚举不变。Linux 圆角 / 阴影由 compositor 决定；Wayland 不提供客户端最小化状态 / 还原，例子不能写假确认。UI 截图可能延迟同次 timer 回调，验收等待真实状态并保留明确超时，不能删透明度断言。
- scripts/check-linux-deps.py 先只读汇总 apt；scripts/test-linux.py / test-linux-packages.py 用独立配置、自有素材，不停止系统音频服务或修改系统主题。硬件 / 竞争测试串行，防自己的不同验收互相占设备。库、字幕、背景、快捷键和音频仍复用同一 controller / Engine。
- scripts/package-linux.py 固定 appimagetool 归档和 runtime 源码 hash，每次从核验归档重新提取工具；两种包使用同一 Release 快照。源码包必须包含完整锁定图（含 Cargo 解析所需其他平台条件依赖），614 crate 原始归档 / notices 与空 Cargo home 的离线解析证据，不把解析称为再次完整编译。
- 当前 Ubuntu 26.04 packages 仅在 dev-linux push 自动运行，并支持手动选择 master 或其他包含工作流的分支。Linux compatibility compile matrix 仍仅手动运行。文档不得把未修改的 Linux push 触发器写成 master 自动运行。
- 新 Linux workflow 只生成短期 CI artifact，不代替用户推送 / 发布，也不代替安装 / 升级 / 卸载真机验收。包是开发预览：上游 AppImage runtime 的完整 Alpine 静态闭包 / 可重链接安排仍待补证，见 packaging/linux/runtime-notices/README.md；Windows DLL 闭包保留既有未完成状态。

## 13. Windows 自动编译打包接续（2026-10-12）

- 当前 dev 应用版本为 0.3.1，五个 workspace crate 继承根版本，保留更新后的 Cargo.lock；EXE 版本资源、NSIS 与包名从同一 Cargo 版本生成，不添加独立硬编码默认。master 仅在授权合入 dev 后接收版本更新。
- Package-Windows 在预检后自动执行 locked x64 MSVC Release build；使用本次 Cargo compiler-artifact.executable，检查实际退出码 / build-finished / 版本与 hash。不要恢复为读取固定旧 EXE，CI 不再另做重复编译。
- 自定义 Cargo 产物目录放 target 子目录或仓库外；.gitignore 不替代暂存 diff 检查，不强制加入运行时二进制、包、缓存或用户配置。临时 NSIS 工具仅在 target 内使用，无需修改全局设置。
- 修改打包执行 Test-WindowsPackaging，覆盖旧 EXE / 编译失败 / 自定义目录 / NSIS 失败 / 重试 / 已有包保护；兼容 PowerShell 5.1 / 7 原生 stderr。当前本机 ZIP / Setup 与独立启动证据见 docs/validation/windows-packaging.md，安装 / 卸载及正式发行资格仍待验。
