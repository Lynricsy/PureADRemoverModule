# PureADRemoverModule

PureADRemoverModule 是一个 Android Root 环境下的非域名本地层去广告模块。它面向 Clash、代理和 DNS 无法覆盖的本地广告落地物，例如应用私有目录中的广告缓存、广告 SDK 文件、已知广告 SQLite 数据库，以及显式 profile 下的组件、AppOps 和 ROM 配置治理。

本项目不是 DNS、hosts、Clash、Mihomo、Box、AdGuardHome 或任何代理方案的替代品。域名、网络连接和代理路由应继续由用户已有网络层工具处理；本仓库只做可 dry-run、可审计、可回滚的本地文件和系统状态治理。

## 用途与分工

PureADRemoverModule 只补齐网络层工具覆盖不到的 Android 本地层场景：清理或占位应用私有广告缓存、处理广告 SDK SQLite 数据、执行明确 profile 下的 AppOps/component/ROM 广告项治理，并通过 ledger 记录可恢复动作。

Clash、Mihomo、Box、DNS、代理路由和域名规则属于网络层职责；本项目不生成网络规则、不改代理配置，也不接管解析或转发链路。推荐分工是：网络层继续处理域名和连接路由，PureAD 只处理已经落到本机文件、数据库或系统状态里的广告痕迹。

## 安装与打包

模块模板位于 [`module/`](module/)，其中包含 Magisk、KernelSU、APatch 通用的 `module.prop`、`customize.sh`、`service.sh`、`action.sh`、`uninstall.sh` 和模块辅助脚本。打包入口约定为 [`scripts/package-module.sh`](scripts/package-module.sh)，产物应以模块 zip 形式安装到支持 Root 模块的 Android 环境。

不要把当前本地构建、fixture 测试或脚本静态校验理解为实机通过。真实安装仍需要在目标 Root 管理器、目标 ABI、目标 ROM 和目标应用版本上单独验证；README 只说明仓库入口和使用边界，不声明已经覆盖所有设备组合。

## 当前能力

- `puread-core` 提供类型化规则模型、路径边界模型和恢复账本。
- `puread-rules` 解析 TOML 规则，并拒绝 hosts、DNS、domain、proxy、iptables、ad_reward、IFW 清空和 Root 隐藏等禁止类别。
- `puread-android` 执行可回滚的文件动作、SQLite 动作、可选 `chattr` 封装和可注入 Android 命令适配层。命令适配层覆盖 `pm`、`cmd appops`、`settings`、`getprop`、`chcon`、`chattr`、`lsattr`，支持 fake runner 与 dry-run，并拒绝 DNS、hosts、proxy、private DNS 相关 settings。
- `puread-android` 支持显式 profile 下的可逆 AppOps、component 和 ROM profile。AppOps 通过 `cmd appops get/set` 记录原 mode 或 default 后再设置目标 mode；component 记录原 enabled/hidden 状态并通过 `pm disable-user` / `pm enable` 恢复；`pm hide` 是 runner-backed capability attempt，不可用或失败时跳过并写入 skipped 记录，成功时写入 durable confirmed record，恢复时才执行 `pm unhide`。ROM profile 仅限定广告相关 `settings` 与 `shared_prefs` XML 布尔项，执行前检测 ROM，记录原值、原文件哈希和备份路径；不包含 DNS、private DNS、proxy 或 network 修改。
- `puread-daemon` 提供事件驱动的文件规则守护能力、低频调度策略和文件规则 dry-run/apply 集成。
- `puread-cli` 支持 `status`、`scan`、`apply-profile`、`profile-report`、`profile-restore`、`restore`、`dump-report` 和 `rules validate`。`scan` 与 `apply-profile` 默认 dry-run，真实执行必须显式传入 `--execute`；修改动作会获取模块全局锁，profile 执行会写入 `profile-actions.jsonl` JSONL ledger，用于 `profile-report` 查看和 `profile-restore` 恢复。普通文件恢复继续走 `restore --ledger ...` 和文件恢复账本。
- `module/` 提供 Magisk、KernelSU、APatch 通用模块模板和生命周期脚本。宿主 dry-run 不写入模块运行态目录；Android 实机启动仍需后续二进制产物。

## 配置与默认安全策略

默认策略是保守执行：先生成计划，再由用户决定是否真实执行；真实修改必须写入恢复账本，失败时保留可追踪状态。CLI 的 `scan` 和 `apply-profile` 默认是 dry-run，真实执行必须显式传入 `--execute`。

默认 profile 只应包含低风险、可解释、可恢复的本地规则。危险能力必须显式启用，例如 AppOps、component、ROM profile、强力模式或应用专项 profile；这些能力不得在默认流程里大范围启用。

强力模式的风险来自系统状态变更，不来自网络层改写：

- AppOps 会读取并记录原 mode 或 default，再设置目标 mode；恢复时依赖 profile ledger。
- component 会记录原 enabled/hidden 状态，再按 profile 执行禁用或恢复。
- `pm hide` 只是 capability attempt：设备、ROM 或权限不支持时记录 skipped 并跳过；只有实际成功才写入 durable confirmed record，恢复时才尝试 `pm unhide`。
- ROM profile 只处理明确广告相关的 settings 或 `shared_prefs` XML 布尔项，执行前记录原值、原文件哈希和备份路径。

## 低功耗行为

`puread-daemon` 面向低功耗运行：文件规则由事件驱动触发，维护任务使用低频调度，不做固定 5 秒轮询，也不默认申请 wake lock。SQLite、AppOps、component 和 ROM profile 不进入高频 watcher；这些高风险或较重动作应通过手动命令、boot_once 或明确 profile 触发。

## 恢复与卸载

恢复依赖 ledger，而不是猜测当前设备状态。普通文件动作使用 `restore --ledger ...` 查看或执行恢复计划；profile 动作写入 `profile-actions.jsonl`，可通过 `profile-report` 查看历史动作，再用 `profile-restore` 按记录恢复。

模块卸载脚本会优先检查 ledger，并把 restore dry-run 计划写入日志和状态文件；真实恢复建议在移除模块前由用户显式执行 CLI 恢复命令。这样可以避免卸载阶段缺少二进制、权限或运行态目录时误判恢复成功。

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

## 开发约定

- 先阅读 [AGENTS.md](AGENTS.md) 再修改源码、规则或同步脚本。
- 查找历史决策时使用 AgentLogs MCP 的 `search_logs` / `read_log`，不要直接编辑日志文件。
- `Example/`、`AgentLogs/` 和 `target/` 不应进入提交。
- 提交信息遵循 `<type>(<scope>): <gitmoji> <subject>`，并包含 `Co-authored-by: Wine Fox <fox@ling.plus>`。
