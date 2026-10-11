# PC 音乐联网搜索验收

2026-10-09 实现，2026-10-10 收尾；Windows x64 MSVC，Rust / Slint 1.17.1 / Winit + FemtoVG，唯一现有 libmpv Engine。Android 工程仅读取，未修改。源码工作区在 `dev-api`，本次功能改动未提交或推送。

## Git 基线

接手时 Rust 工作区干净，原 dev 的 `ec55789` 已在 origin/dev，不创建空提交。实际 `git fetch origin` 后，将 master 从 `1406840` 快进到 origin/master 的 `5390a3dcbdafc94ed806d0d11a6b3d5653044ebb`，再从该 master 创建 `dev-api`。master 与 origin/master 的 ahead/behind 均为 0；没有重置或覆盖用户文件。

## 自有素材与隔离

`scripts/Serve-OnlineFixtures.py` 生成 12 秒、44.1kHz、双声道、低幅正弦 WAV/FLAC、原创 LRC 和几何封面。HTTPS 仅监听 localhost:18443，临时测试 CA 与服务器私钥留在 target，不改系统信任。测试使用 `target/online-qa` 的独立设置与目录，不写用户媒体或 APPDATA。

| 文件 | SHA-256 |
| --- | --- |
| WAV | `0f6be3d71b2ef29d0916f5e8c68f3ffb54e046cb230cbd568f05d416d501dbd9` |
| FLAC | `0fcb111c7299dcc44c626d8c33dba58c1edf86fd8f51eb721e361344f74380c7` |
| Android / PC 共用 JSON | `418fe8d499d04b1cb3e5ff1f5da0630aacae29eb5aee9b2d4829c62fc4cfaf7e` |

固定 Windows libmpv `20260928-git-e470f8986e` 的 DLL SHA-256 为 `0d5b9dbecb73e179ef39dc94314ff8905fab926a80f8d60a2367d824b7254e2e`。QA 例子只开放指定 localhost 测试 CA；普通 Release 不启用 online-qa feature。

最终本地 `target/release/yyplayer.exe` SHA-256：`0f11efde52c254a74075607e1981d79f7cd1bbe9ac092ac240b220968e0a51d3`；这是开发预览产物，本轮没有生成安装包。

## 检查与实跑

| 命令 / 场景 | 结果 |
| --- | --- |
| `cargo fmt --all --check` | exit 0（最终检查） |
| `cargo clippy --locked --workspace --all-targets --offline -- -D warnings` | exit 0 |
| `cargo test --locked --workspace --all-targets --offline` | exit 0；core 12、mpv 9、UI 1、app 13通过；1个 HTTPS 测试按设计 ignored，随后显式运行；多个例子复用相同 controller 测试 |
| `scripts/Test-Online.ps1` 中 HTTPS 测试 | exit 0，最终39.06秒（完整脚本前轮38.80秒）；中文含引号/& 编码、GET、form/嵌套 JSON POST、候选排序、翻译合并、真实文件签名、重名、封面、Range代理、换authority不泄露头、拒绝 POST 重定向、错误音质、无效音频、取消与 partial 清理、限流 |
| `online-search` 实际 Slint/controller/libmpv | exit 0；HTTPS串流 source 为 Url，真实 position=1.020828秒、video=false；暂停与50%定位通过；999下载获得签名为 WAV 的成品及同名资源，加入音乐库，真实播放位置0.704263秒 |
| 关闭歌曲联网后的本地联网歌词 | 下载文件仍是 Local 且可播放；lyrics_enabled 独立开启，选择「手动候选版本」，关联文件持久化且存在 |
| `cargo test --locked --offline -p yyplayer-app --bin yyplayer` 最终代码 | exit 0，14通过/1个HTTPS按设计ignored；新增缓存配置/音质/变量隔离边界，随后显式HTTPS与真实UI重复通过 |
| 普通Release独立启动 | exit 0，固定内核ready=true/error为空，4秒后正常退出，使用独立target配置 |
| `navigation-ui` | exit 0；实际侧栏点击、空目录按钮、三种窗口尺寸、内容透明度与选项栏、简洁/减少动态模式通过 |
| 普通 `cargo build --locked --release --offline -p yyplayer-app --bin yyplayer` | exit 0，最终代码构建2分25秒，随后再次build确认无新增编译0.59秒；不启用 online-qa |

截图来自实际 Winit/FemtoVG 窗口，定时器只驱动测试动作，播放器状态来自真实 Engine；截图与数值不证明 DAC 位准确或输出资格。

本次保留的实际截图：[搜索结果](online-api/results.png)、[逐档音质检查](online-api/quality-checked.png)、[手动歌词](online-api/manual-lyrics.png)、[联网目录设置](online-api/settings.png)。原始运行数据留在 `target/online-qa/ui-results.json`，不把测试私钥/用户设置入库。

## 实现范围与待验

五类目录设置、Android v1 多份 JSON、纯歌词源筛选、封面、逐档质量检查、完整下载/串流缓存、独立开关、旧请求取消与资源版本隔离均已接入生产路径。坏 JSON 重读保留现有有效配置；没有 JSON 时回退编译内置的同一示例，存在其他 JSON 时忽略默认 example-gd.json。

没有对 GD 第三方曲目供给、用户账号、物理目录选择、Linux 新功能运行、安装/升级/卸载进行本轮验收。Windows/Linux 打包脚本只复制固定示例与指南，未生成新包；Linux 下载发布需要支持同目录 hard link 的文件系统。无后台下载服务/断点续传。既有 USB、HDR、多屏、物理输入及二进制完整发行许可限制仍保留。下一步用用户实际 API JSON 和授权曲目验收远端供给，并在 Linux 补同一 HTTPS/播放/目录及包回归。
