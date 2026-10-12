# Linux 最低编译版本测试

入口为 [linux-compatibility.yml](../.github/workflows/linux-compatibility.yml)，只配置 `workflow_dispatch`，不会因 push / PR 自动运行。它在每个发行版自己的 amd64 用户空间中完整编译当前应用，然后汇总各系统结果。矩阵定义见 [matrix.json](../scripts/linux-compat/matrix.json)。

## 手动运行

1. 将工作流和 `scripts/linux-compat/` 配套文件提交、推送到 GitHub 默认分支（本项目为 `master`），使手动工作流入口注册。待测试分支也必须包含这些文件及 Linux 适配代码。只放在非默认分支时，GitHub 页面可能没有 Run workflow 按钮，详见 [GitHub 手动运行说明](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)。
2. 打开仓库 **Actions → Linux compatibility compile matrix → Run workflow**，选择 `master` 或包含共用代码及工作流的功能分支。
3. 保留默认参数运行，等待 `report` job 完成；单个发行版编译失败不会取消其他发行版。
4. 在该次运行 Summary 查看中文报告，或下载 `linux-compatibility-report-<attempt>`，其中包含 `report.md` 和 `report.json`。逐系统日志位于 `linux-compat-case-<attempt>-<系统>`。

工作流已有本地验证证据；当前尚未收到真实远端矩阵报告，最低源码编译版本仍待实测。Windows 共用代码的编译打包已由用户确认，不替代此 Linux 矩阵的结果。无需为这项远端测试在本机安装 apt 库；GitHub 的宿主 runner 提供 Docker，依赖安装只发生在临时容器内。

| 参数 | 默认值 | 用途 |
| --- | --- | --- |
| `rust_version` | `1.99.0` | 所有 case 使用同一个精确 Rust 版本；必须为 `x.y.z`，且满足项目的最低工具链 `1.92.0` |
| `include_testing` | `false` | 加测 Debian 14 / forky testing，单列结果，不参与正式版最低结论 |
| `keep_binaries` | `false` | 将成功编译的 `yyplayer` 保存在各系统 artifact；默认只保留日志、版本和 ELF 证据 |

默认最多 3 个 case 并行。每个容器构建限时 90 分钟，job 留出镜像、日志和上传时间，限时 105 分钟。取消整个运行可能来不及生成报告。重测建议选择 **Re-run all jobs** 或重新 Run workflow；仅重跑失败 job 时，新 attempt 可能缺少未重跑的 case，报告会列为缺失，不混入旧 attempt 的结果。Artifact 保留 30 天。

预期的编译失败作为实验结果保存，不让构建步骤因这个结果中断；因此 workflow 显示绿色仅表示测试流程完成，各系统是否编译通过以报告为准。

## 覆盖范围

| 系列 | 默认版本 | 镜像 |
| --- | --- | --- |
| Ubuntu LTS | 18.04、20.04、22.04、24.04、26.04 | 对应版本的官方 `ubuntu` 镜像 |
| Debian 正式版 | 9 stretch、10 buster、11 bullseye、12 bookworm、13 trixie | 对应代号的官方 `debian:<codename>-slim` |
| Debian testing，可选 | 14 forky | `debian:forky-slim` |

Debian 9 是 Ubuntu 18.04 发布时的 Debian stable，因此作为起点；包含此后到当前的每一代正式 Debian。发行信息见 [Debian releases](https://www.debian.org/releases/)。矩阵不包含 Ubuntu 非 LTS、Debian unstable 或其他架构。

Debian 9 / 10 的旧仓库改用 [Debian 官方归档](https://www.debian.org/distrib/archive)，仅跳过已过期的 Release `Valid-Until` 日期检查，保留签名验证。若归档不可用或签名检查失败，结果为环境失败，不能当成代码不兼容。所有镜像 tag 和 apt 仓库均可能变化；每次保留实际镜像 digest、源配置和安装包版本。

## 实际编译方式

Checkout、artifact 上传 / 下载在现代 `ubuntu-24.04` 宿主执行；宿主用 Docker 运行发行版容器。这避免较新的 Node Actions 在旧 glibc 系统内无法启动，仍让 Rust、C 编译器、链接器和开发库来自所选发行版。

每个 case 使用只读源码、相同 Cargo.lock、全新 Rust / Cargo / target 目录，无跨系统二进制缓存。容器安装最小原生编译依赖：`build-essential`、`pkg-config`、`ca-certificates`、`curl`、`binutils`、fontconfig / FreeType 开发包；自动兼容旧新包名。先下载锁定依赖，再离线完整编译：

```bash
cargo fetch --locked --target x86_64-unknown-linux-gnu
cargo build --locked --offline --release -p yyplayer-app --bin yyplayer
```

这会生成实际 Release ELF 可执行文件。libmpv 是运行时动态加载依赖，编译不要求安装 libmpv 0.41；否则旧系统会在到达编译器之前被播放器运行时版本人为拦住。各系统可用 mpv 包版本另行记录，供之后判断运行环境。

成功后记录 ELF 动态符号、版本、`ldd`、二进制 SHA-256 和所需最高强引用 GLIBC 版本。弱符号不提高该统计门槛；该值只覆盖主程序，不代表完整 libmpv、图形驱动和所有动态加载依赖的最低版本。

## 怎样理解报告

报告分别计算 Ubuntu LTS、Debian 正式版的最早编译通过版本，不把两个系列合并成一个年号。testing 不参与正式版结论。

| 结果 | 含义 | 能否证明更早版本编译失败 |
| --- | --- | --- |
| `passed` | 完整 Release 构建、ELF 检查和容器完成证据通过 | 此版本编译成功 |
| `build_failed` | 实际进入编译阶段，编译或链接失败 | 可以，限本次源码 / 依赖 / 工具链组合 |
| `environment_failed` / `toolchain_failed` / `fetch_failed` / `image_failed` | apt、工具链、网络依赖或镜像准备失败 | 不能 |
| `resource_failed` / `timeout` | 内存、空间耗尽或执行超时 | 不能 |
| `inspection_failed` / `incomplete` / `missing` / `invalid` | 检查未完成、artifact 缺失，或证据与本次运行不符 | 不能 |

只有最早成功之前的所有矩阵版本均有实际编译失败证据，报告才将 `minimum_demonstrated_within_matrix` 设为 true；否则保留最早观测成功版本，同时列出 `earlier_unresolved`，说明更早下限尚未确定。如果没有任何成功，报告明确写“最低版本未确定”。后续版本仍可能失败，不能从一个成功结果推断其他版本都通过。

每个 artifact 的 `result.json` 包含提交、工具链、Cargo.lock hash、run / attempt、镜像 digest、阶段、耗时和结果。汇总拒绝重复、过期或不完整的成功 / 失败证据。完整错误上下文在 `build.log`；报告不将网络失败伪装为不支持旧系统。

## 边界与后续

这是源码编译实验。容器共用现代宿主内核，没有运行 UI、音视频、GPU、音频设备或 deb / AppImage 安装测试。即使 Ubuntu 18.04 编译通过，也不能直接宣称本轮在 Ubuntu 26.04 生成的 deb / AppImage 可用于 18.04；不同构建环境的链接要求不同。

收到真实报告后，选择最早成功的候选基线，另做该系统上的运行时闭包、真实播放、GPU 和安装验证，再决定发布基线。当前 README 中已生成包的最低条件仍保留；首次 CI 报告出来后才能补充实测源码编译版本。

本地报告边界测试与工作流检查记录见 [验证记录](validation/linux-compatibility-ci.md)。
