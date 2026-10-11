# 页面历史栈与返回验收（2026-10-11）

## 目标与实现

用户要求每个页面记住进入前的页面，返回按后进先出逐层恢复；完成后提交当前 `dev-api` 分支并清理本地编译缓存。本轮承接本分支已有的联网音乐与前端改动。

`AppController` 统一通过会话内 `navigation::History` 切换页面，覆盖侧栏、设置、音乐详情、本地 / 最近媒体打开、视频恢复、全屏与实际轨道确认。进入不同页面才压入原页面，返回弹出且不重新压栈；重复访问当前页和空栈返回不产生记录。七个页面都使用同一历史。

侧栏增加「返回上一页」，设置原固定返回改为同一动作，歌词大小封面和视频返回也接入。默认 Alt+Left 一次返回一层，按键重复不连续弹栈，已有自定义播放绑定优先保留；Esc 继续处理全屏和选项。离开全屏歌词页会恢复窗口模式。

返回联网搜索不执行侧栏切换歌曲搜索模式的重置，保留关键词、API / 来源、结果与歌词模式；离开时取消在途搜索，过期任务仍按原 revision 丢弃。视频离开继续提交 Stop 并释放 render 租约，返回视频页先弹栈，再通过原 worker Load 恢复保存位置，避免把设置重新压入历史。没有同步 UI mpv 调用或修改 presenter / runtime。

历史仅保留在当前进程内，重启从音乐库开始；不持久化页面历史或每次访问的滚动位置。

## 检查与证据

环境：Windows x64、Rust / Cargo 1.97.0、Slint 1.17.1、Winit / FemtoVG、固定 mpv `v0.41.0-1087-ge470f8986`。配置与媒体均在独立测试目录；导航布局例子使用生产 controller 的预览模式，未启动媒体引擎，预览曲目仅用于布局，不作为播放证据。

| 检查 | 结果 |
|---|---|
| `cargo fmt --all --check` | 通过 |
| `cargo clippy --locked --workspace --all-targets --offline -- -D warnings` | 通过，最终 2.60 秒 |
| `cargo test --locked --workspace --offline` | 最终通过，core12 / mpv9 / platform1 / UI1 / app20；HTTPS 默认 ignored，在完整 Test-Online 显式执行 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/Test-NavigationHistory.ps1` | 通过，24 步真实点击、controller 历史、透明度 / 宽度断言、三种窗口尺寸 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/Test-Online.ps1` | 完整运行通过，HTTPS 38.84 秒、真实流 / 下载播放、歌词 / 分页 / 目录 / 导航 |
| 同脚本 `-SkipHttps -SkipNavigation -FrontendOnly` | 补充有 20 条结果的设置返回后通过；此定向复跑跳过媒体，完整媒体场景已先运行 |
| `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/Test-VideoLibrary.ps1` | 通过，真实视频 10 次释放 / 重建，其中 5 次设置 → 返回视频页，位置恢复；保留 EOF / 暂停 / 取消 / 独立缓存回归 |
| `Test-UiCustomization.ps1 -SkipBuild`、`ui-feedback`、`audio-ui` | 全部通过；例子由最终 workspace test 构建；字体 / 字幕 / 列宽 / 原生窗口 / 封面与中心点击回归通过 |
| `cargo build --locked --release --offline -p yyplayer-app --bin yyplayer` | 通过，3 分 51 秒，不含 online-qa |
| 普通 Release 独立临时配置启动 / 4 秒正常退出 | exit0，ready=true、error / render_error 为空、Idle、render 初始化 0；没有修改用户 APPDATA |
| `pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/Test-Cleanup.ps1` | 通过，隔离实际清理与 WhatIf；debug/release 的 API config / EXE / runtime / 头文件 / 源码 hash 保留，生成文件清除 |
| `scripts/Clean-Workspace.ps1 -WhatIf` | 实际工作区预览通过，全部目标在当前 workspace 内；未删除文件 |
| `scripts/Clean-Workspace.ps1` | 提交后实际执行 exit0，移除 53,407,759,825 字节（49.74 GiB），保留文件 hash 均未变 |
| 清理后的普通 Release 再次独立启动 | exit0，ready=true、error / render_error 为空、Idle、render 初始化 0，EXE hash 未变 |
| 最终 diff / PowerShell / Python 语法与 Markdown 链接检查 | 通过 |

