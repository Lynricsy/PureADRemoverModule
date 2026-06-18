3:PureADRemoverModule 是一个 Android Root 环境下的非域名本地层去广告模块。它面向 Clash、代理和 DNS 无法覆盖的本地广告落地物，例如应用私有目录中的广告缓存、广告 SDK 文件、已知广告 SQLite 数据库，以及显式 profile 下的组件、AppOps 和 ROM 配置治理。
5:本项目不是 DNS、hosts、Clash、Mihomo、Box、AdGuardHome 或任何代理方案的替代品。域名、网络连接和代理路由应继续由用户已有网络层工具处理；本仓库只做可 dry-run、可审计、可回滚的本地文件和系统状态治理。
9:- `puread-core` 提供类型化规则模型、路径边界模型和恢复账本。
13:- `puread-cli` 提供规则校验、dry-run 扫描和账本查看/恢复入口。
17:默认策略是保守执行：真实修改前先规划，真实修改时写恢复账本，失败时保留可追踪状态。高风险能力必须由显式 profile 启用，不能默认大范围启用。
23:- Clash、Mihomo、Box、ProxyConfig 或其他代理配置改写。
27:## 上游同步
29:上游快照和规则更新流程以 [AGENTS.md](AGENTS.md) 为准。`Example/` 是 `.gitignore` 忽略的只读上游参考快照，不进入提交；未来同步只能从快照中提取非域名规则候选，并必须把禁止能力分类为 rejected。
55:- 先阅读 [AGENTS.md](AGENTS.md) 再修改源码、规则或同步脚本。
