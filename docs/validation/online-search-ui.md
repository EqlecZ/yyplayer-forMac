# 联网搜索前端调整验收

2026-10-11，Windows x64 MSVC，Slint / slint-build 固定 1.17.1，Winit + FemtoVG。沿用 dev-api 当前未提交工作区，未提交 / 推送。上轮联网功能证据见 [online-api](online-api.md)。

## 范围

- 页标题右侧搜索框，保留 Enter / 按钮提交、禁用状态、歌词候选与历史切换；取消按钮位于请求状态行。窄窗口 / 打开播放选项时压缩标题与选项文字，打开文件仍可通过侧栏进入。
- 虚拟化 ListView 到底后延迟提交原有有界搜索任务。追加结果保留滚动位置，忙碌 / 关闭服务 / 历史 / 音质弹层时不自动翻页；末页与重复整页停止。失败保留已有结果，可重新搜索。
- 设置 → 联网服务 → API JSON 增加「打开文件夹」，使用当前有效原始路径，包含默认 exe 旁目录；目录创建 / 打开在后台。失败显示原因，未改用户默认配置 / 系统偏好。
- Windows 文件夹打开使用原生 ShellExecuteW；Linux / macOS 分别提供 xdg-open / open 平台边界，本轮只有 Windows 运行证据。API 依据锁定 windows-sys 0.61.2、本地 Slint 1.17.1 widget 源码及 [Microsoft 文档](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutew)核对。

## 隔离与素材

`scripts/Test-Online.ps1` 使用 `target/online-qa` 独立配置、自有 HTTPS / WAV / FLAC / LRC / 封面与临时 localhost CA，未安装系统证书。新增分页关键词的接口按 20 + 20 + 5 返回自有记录，另提供重复整页 / 503 失败页。普通搜索的既有 HTTPS 契约保持。

`-SkipHttps` / `-SkipNavigation` / `-FrontendOnly` 只用于局部诊断，脚本输出注明跳过项；完整验收不传这些参数。截图证明实际 Slint 布局 / 交互，不代替物理输入、音频 / HDR 或跨平台资格。

## 检查与结果

| 命令 / 场景 | 结果 |
| --- | --- |
| `cargo clippy --locked --workspace --all-targets --offline -- -D warnings` | exit 0，最终代码，36.08 秒 |
| `cargo fmt --all --check` | exit 0，最终代码 |
| `cargo test --locked --workspace --offline` | exit 0；core 12、mpv 9、platform 1、UI 1、app 14；HTTPS 单测默认 ignored，已在下述完整脚本显式实跑 |
| `scripts/Test-Online.ps1`（不传跳过参数） | exit 0；HTTPS 契约 38.70 秒；真实 Slint/controller/libmpv 与导航全部通过 |
| 页标题右侧输入 / Enter / 搜索按钮 | 实际窗口事件提交并得到自有 HTTPS 结果；歌曲与歌词候选沿用相同输入框 |
| 自动分页 / 追加位置 / 末页 | 20 → 40 → 45；追加后 visible-first=12，未回到顶部；末页 more=false。请求页依次 1 / 2 / 3，无重复请求 |
| 不满一屏自动补充 | 1240×2000 逻辑窗口；未发送滚轮事件，自动加载至 40 条，visible-first=0；请求页 1 / 2 |
| 重复整页 / 失败页 | 重复第二页保留 20 条并停止；503 第二页保留 20 条、显示「服务返回HTTP 503」、停止。各场景仅请求 1 / 2 页 |
| API 「打开文件夹」 | 实际设置按钮点击；自选 QA 目录与恢复默认后的 exe 旁目录均获得 Windows 原生打开成功回复，并核对默认目录存在与当前显示路径一致 |
| 窗口尺寸 / 播放选项 | 1000×640、1240×900、1460×920 实际截图；最低窗口展开选项时标题简写、筛选动作移到下一行，人工看图无重叠 |
| 原有联网播放 / 下载 / 歌词 | 实际串流 position=1.020775 秒、video=false；下载本地文件 position=0.704930 秒，暂停 / seek / 本地歌词持久化 / 关闭歌曲联网后的本地播放仍通过 |
| `cargo run --locked --offline -p yyplayer-app --example ui-feedback` | exit 0；浮空提示、全屏 / 窗口控件计时与悬停、顶部无空白、三尺寸居中播放点击通过 |
| `cargo build --locked --release --offline -p yyplayer-app --bin yyplayer` | exit 0，4分08秒；普通 Release，不启用 online-qa |
| 普通 Release 独立配置启动 / 4 秒正常退出 | exit 0；ready=true、error为空、phase=Idle、runtime=mpv v0.41.0-1087-ge470f8986，未加载媒体；正常清理退出 |
| 最终 diff / Python / PowerShell 脚本检查 | 通过 |

最终可执行文件 `target/release/yyplayer.exe` SHA-256：`da7e438aa0eddfd4a254076da9037336617728ddb89719e90cde9876f9ee5da1`。本轮未打安装包或发布 GitHub Release。

固定 libmpv DLL SHA-256：`0d5b9dbecb73e179ef39dc94314ff8905fab926a80f8d60a2367d824b7254e2e`；原创 WAV SHA-256：`0f6be3d71b2ef29d0916f5e8c68f3ffb54e046cb230cbd568f05d416d501dbd9`，与上一轮自有素材相同。

验收滚轮事件跨多个 event loop 回合投递：虚拟列表大幅滚动后可能修正估算范围，不能假设单次巨大 delta 已到最终底部。最终判定依然使用实际 viewport-y / viewport-height / visible-height；加载延迟为 150ms，条件不满足则停止。

保存的证据：[搜索栏](online-search-ui/results.png)、[追加第二页](online-search-ui/pagination-second.png)、[API 目录按钮](online-search-ui/settings-open-folder.png)、[最小窗口展开选项](online-search-ui/header-options-minimum.png)、[真实运行数据](online-search-ui/ui-results.json)。以上只包含自有素材和独立测试目录。

Release 诊断见 [独立启动记录](online-search-ui/release-smoke.json)。启动检查的第一次环境隔离把已清除变量变成空字符串，触发开发路径覆盖并正确报告 core 未就绪；未计为通过。改用真正移除进程环境变量后，固定运行时启动与正常退出通过；生产 loader 的校验未改变。

## 限制与下一步

Linux / macOS 的新文件夹按钮与本轮前端运行、第三方 API 分页供给、只读安装目录与特殊文件系统、不同 DPI / 物理触控板尚未验收。下一步在 Linux 用相同独立 HTTPS / UI 场景复验，并用实际 API JSON 检查分页协议与权限失败提示。