- 导航路径：音乐库 → 联网搜索 → 设置 → 最近打开 → 视频库，逐层返回最近打开 → 设置 → 联网搜索 → 音乐库。重复点击联网搜索后进入设置，只需一次返回就恢复搜索。由搜索展开歌词页，大小封面均返回搜索；根页面按钮禁用。1240×900、1000×640、1460×920 的实际逻辑坐标点击及页面动画断言通过。数据见 [24 步记录](navigation-history/ui-results.json)。
- 搜索条件：最终有结果的返回实测为 `page=6`、关键词「失败分页验收」、provider=0、source=0、20 条原结果逐项一致。截图见 [返回搜索](navigation-history/returned-search.png)，数据见 [前端返回记录](navigation-history/online-return-results.json)。完整 HTTPS / 串流 / 本地下载记录单独保留在 [完整回归](navigation-history/full-online-results.json)；完整测试末尾第一次返回检查发生在 API 目录重读之后，结果已清空，故补充前述有结果场景。
- 视频：独立 24 秒 1280×720 SMPTE bars 无音轨素材，SHA-256 `d7e0e0f9964e1d5425418b69395d5b92a944858313cbfa8f8e5fa9949178dae6`。设置返回与视频库恢复交替完成 10 循环；后台 Stop 后实际 Idle / video output 关闭，恢复需实际 Playing / renderer ready / 帧数增加且位置不从零开始。见 [运行数据](navigation-history/video-results.json)、[离开](navigation-history/video-returned.png)和[恢复](navigation-history/video-recreated.png)。此截图不替代 GL / HDR / 硬解画质资格。

导航例子首次点大封面的坐标沿用了有歌词双栏布局，无歌词时封面位于页面中央，导致点击未命中；改用实际居中位置后通过。验收脚本使用 `Start-Process -Wait -PassThru` 取得 Windows PowerShell 的实际退出码，未把空退出码计作通过。

音频布局例子最初缺少自有封面 fixture，复制本轮生成的方形素材后通过；该例子仅验证布局 / 点击，不扩展为真实音频设备资格。

固定 libmpv DLL SHA-256：`0d5b9dbecb73e179ef39dc94314ff8905fab926a80f8d60a2367d824b7254e2e`。最终 Release EXE 为 27,524,096 字节，SHA-256：`613062b307f1351052d4bd6102c482c3cae76a8ac1b8371b71138ee7bdd0bded`。独立启动数据见 [Release 诊断](navigation-history/release-smoke.json)。未打新安装包或发布 Release。

## 提交与缓存清理

已按用户授权将本分支联网功能、前端与导航历史一起提交当前 `dev-api`，提交标题为「完成联网音乐搜索、前端优化与页面历史栈」。先通过 `scripts/Clean-Workspace.ps1 -WhatIf`，提交后执行实际清理，exit0，移除 **53,407,759,825 字节（49.74 GiB）**。清理与再次启动证据补录到本轮本地提交，未推送。

源码、Git、文档、固定 DLL / 四个 mpv 头文件及 `target/release/yyplayer.exe` 保留。可执行文件旁 `config/cloud_api` 属于 API 数据，新版脚本保留 debug/release 的 config 目录；真实 release/config 为空并已保留，真实 debug/config 不存在。target 递归文件清单仅剩最终 EXE，没有编译中间产物。未清理全局 Cargo registry 或用户 APPDATA / 媒体目录，未终止用户播放器。

清理后再次用临时独立配置启动同一 EXE，4 秒正常退出，固定 runtime 就绪、error / render_error 为空，EXE / DLL / 头文件 hash 与清理前逐项一致。结果见 [清理结果](navigation-history/cleanup-results.json)、[保留文件 hash](navigation-history/cleanup-preserved-files.json)、[清理后启动](navigation-history/release-after-cleanup.json)。隔离保护结果见 [清理测试](navigation-history/cleanup-test-results.json)。

## 限制与下一步

本轮实测为 Windows；Linux / macOS 运行、物理鼠标 / 触控板 / 键盘与跨屏 DPI 未新增资格。USB、位准确、HDR 和二进制正式发行条件沿用原限制。下一步在 Linux 复验同一返回路线与视频资源恢复，并补物理输入场景；代码编译不能替代这些实机证据。
