# master / dev 两端同步（2026-10-12）

用户要求合并两端 master 的最新代码，再让本地 / 远端 dev 同步。工作区开始时干净，当前 master；origin 为 `https://github.com/YYDanielJr/yyplayer.git`。未触碰另一个 `.kilo/worktrees/functional-pie` 的 detached checkout，未清理缓存或用户设置。

## 同步前状态

获取 origin 最新状态后：

| 分支 | 提交 | 内容 |
| --- | --- | --- |
| 本地 master | `ee3c701c3707c31ec85a8e984d00f0ccbb920f17` | Windows 自动编译打包、0.3.0、文档 / gitignore |
| 远端 master | `7a0bac882bfc7bd21fb1896a0a742dad2b977bb3` | 合入 PR #5，包含最新歌词主题与 UI |
| 本地 dev | `f9a0219ec4c7ad25c89049dfd8b0c23c5ace8b2f` | 最新歌词主题与 UI |
| 远端 dev | `f9a0219ec4c7ad25c89049dfd8b0c23c5ace8b2f` | 与本地 dev 相同 |

master 双方独有提交数为 1 / 2，dev 双方为 0 / 0。两端 dev tip 都是远端 master 的祖先，故其改动已包含在本次 master 合并中。

四个起始 tip 保留在本地恢复引用 `refs/backup/codex-sync-20261012-187b98fa/{master-local,master-remote,dev-local,dev-remote}`，可用 `git show-ref` 查询。详细起始快照保存在生成文件 `target/branch-sync-before.json`，不提交。

## 合并与验证

在本地 master 执行普通 merge origin/master，保留原始提交。仅 README、CHANGELOG 和 STATUS 内容冲突；README 合并等价主分支说明并保留 0.3.0 与新歌词描述，后两份文档同时保留两套更新与验收记录。没有手工改写应用源码。

核对合并结果相对 origin/master 的 crates / scripts / workflow / Cargo 差异，仅为本地已有的 Windows 打包脚本、对应回归、workflow、应用 build.rs 与 Cargo 版本 / lock；其余远端功能代码完整保留。相对原本地 master 的打包 / 版本 / ignore 文件没有变化。PUBLISHING 更新本轮同步授权。

实际验证：

| 检查 | 结果 |
| --- | --- |
| `cargo fmt --all --check` | exit0 |
| `cargo clippy --locked --offline --workspace --all-targets -- -D warnings` | exit0，40.80秒 |
| `cargo test --locked --offline --workspace --all-targets` | exit0；core14/mpv9/platform1/UI1/app22，共47项独立单测；17个测试目标中的例子复用测试也通过，既有 HTTPS 默认 ignored |
| `cargo build --locked --offline --release --target x86_64-pc-windows-msvc -p yyplayer-app --bin yyplayer` | exit0，4分10秒，EXE版本0.3.0 |
| 合并后 Release 独立配置启动 | 4秒正常退出exit0，ready=true、Idle、error/render_error空、render初始化0 |
| `Test-WindowsPackaging.ps1 -NsisCompiler <便携工具>` | 7项通过 |
| Markdown 链接 / PowerShell AST / diff / source保留断言 | 通过；源码冲突0项 |

本轮 Release EXE 为28,974,080字节，SHA-256：`f641925b7e881daa02518c975bb6c28640e55283d25e53496e99e2d81af2d144`。日志保留在 `target/branch-sync-{fmt,clippy,tests,release,packaging}.log`，启动数据在 `target/branch-sync-smoke/`；生成物不提交。原 dist 包仍是打包验收时的旧基准快照，未改文件名冒充合并后包。

## 分支更新

合并与验证完成后，将本地 dev 快进到 master，使用普通 atomic push 同时更新远端 master / dev。远端若在此期间前进，正常推送拒绝会保护它，再获取并合并新增提交；不使用 force push。本记录随合并提交同步；交付前重新获取远端，并用 ls-remote、四指针相等与四个起始 tip 的祖先断言确认同步，最终提交号见本轮交付及 `target/branch-sync-after.json`。

本轮只处理分支 / 文档冲突与合并验证，未重复实际歌词 GPU / 联网专项，原始验收报告完整保留；不把编译或启动结果扩展为 Linux / macOS、物理输入、USB / 位准确、HDR、安装 / 卸载或正式发行资格。提交后的 Windows push 会触发既有打包工作流；远端 CI 结果需另行验收。
