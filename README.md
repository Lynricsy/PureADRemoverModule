# PureADRemoverModule

[![Release](https://github.com/Lynricsy/PureADRemoverModule/actions/workflows/release.yml/badge.svg)](https://github.com/Lynricsy/PureADRemoverModule/actions/workflows/release.yml)
![GitHub release](https://img.shields.io/github/v/release/Lynricsy/PureADRemoverModule?display_name=tag&style=flat-square)
![GitHub downloads](https://img.shields.io/github/downloads/Lynricsy/PureADRemoverModule/total?style=flat-square)
![GitHub repo size](https://img.shields.io/github/repo-size/Lynricsy/PureADRemoverModule?style=flat-square)
![GitHub code size](https://img.shields.io/github/languages/code-size/Lynricsy/PureADRemoverModule?style=flat-square)
![GitHub last commit](https://img.shields.io/github/last-commit/Lynricsy/PureADRemoverModule?style=flat-square)

PureADRemoverModule 是一个 Android Root 环境下的非域名本地层去广告模块。它面向 Clash、代理和 DNS 无法覆盖的本地广告落地物，例如应用私有目录中的广告缓存、广告 SDK 文件、已知广告 SQLite 数据库，以及显式 profile 下的组件、AppOps 和 ROM 配置治理。

本项目不是 DNS、hosts、Clash、Mihomo、Box、AdGuardHome 或任何代理方案的替代品。域名、网络连接和代理路由应继续由用户已有网络层工具处理；本仓库只做可审计、可回滚的本地文件和系统状态治理。

## 用途与分工

PureADRemoverModule 只补齐网络层工具覆盖不到的 Android 本地层场景：清理或占位应用私有广告缓存、处理广告 SDK SQLite 数据、执行明确 profile 下的 AppOps/component/ROM 广告项治理，并通过 ledger 记录可恢复动作。

Clash、Mihomo、Box、DNS、代理路由和域名规则属于网络层职责；本项目不生成网络规则、不改代理配置，也不接管解析或转发链路。推荐分工是：网络层继续处理域名和连接路由，PureAD 只处理已经落到本机文件、数据库或系统状态里的广告痕迹。

## 安装与打包

模块模板位于 [`module/`](module/)，其中包含 Magisk、KernelSU、APatch 通用的 `module.prop`、`customize.sh`、`service.sh`、`action.sh`、`uninstall.sh` 和模块辅助脚本。安装用 zip 应从 GitHub Release 下载；Release 产物由 tag workflow 使用 Android NDK 构建真实 Android ABI 二进制。本地直接运行 [`scripts/package-module.sh`](scripts/package-module.sh) 且未设置 `PUREAD_ANDROID_ABIS` 时，只生成当前宿主机 fixture 包，用于结构校验，不作为 Android 安装包。

安装后不需要额外手动操作。Android 启动 `service.sh` 时默认会先执行一次 bundled 本地治理 profile：`conservative sdk_cache sqlite`，随后以 apply 模式启动 `puread-daemon`，用 inotify 继续处理后续新落地的文件/SDK 缓存。每个模块版本在全部自动 profile 成功后只会写入一次 `state/auto-apply-<version>.done`，避免每次开机重复执行同一批本地治理。AppOps、component 和 ROM profile 仍可通过显式 profile 手工执行；默认不自动运行它们，避免设备、ROM 或应用差异把可选系统状态治理误报成 Root 管理器里的 profile errors。

模块会把当前运行态同步到 `module.prop` 的 `description=` 字段，因此 Root 管理器的模块列表可以直接看到带 emoji 的短状态，例如 `🔵 installed · reboot to activate`、`🟢 active · daemon running · profiles 6/6 · pid 1234`、`🟠 profile errors · daemon disabled · profiles 5/6`、`🔴 missing native binary` 或 `🔴 uninstall needs attention`。这些运行统计只在已有生命周期写状态时读取现有 `auto-apply-summary.log`、ledger 和 pid 文件，不新增轮询、定时器或常驻任务。不同管理器可能会缓存模块列表；安装、重启或重新打开管理器后通常会刷新显示。

可选调试开关：

- `PUREAD_AUTO_APPLY=0`：跳过安装/开机自动 profile 执行。
- `PUREAD_AUTO_APPLY_FORCE=1`：强制当前版本重新执行自动 profile。
- `PUREAD_AUTO_PROFILES="conservative sdk_cache sqlite"`：覆盖自动执行的 profile 列表。AppOps、component、ROM 这类系统状态 profile 需要明确加入后才会自动跑。
- `PUREAD_DRY_RUN=1`：daemon 只输出计划，不执行后续文件规则动作。
- `PUREAD_DAEMON_DISABLE=1`：执行自动 profile 后不启动 daemon。

不要把当前本地构建、fixture 测试或脚本静态校验理解为实机通过。真实安装仍需要在目标 Root 管理器、目标 ABI、目标 ROM 和目标应用版本上单独验证；README 只说明仓库入口和使用边界，不声明已经覆盖所有设备组合。

## 发布

仓库包含 GitHub Actions release workflow。推送 `v*` tag 时会自动安装 Rust 与 Android NDK，构建 release profile 的 Android ABI 模块包，并把 `dist/*.zip` 与对应 SHA256 上传到 GitHub Release。

```sh
git tag v0.1.0-t31
git push origin v0.1.0-t31
```

Release workflow 只做自动构建、结构校验和发布，不做真实 Android 设备安装验证；实机验证仍需在目标 Root 环境中单独执行。

## 当前能力

- `puread-core` 提供类型化规则模型、路径边界模型和恢复账本。
- `puread-rules` 解析 TOML 规则，并拒绝 hosts、DNS、domain、proxy、iptables、ad_reward、IFW 清空和 Root 隐藏等禁止类别。
- `puread-android` 执行可回滚的文件动作、SQLite 动作、可选 `chattr` 封装和可注入 Android 命令适配层。命令适配层覆盖 `pm`、`cmd appops`、`settings`、`getprop`、`chcon`、`chattr`、`lsattr`，支持 fake runner 与 dry-run，并拒绝 DNS、hosts、proxy、private DNS 相关 settings。
- `puread-android` 支持显式 profile 下的可逆 AppOps、component 和 ROM profile。AppOps 通过 `cmd appops get/set` 记录原 mode 或 default 后再设置目标 mode；component 记录原 enabled/hidden 状态并通过 `pm disable-user` / `pm enable` 恢复；`pm hide` 是 runner-backed capability attempt，不可用或失败时跳过并写入 skipped 记录，成功时写入 durable confirmed record，恢复时才执行 `pm unhide`。可选应用包不存在、`pm path` 无安装路径、MIUI shared_prefs 文件或目标键不存在时会记录 skipped，不计入 profile failed。ROM profile 仅限定广告相关 `settings` 与 `shared_prefs` XML 布尔项，执行前检测 ROM，记录原值、原文件哈希和备份路径；不包含 DNS、private DNS、proxy 或 network 修改。
- `puread-daemon` 提供事件驱动的文件规则守护能力、低频调度策略和文件规则 dry-run/apply 集成；在模块生命周期中默认由 `service.sh` 以 apply 模式启动。
- `puread-cli` 支持 `status`、`scan`、`apply-profile`、`profile-report`、`profile-restore`、`restore`、`dump-report` 和 `rules validate`。手工执行 CLI 时，`scan` 与 `apply-profile` 默认 dry-run，真实执行必须显式传入 `--execute`；模块 `service.sh` 会在 Android 启动时显式调用 `apply-profile --execute`。修改动作会获取模块全局锁，profile 执行会写入 `profile-actions.jsonl` JSONL ledger，用于 `profile-report` 查看和 `profile-restore` 恢复。普通文件恢复继续走 `restore --ledger ...` 和文件恢复账本。
- `module/` 提供 Magisk、KernelSU、APatch 通用模块模板和生命周期脚本。宿主 dry-run 不写入模块运行态目录；Android 实机启动会自动应用 bundled profiles 并启动 daemon。

## 配置与默认执行策略

模块生命周期默认是真实执行：安装后首次 Android 启动会自动应用 `conservative sdk_cache sqlite` bundled profiles，并启动 apply 模式 daemon。手工 CLI 仍默认 dry-run，方便调试、审计和恢复前检查；真实执行必须显式传入 `--execute`。所有真实修改必须写入恢复账本，失败时保留可追踪状态。

自动 profile 默认只执行仓库内已审查的本地文件、SDK 缓存和 SQLite 规则，不包含 hosts、DNS、代理、iptables 或域名类能力。新增高风险规则仍必须放在明确 profile 下，具备来源、恢复策略和账本记录；是否加入自动 profile 列表必须单独审查。

强力模式的风险来自系统状态变更，不来自网络层改写：

- AppOps 会读取并记录原 mode 或 default，再设置目标 mode；恢复时依赖 profile ledger。
- component 会记录原 enabled/hidden 状态，再按 profile 执行禁用或恢复。
- `pm hide` 只是 capability attempt：设备、ROM 或权限不支持时记录 skipped 并跳过；只有实际成功才写入 durable confirmed record，恢复时才尝试 `pm unhide`。
- ROM profile 只处理明确广告相关的 settings 或 `shared_prefs` XML 布尔项，执行前记录原值、原文件哈希和备份路径。

## 低功耗行为

`puread-daemon` 面向低功耗运行：文件规则由 inotify 事件驱动触发，维护任务使用低频调度，不做固定 5 秒轮询，也不默认申请 wake lock。SQLite 不进入高频 watcher，只在安装/开机的一次性自动 profile 或手工 profile 命令中执行；AppOps、component 和 ROM profile 仅由显式手工 profile 或用户覆盖后的 `PUREAD_AUTO_PROFILES` 触发。若当前设备没有可监控的目标目录，daemon 会记录 `no_watch_roots=true` 后退出，不会空转。

## 恢复与卸载

恢复依赖 ledger，而不是猜测当前设备状态。普通文件动作使用 `restore --ledger ...` 查看或执行恢复计划；profile 动作写入 `profile-actions.jsonl`，可通过 `profile-report` 查看历史动作，再用 `profile-restore` 按记录恢复。

模块卸载脚本会尝试停止本模块 daemon，并把普通文件 ledger 与 profile ledger 的 restore dry-run 计划写入日志和状态文件；真实恢复建议在移除模块前由用户显式执行 CLI 恢复命令。这样可以避免卸载阶段缺少二进制、权限或运行态目录时误判恢复成功。

## 安全边界

生产路径不得实现以下能力：

- hosts 生成、合并、系统映射改写或动态切换。
- DNS 接管、私人 DNS 强制关闭、解析劫持或解析服务托管。
- Clash、Mihomo、Box、ProxyConfig 或其他代理配置改写。
- iptables 域名、字符串、IP、TLS/SNI 阻断。
- 广告奖励域名切换、IFW 清空、Root 隐藏或反检测规避。

## 上游同步

上游快照和规则更新流程以 [AGENTS.md](AGENTS.md) 为准。`Example/` 是 `.gitignore` 忽略的只读上游参考快照，不进入提交；未来同步只能从快照中提取非域名规则候选，并必须把禁止能力分类为 rejected。

推荐入口：

```sh
scripts/update-upstream.sh --from-local Example --report-only
```

兼容旧入口：

```sh
scripts/update-upstream.sh --dry-run
```

同步报告由 `xtask/upstream-report` Rust 工具生成，必须记录来源、commit 或 SHA256、允许人工审查候选、拒绝类别和验证结果。所有候选都必须保持 `review_state="manual_review_only"` 且 `auto_import_allowed=false`，不得自动改写 `rules/`。

## 本地验证

常用质量门：

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
./scripts/verify-local.sh
```

Rust LSP 诊断依赖本机 `rust-analyzer`。如果当前 toolchain 未安装 `rust-analyzer`，以 Cargo 格式化、Clippy 和测试结果作为本地 Rust 验证证据。

Wave4 T20/T23/T24 常用 targeted commands：

```sh
cargo test -p puread-cli cli_profile_restore -- --nocapture
cargo test -p puread-cli cli_profiles -- --nocapture
cargo test -p puread-cli cli_execute_security -- --nocapture
cargo test -p puread-android appops_components -- --nocapture
cargo test -p puread-android rom_profiles -- --nocapture
cargo run -p puread-cli -- rules validate rules/common rules/apps rules/sqlite rules/appops rules/components rules/rom
```

当前这些验证覆盖 fake runner、fixtures 和 Cargo tests：CLI profile dry-run/`--execute`、全局锁、profile JSONL ledger、profile restore/report、AppOps/component runner 参数、`pm hide` 成功/不可用分支，以及 ROM settings/shared_prefs 修改和恢复。它们不等同于真实 Android 设备验证；实机验证仍需在 Root 环境、目标 ROM 和目标应用版本上单独执行。

当前自动执行链路还应覆盖：

```sh
sh -n module/service.sh module/uninstall.sh module/scripts/puread-module-lib.sh module/action.sh
target/debug/puread-daemon --apply --root /tmp/puread-daemon-smoke/root --rules rules --state-dir /tmp/puread-daemon-smoke/state --ledger /tmp/puread-daemon-smoke/state/actions.jsonl --log-file /tmp/puread-daemon-smoke/daemon.log
```

## 开发约定

- 先阅读 [AGENTS.md](AGENTS.md) 再修改源码、规则或同步脚本。
- 查找历史决策时使用 AgentLogs MCP 的 `search_logs` / `read_log`，不要直接编辑日志文件。
- `Example/`、`AgentLogs/` 和 `target/` 不应进入提交。
- 提交信息遵循 `<type>(<scope>): <gitmoji> <subject>`，并包含 `Co-authored-by: Wine Fox <fox@ling.plus>`。
