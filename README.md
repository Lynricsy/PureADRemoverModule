# PureADRemoverModule

PureADRemoverModule 是一个 Android Root 环境下的非域名本地层去广告模块。它面向 Clash、代理和 DNS 无法覆盖的本地广告落地物，例如应用私有目录中的广告缓存、广告 SDK 文件、已知广告 SQLite 数据库，以及显式 profile 下的组件、AppOps 和 ROM 配置治理。

本项目不是 DNS、hosts、Clash、Mihomo、Box、AdGuardHome 或任何代理方案的替代品。域名、网络连接和代理路由应继续由用户已有网络层工具处理；本仓库只做可 dry-run、可审计、可回滚的本地文件和系统状态治理。

## 当前能力

- `puread-core` 提供类型化规则模型、路径边界模型和恢复账本。
- `puread-rules` 解析 TOML 规则，并拒绝 hosts、DNS、domain、proxy、iptables、ad_reward、IFW 清空和 Root 隐藏等禁止类别。
- `puread-android` 执行可回滚的文件动作、SQLite 动作和可选 `chattr` 封装。
- `puread-daemon` 提供事件驱动的文件规则守护能力、低频调度策略和文件规则 dry-run/apply 集成。
- `puread-cli` 提供规则校验、dry-run 扫描和账本查看/恢复入口。

## 安全边界

默认策略是保守执行：真实修改前先规划，真实修改时写恢复账本，失败时保留可追踪状态。高风险能力必须由显式 profile 启用，不能默认大范围启用。

生产路径不得实现以下能力：

- hosts 生成、合并、挂载或动态切换。
- DNS 重定向、私人 DNS 强制关闭、DNS 劫持或 DNS 服务接管。
- Clash、Mihomo、Box、ProxyConfig 或其他代理配置改写。
- iptables 域名、字符串、IP、TLS/SNI 阻断。
- 广告奖励域名切换、IFW 清空、Root 隐藏或反检测规避。

## 上游同步

上游快照和规则更新流程以 [AGENTS.md](AGENTS.md) 为准。`Example/` 是 `.gitignore` 忽略的只读上游参考快照，不进入提交；未来同步只能从快照中提取非域名规则候选，并必须把禁止能力分类为 rejected。

推荐入口：

```sh
scripts/update-upstream.sh --dry-run
```

同步报告必须记录来源、commit 或 SHA256、允许候选、拒绝类别和验证结果。

## 本地验证

常用质量门：

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
./scripts/verify-local.sh
```

Rust LSP 诊断依赖本机 `rust-analyzer`。如果当前 toolchain 未安装 `rust-analyzer`，以 Cargo 格式化、Clippy 和测试结果作为本地 Rust 验证证据。

## 开发约定

- 先阅读 [AGENTS.md](AGENTS.md) 再修改源码、规则或同步脚本。
- 查找历史决策时使用 AgentLogs MCP 的 `search_logs` / `read_log`，不要直接编辑日志文件。
- `Example/`、`AgentLogs/` 和 `target/` 不应进入提交。
- 提交信息遵循 `<type>(<scope>): <gitmoji> <subject>`，并包含 `Co-authored-by: Wine Fox <fox@ling.plus>`。
