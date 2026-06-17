# PureAD Rust 非域名去广告模块开发计划

## TL;DR
> Summary:      从零建立一个 Rust 驱动的 Android Root 去广告模块，专注 Clash/代理无法覆盖的本地广告落地物：文件/目录缓存、广告 SDK SQLite、AppOps、组件禁用和 ROM profile。模块必须低功耗、事件驱动、可回滚，并明确禁止 hosts、DNS、域名和代理配置接管能力。
> Deliverables:
> - Rust workspace：`puread-core`、`puread-rules`、`puread-android`、`puread-daemon`、`puread-cli`
> - 可刷入模块模板：`module/` 下 `module.prop`、`service.sh`、`uninstall.sh`、`action.sh`、多架构 binary 布局
> - 默认非域名规则库：文件/SDK/SQLite/AppOps/组件/ROM profile
> - 根目录 `AGENTS.md`：上游获取、规则同步、禁做事项、验证流程
> - 构建、打包、宿主测试、Android smoke QA、功耗/唤醒证据脚本
> Effort:       XL
> Risk:         High - 新工程、Android root 生命周期、多平台兼容、可回滚系统改动与低功耗守护进程都在同一交付里

## Scope
### Must have
- 使用 Rust 编写核心逻辑，建立 strict Cargo workspace。
- 只做非域名去广告：本地文件/目录、广告 SDK 缓存、SQLite 广告库、AppOps、组件禁用、ROM profile。
- 合入“建议合入”和“谨慎合入”的能力，但谨慎能力必须是显式 profile、默认不大范围启用、可回滚。
- 低功耗 daemon 使用 `inotify + epoll` 阻塞事件循环，允许低频补扫但禁止 5 秒固定轮询。
- `service.sh` 启动 root native daemon；`action.sh` 调用 Rust CLI 提供状态、手动扫描、应用/撤销 profile、恢复。
- 状态记录必须能支持卸载恢复：文件占位、`chattr +i`、AppOps、组件禁用、ROM 设置都要有恢复证据。
- `AGENTS.md` 必须说明未来 agent 如何获取最新上游文件、比较上游规则、同步非域名规则、验证未引入禁用能力。
- `AGENTS.md` 必须包含这些章节：项目边界、禁止能力、上游源清单、当前快照记录、上游同步流程、规则分类矩阵、规则溯源字段、验证命令、日志记录要求、Git/非 Git 注意事项、`Example/` 只读约定。
- 上游同步只能把上游作为审查快照；只允许提取非域名规则候选，不允许复制 hosts/DNS/代理/iptables/network/ad_reward 能力。
- 每条同步规则必须带 `source`、`source_file` 或 `zip_entry`、`source_line_or_pattern` 或可搜索片段哈希、`category`、`default_enabled`、`rollback_strategy`。
- 状态账本必须记录原路径、动作、原文件类型、mode、uid、gid、SELinux context、immutable 状态、时间戳、profile，并支持反向恢复。
- daemon、CLI、卸载脚本必须使用全局锁，防止并发 scan/apply/restore/uninstall 损坏状态。
- README 必须解释：本模块不是 DNS/hosts/Clash 替代品，而是 Clash 之外的本地层补强。
- 所有 Rust 文件遵守 250 pure LOC 上限；非测试代码禁止 `unwrap` / `expect`；禁止未证明的 `unsafe`。

### Must NOT have
- 不做 hosts 生成、合并、挂载、动态切换。
- 不做 DNS 重定向、AdGuard Home、私人 DNS 强制关闭。
- 不改 Clash / Box / Mihomo / 代理配置。
- 不维护域名名单、不做广告奖励域名切换。
- 不做 iptables 域名/字符串/IP/TLS 阻断。
- 不清空 IFW、不执行全局 `iptables -F`。
- 不做 Zygisk 隐藏、Root 环境隐藏、反检测。
- 不实现前台服务，不默认申请 wake lock。
- 不默认启用 `chattr +i`，不默认启用系统包 `.replace`。
- 不在 `Example/` 示例项目里实现新模块；示例只作为规则来源证据。
- 不复制或生成 `hosts`、`Host/`、`host.sh`、`mount_hosts*`、`iptables.sh`、`ad_reward*`、代理配置、AdGuardHome 二进制。
- 不导入 `其他脚本/IFW规则转PM命令.sh` 或 IFW XML 规则；除非未来人工转换成明确 component disable profile，且默认禁用、可回滚。

## Current Baseline
- 正式计划文件是当前已有 `.omo/plans/` 下的新文件：`.omo/plans/puread-rust-module.md`。
- 当前已有 `.omo/drafts/puread-rust-module.md`；执行计划时不得把创建 `.omo/` 当作验收点。
- 顶层目录当前不是 Git 仓库；执行者不能依赖顶层 `git diff` 作为唯一变更基线。
- 项目 Git 上游远端指定为 `git@github.com:Lynricsy/PureADRemoverModule.git`；开始实现前若顶层仍不是 Git 仓库，执行者必须初始化仓库、设置远端并确认可推送。不得设置 local `user.name`、`user.email` 或其他身份信息。
- 开发过程中需要及时提交并推送；提交必须使用用户要求的格式 `<type>(<scope>): <gitmoji> <subject>`，并包含 `Co-authored-by: Wine Fox <fox@ling.plus>`。
- `Example/Adguard-Home-For-Magisk-Mod` 是嵌套 Git 仓库；`Example/ads288.zip` 是本地 zip 快照，不能声称是最新上游。
- `Example/` 必须写入 `.gitignore`，作为本地上游参考快照保留，不纳入本项目提交。

## Rule Classification Matrix
| Category | Sync? | Default | Rollback required | Notes |
|---|---:|---|---:|---|
| `file_path` | Yes | enabled for conservative file/dir placeholders | Yes | App-local ad cache, splash, SDK cache paths only |
| `sdk_cache` | Yes | enabled if file/dir only | Yes | Pangle/GDT/Kwai/BeiZi/AnyThink cache names |
| `sqlite` | Yes | disabled/manual or low-frequency | Yes | Must target known ad SDK DB paths |
| `component` | Yes | disabled | Yes | `pm disable` / `pm hide`, explicit profile only |
| `appops` | Yes | disabled | Yes | Explicit profile only |
| `rom_profile` | Yes | disabled | Yes | Ad-related settings/XML only |
| `hosts` | No | n/a | n/a | Reject `hosts`, `Host/`, host generation/mounting |
| `dns` | No | n/a | n/a | Reject DNS redirect/private DNS changes |
| `proxy` | No | n/a | n/a | Reject Clash/Box/Mihomo edits |
| `iptables_network` | No | n/a | n/a | Reject string/IP/TLS/domain blocking |
| `ad_reward_domain` | No | n/a | n/a | Reject reward domain switching |
| `ifw_clear` | No | n/a | n/a | Reject IFW clear/import |

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: TDD with Rust `cargo test` / `proptest` where meaningful; shell lifecycle uses Bats-style or POSIX shell harness if introduced, otherwise CLI smoke scripts.
- QA policy: every todo has agent-executed scenarios; Android/device-only actions must include host-side dry-run plus explicit Android smoke command.
- Evidence: `.omo/evidence/task-<N>-<slug>.<ext>`
- Required recurring gates:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --workspace`
  - `cargo test --workspace --target <android-target>` where toolchain is available, otherwise record missing target as evidence and keep host tests mandatory
  - module zip structure validation
  - grep guard proving no forbidden DNS/hosts/iptables/proxy behavior entered production code
  - Android smoke QA script command captured, or explicit “device unavailable” artifact with dry-run output
- Forbidden scope scan must fail on implementation use of `hosts`, `DNS`, `iptables`, `private_dns`, `mihomo`, `clash`, `AdGuardHome`, `ProxyConfig`, `mount_hosts`, `ad_reward`, while allowing those tokens only inside documentation sections that explicitly label them as forbidden/out-of-scope.
- Android power evidence when a device is available:
  - `adb shell dumpsys batterystats`
  - `adb shell dumpsys alarm`
  - `adb shell dumpsys deviceidle`
  - `adb shell top -b -n 1` or equivalent process CPU sample
  - expected: no wake lock held by PureAD, no alarm storm, daemon idle CPU approximately 0, no 5-second loop log cadence.
- If no rooted Android device is available, implementation must record `.omo/evidence/android-device-unavailable.txt` and must not claim Android real-device verification complete.

## Execution strategy
### Parallel execution waves
> Target 5-8 todos per wave. < 3 per wave (except the final) = under-splitting.
Wave 1 (no deps): T1, T2, T3, T4, T5, T6
Wave 2 (after T1-T6): T7, T8, T9, T10, T11, T12
Wave 3 (after T7-T12): T13, T14, T15, T16, T17, T18
Wave 4 (after T13-T18): T19, T20, T21, T22, T23, T24
Wave 5 (after T19-T24): T25, T26, T27, T28, T29, T30
Final verification wave: F1-F4
Critical path: T1 -> T3 -> T7 -> T10 -> T13 -> T16 -> T20 -> T25 -> Final verification

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
|---|---|---|---|
| T1 | none | T3, T4, T5, T7 | T2, T6 |
| T2 | none | T11, T21, T28, T29 | T1, T3, T4 |
| T3 | T1 | T7, T8, T9, T10 | T2, T4, T5 |
| T4 | T1 | T7, T13, T14 | T2, T3, T5 |
| T5 | T1 | T12, T19, T20, T27 | T2, T3, T4 |
| T6 | none | T21, T29 | T1, T2 |
| T7 | T3, T4 | T10, T13, T16 | T8, T9 |
| T8 | T3 | T13, T14 | T7, T9 |
| T9 | T3 | T15, T16 | T7, T8 |
| T10 | T7 | T16, T20 | T11, T12 |
| T11 | T2 | T21 | T10, T12 |
| T12 | T5 | T19, T24 | T10, T11 |
| T13 | T7, T8 | T17, T20 | T14, T15 |
| T14 | T8 | T17, T20 | T13, T15 |
| T15 | T9 | T18, T20 | T13, T14 |
| T16 | T7, T9, T10 | T20, T25 | T17, T18 |
| T17 | T13, T14 | T23, T25 | T16, T18 |
| T18 | T15 | T23, T25 | T16, T17 |
| T19 | T5, T12 | T20, T24, T25 | T21 |
| T20 | T5, T10, T13, T14, T15, T16, T19 | T25, T26 | T21 |
| T21 | T2, T6, T11 | T26, T29 | T19, T20 |
| T22 | T1, T3 | T25, T28 | T19, T20, T21 |
| T23 | T17, T18 | T25, T27 | T22, T24 |
| T24 | T12, T19 | T25, T29 | T22, T23 |
| T25 | T16, T17, T18, T19, T20, T22, T23, T24 | F1-F4 | T26, T27 |
| T26 | T20, T21 | F1-F4 | T25, T27 |
| T27 | T5, T23 | F1-F4 | T25, T26 |
| T28 | T22 | F1-F4 | T29, T30 |
| T29 | T2, T6, T21, T24 | F1-F4 | T28, T30 |
| T30 | T1-T29 | F1-F4 | none |

## Todos
> Implementation + Test = ONE todo. Never separate.

- [x] 1. 建立 Rust workspace 与 strict 工具链
  What to do / Must NOT do: 创建 `Cargo.toml` workspace、`.cargo/config.toml`、`rustfmt.toml`、基础 crate 目录和最小可编译入口；设置 clippy 严格门禁。不要写业务逻辑，不要超过 250 pure LOC。
  Parallelization: Can parallel Y | Wave 1 | Blocks T3/T4/T5/T7
  References: `.omo/drafts/puread-rust-module.md`; `/root/.codex/plugins/cache/sisyphuslabs/omo/4.10.0/skills/programming/references/rust/README.md`
  Acceptance criteria (agent-executable): `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --workspace`
  QA scenarios (name the exact tool + invocation): `cargo metadata --format-version 1 > .omo/evidence/task-1-workspace-metadata.json`; `cargo test --workspace | tee .omo/evidence/task-1-cargo-test.txt`
  Commit: N | build(rust): scaffold strict workspace | Files `Cargo.toml`, `.cargo/config.toml`, `crates/*`, `rustfmt.toml`

- [x] 2. 编写根目录 `AGENTS.md` 上游同步指南
  What to do / Must NOT do: 创建根目录 `AGENTS.md`，说明项目边界、禁止能力、上游源清单、当前快照记录字段、获取方法、规则 diff、同步流程、规则分类矩阵、禁止引入 DNS/hosts/domain/proxy/iptables 能力、验证命令和日志记录规则。声明项目 Git 上游远端为 `git@github.com:Lynricsy/PureADRemoverModule.git`，开发中需要及时提交并推送，且不得设置 local git 身份信息。声明 `Example/` 是只读参考/上游快照，除同步快照步骤外不得改写，并且必须位于 `.gitignore`。不要创建或编辑 AgentLogs 文件；日志必须继续使用 MCP。
  Parallelization: Can parallel Y | Wave 1 | Blocks T11/T21/T29
  References: `.omo/drafts/puread-rust-module.md`; `AgentLogs/0003-纯净非域名去广告模块合入建议.md`; `Example/ads288.zip`; `Example/Adguard-Home-For-Magisk-Mod/Adguardhome/scripts/NoAdsService.sh:7`
  Acceptance criteria (agent-executable): `test -f AGENTS.md && rg -n "项目边界|禁止能力|上游源|当前快照|同步流程|规则分类|验证命令|AgentLogs|search_logs|Example" AGENTS.md`
  QA scenarios (name the exact tool + invocation): `sed -n '1,320p' AGENTS.md > .omo/evidence/task-2-agents-content.txt`; `rg -n "hosts|DNS|iptables|Clash|AdGuardHome|mount_hosts|ad_reward" AGENTS.md > .omo/evidence/task-2-forbidden-docs.txt`; `rg -n "只读|不得改写|快照|SHA256|commit|git@github.com:Lynricsy/PureADRemoverModule.git|及时提交|推送" AGENTS.md > .omo/evidence/task-2-upstream-policy.txt`; `rg -n "^Example/$" .gitignore > .omo/evidence/task-2-gitignore-example.txt`
  Commit: Y | docs(project): 📝 add agent upstream sync guide | Files `AGENTS.md`, `.gitignore`

- [x] 3. 定义规则数据模型和 typed invariants
  What to do / Must NOT do: 在 `puread-core` 定义 `RuleId`、`PackageName`、`RootPath`、`RuleAction`、`ProfileKind`、`RiskLevel`、`RestoreToken` 等类型；用 enum 表达动作，不用字符串散落。不要接触 Android 命令执行。
  Parallelization: Can parallel Y | Wave 1 | Blocks T7/T8/T9/T10
  References: `Example/ads288.zip/mod/util_functions.sh:16`; `Example/ads288.zip/mod/util_functions.sh:52`; `/root/.codex/plugins/cache/sisyphuslabs/omo/4.10.0/skills/programming/references/rust/type-state.md`
  Acceptance criteria (agent-executable): 首先写失败测试覆盖非法包名/非法路径/动作解析；随后 `cargo test -p puread-core rule_model`
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-core -- --nocapture | tee .omo/evidence/task-3-core-tests.txt`
  Commit: N | feat(core): model non-domain rule primitives | Files `crates/puread-core/**`

- [x] 4. 实现路径展开和危险路径拒绝
  What to do / Must NOT do: 支持 exact/glob/package-relative/name-match 规则展开；硬拒绝 `/`、`/data`、`/sdcard`、`/storage`、`/system`、`/vendor`、`/data/adb` 非本模块目录、空路径、`..`、符号链接逃逸、根级通配删除等危险路径。只允许受控模板如 `/data/user/*/<pkg>`、`/data/data/<pkg>`、`/sdcard/Android/data/<pkg>`。不要执行删除。
  Parallelization: Can parallel Y | Wave 1 | Blocks T7/T13/T14
  References: `Example/ads288.zip/mod/util_functions.sh:18`; `Example/ads288.zip/mod/util_functions.sh:53`; `Example/ads288.zip/mod/ad.sh:27`
  Acceptance criteria (agent-executable): TDD 覆盖危险路径拒绝、`/data/user/[0-9]*/pkg` 展开、`Android/data` 展开；`cargo test -p puread-core path_expansion`
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-core path_expansion -- --nocapture | tee .omo/evidence/task-4-path-tests.txt`
  Commit: N | feat(core): add safe path expansion | Files `crates/puread-core/**`

- [x] 5. 建立状态记录与恢复账本模型
  What to do / Must NOT do: 定义 JSONL 或 SQLite 状态账本格式，路径固定为 `/data/adb/modules/<id>/state/actions.*`；记录原路径、动作、原文件类型、mode、uid、gid、SELinux context、immutable 状态、时间戳、profile、恢复步骤。恢复失败必须保留原记录。不要把状态存在临时目录。
  Parallelization: Can parallel Y | Wave 1 | Blocks T12/T19/T27
  References: `Example/ads288.zip/mod/util_functions.sh:34`; `Example/Adguard-Home-For-Magisk-Mod/Adguardhome/uninstall.sh`
  Acceptance criteria (agent-executable): TDD 覆盖账本 append、幂等去重、恢复顺序排序；`cargo test -p puread-core restore_ledger`
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-core restore_ledger -- --nocapture | tee .omo/evidence/task-5-ledger-tests.txt`
  Commit: N | feat(core): add reversible state ledger | Files `crates/puread-core/**`

- [x] 6. 建立 `.omo/evidence` 和验证脚本骨架
  What to do / Must NOT do: 创建 evidence 目录占位说明、验证脚本骨架和 README 中的 QA 约定。不要伪造 evidence 内容。
  Parallelization: Can parallel Y | Wave 1 | Blocks T21/T29
  References: `.omo/drafts/puread-rust-module.md`
  Acceptance criteria (agent-executable): `test -d .omo/evidence && test -f scripts/verify-local.sh && sh -n scripts/verify-local.sh`
  QA scenarios (name the exact tool + invocation): `sh -n scripts/verify-local.sh | tee .omo/evidence/task-6-shell-parse.txt`
  Commit: N | chore(qa): scaffold evidence and verification scripts | Files `.omo/evidence/README.md`, `scripts/verify-local.sh`

- [x] 7. 实现规则解析和 schema 校验
  What to do / Must NOT do: 在 `puread-rules` 解析 TOML 规则，校验 action、profile、包名、路径、风险等级、默认启用状态、source metadata、rollback_strategy。解析边界使用 serde，不在业务层重复字符串验证。
  Parallelization: Can parallel Y | Wave 2 | Blocks T10/T13/T16
  References: T3/T4 outputs; `Example/ads288.zip/配置.prop:25`; `Example/ads288.zip/配置.prop:37`
  Acceptance criteria (agent-executable): TDD 覆盖每类规则至少一个 valid fixture 和一个 invalid fixture；未知字段失败、禁用能力字段失败、缺少 rollback/source metadata 失败；`cargo test -p puread-rules`
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-rules -- --nocapture | tee .omo/evidence/task-7-rules-tests.txt`
  Commit: N | feat(rules): parse and validate non-domain rules | Files `crates/puread-rules/**`

- [x] 8. 转换默认文件/SDK 缓存规则库
  What to do / Must NOT do: 从 `NoAdsService.sh` 与 `ads288.zip/mod/ad.sh`、应用专项脚本提取非域名文件规则到 `rules/common/*.toml`、`rules/apps/<package>.toml`。每条规则必须带 source metadata、category、default_enabled、rollback_strategy。不要提取任何域名、hosts、iptables 字符串规则。
  Parallelization: Can parallel Y | Wave 2 | Blocks T13/T14
  References: `Example/Adguard-Home-For-Magisk-Mod/Adguardhome/scripts/NoAdsService.sh:14`; `Example/ads288.zip/mod/ad.sh:27`; `Example/ads288.zip/mod/killpangle.sh`; `AgentLogs/0002-分析-Example-模块去广告能力-阶段2.md`
  Acceptance criteria (agent-executable): `cargo run -p puread-cli -- rules validate rules/common rules/apps` 成功；`! rg -n "127\\.0\\.0\\.1|0\\.0\\.0\\.0|iptables|dns|hosts|gdt\\.qq\\.com|doubleclick" rules/common rules/apps`
  QA scenarios (name the exact tool + invocation): `cargo run -p puread-cli -- rules list --kind files > .omo/evidence/task-8-file-rules-list.txt`; `rg -n "splash|pangle|GDTDOWNLOAD|TTCache" rules/common rules/apps > .omo/evidence/task-8-rule-sample.txt`
  Commit: N | feat(rules): add file and SDK cache profiles | Files `rules/common/**`, `rules/apps/**`

- [x] 9. 转换 SQLite 广告库规则
  What to do / Must NOT do: 从 `sqlite_clean_up.sh` 和 `coolapk.sh` 提取 SQLite 规则，动作分为 delete、minimal-sqlite、deny-write。默认低频或手动，不加入高频监控。只允许明确广告 SDK DB 路径；每条规则必须带 source metadata 和 rollback_strategy。
  Parallelization: Can parallel Y | Wave 2 | Blocks T15/T16
  References: `Example/ads288.zip/mod/sqlite_clean_up.sh:9`; `Example/ads288.zip/mod/coolapk.sh`
  Acceptance criteria (agent-executable): `cargo run -p puread-cli -- rules validate rules/sqlite` 成功；规则含调度字段 `manual|boot_once|low_frequency`
  QA scenarios (name the exact tool + invocation): `cargo run -p puread-cli -- rules list --kind sqlite > .omo/evidence/task-9-sqlite-rules-list.txt`
  Commit: N | feat(rules): add SQLite ad database profiles | Files `rules/sqlite/**`

- [x] 10. 实现 dry-run 执行计划生成
  What to do / Must NOT do: CLI 支持 `scan --dry-run`，读取规则并输出将要处理的路径和动作，不修改文件。不要执行 root 命令。
  Parallelization: Can parallel Y | Wave 2 | Blocks T16/T20
  References: T7/T8/T9 outputs
  Acceptance criteria (agent-executable): TDD/CLI 测试验证 dry-run 输出稳定 JSON；`cargo run -p puread-cli -- scan --dry-run --rules rules/files --root tests/fixtures/android-fs`
  QA scenarios (name the exact tool + invocation): `cargo run -p puread-cli -- scan --dry-run --rules rules/files --root tests/fixtures/android-fs > .omo/evidence/task-10-dry-run.json`
  Commit: N | feat(cli): generate dry-run action plans | Files `crates/puread-cli/**`, `tests/fixtures/**`

- [x] 11. 编写上游同步工具骨架
  What to do / Must NOT do: 创建 `scripts/update-upstream.sh` 或 Rust CLI 子命令骨架，用于刷新上游快照到 `upstream/` 或校验本地 `Example/`；输出 diff 报告。记录 AdGuard 嵌套仓库 commit、remote URL、`ads288.zip` SHA256、获取日期。当前 `ads288.zip` 只能视为本地快照；若未来主人提供新版 zip，脚本记录新 SHA256。不要自动改规则，不要自行从非指定来源下载替换 zip。
  Parallelization: Can parallel Y | Wave 2 | Blocks T21
  References: `AGENTS.md`; `Example/ads288.zip`; `Example/Adguard-Home-For-Magisk-Mod`
  Acceptance criteria (agent-executable): `sh -n scripts/update-upstream.sh`; dry-run 生成 `.omo/evidence/task-11-upstream-dry-run.txt`，包含 commit/SHA256 字段。
  QA scenarios (name the exact tool + invocation): `scripts/update-upstream.sh --dry-run | tee .omo/evidence/task-11-upstream-dry-run.txt`; `sha256sum Example/ads288.zip > .omo/evidence/task-11-ads288-sha256.txt`; `git -C Example/Adguard-Home-For-Magisk-Mod rev-parse HEAD > .omo/evidence/task-11-adguard-head.txt`
  Commit: N | chore(upstream): add upstream refresh dry-run script | Files `scripts/update-upstream.sh`, `upstream/README.md`

- [x] 12. 实现恢复账本 CLI
  What to do / Must NOT do: CLI 支持 `ledger show`、`restore --dry-run`，能读取状态账本并输出恢复动作。不要执行真实恢复。
  Parallelization: Can parallel Y | Wave 2 | Blocks T19/T24
  References: T5 output
  Acceptance criteria (agent-executable): `cargo test -p puread-cli ledger`; `cargo run -p puread-cli -- restore --dry-run --ledger tests/fixtures/ledger.json`
  QA scenarios (name the exact tool + invocation): `cargo run -p puread-cli -- restore --dry-run --ledger tests/fixtures/ledger.json > .omo/evidence/task-12-restore-dry-run.json`
  Commit: N | feat(cli): add restore ledger dry-run | Files `crates/puread-cli/**`

- [ ] 13. 实现文件动作执行器
  What to do / Must NOT do: 在 `puread-android` 或 core adapter 中实现 delete、empty-file、empty-dir、chmod-000、chown/chcon 封装和账本记录。必须支持 dry-run 与真实执行分离。若无法确定 SELinux context，只记录并跳过高风险路径，不盲目 `chcon`。不要默认 chattr。
  Parallelization: Can parallel Y | Wave 3 | Blocks T17/T20
  References: `Example/ads288.zip/mod/util_functions.sh:52`; `Example/ads288.zip/mod/ad.sh:152`
  Acceptance criteria (agent-executable): TDD 用临时目录验证动作和恢复；`cargo test -p puread-android file_actions`
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-android file_actions -- --nocapture | tee .omo/evidence/task-13-file-actions.txt`
  Commit: N | feat(android): execute reversible file actions | Files `crates/puread-android/**`

- [ ] 14. 实现可选 `chattr +i` 强力模式
  What to do / Must NOT do: `chattr` 只能在强力 profile 中执行，先检测命令存在，记录原属性，失败要降级并写日志。不要让默认规则触发 chattr。
  Parallelization: Can parallel Y | Wave 3 | Blocks T17/T20
  References: `Example/ads288.zip/配置.prop:25`; `Example/ads288.zip/mod/util_functions.sh:16`
  Acceptance criteria (agent-executable): mock command runner 测试强力/默认模式分支；`cargo test -p puread-android chattr`
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-android chattr -- --nocapture | tee .omo/evidence/task-14-chattr-tests.txt`; `rg -n "chattr" rules | tee .omo/evidence/task-14-chattr-rules.txt`
  Commit: N | feat(android): gate immutable file mode behind profiles | Files `crates/puread-android/**`, `rules/**`

- [ ] 15. 实现 SQLite 动作执行器
  What to do / Must NOT do: 实现删除、写最小 SQLite 头、deny-write 占位三种动作；默认只由 boot_once/manual/low_frequency 调度触发。执行前记录元信息，避免高频处理正在使用的数据库；完整性验证不能只检查固定二进制头。不要监控每次写入。
  Parallelization: Can parallel Y | Wave 3 | Blocks T18/T20
  References: `Example/ads288.zip/mod/sqlite_clean_up.sh:9`
  Acceptance criteria (agent-executable): TDD 验证生成文件可被 SQLite 工具或 Rust sqlite parser 识别，恢复账本可逆，正在使用/权限失败时记录错误不中断批处理；`cargo test -p puread-android sqlite_actions`
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-android sqlite_actions -- --nocapture | tee .omo/evidence/task-15-sqlite-actions.txt`
  Commit: N | feat(android): add reversible SQLite ad database actions | Files `crates/puread-android/**`

- [ ] 16. 实现 daemon 事件循环骨架
  What to do / Must NOT do: `puread-daemon` 使用 inotify/epoll 或 Rust crate 对应封装建立阻塞事件循环；支持 shutdown signal、reload signal、事件去抖。禁止 busy loop。
  Parallelization: Can parallel Y | Wave 3 | Blocks T20/T25
  References: 外部约束：inotify/epoll man pages; `.omo/drafts/puread-rust-module.md`
  Acceptance criteria (agent-executable): unit/integration test 用临时目录创建文件触发事件；源码 grep 不得出现 `sleep(Duration::from_secs(5))` 或无限固定轮询。
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-daemon event_loop -- --nocapture | tee .omo/evidence/task-16-daemon-event-loop.txt`; `rg -n "sleep\\(|loop \\{|while true" crates/puread-daemon > .omo/evidence/task-16-loop-grep.txt`
  Commit: N | feat(daemon): add event-driven file watcher | Files `crates/puread-daemon/**`

- [ ] 17. 集成文件规则到 daemon
  What to do / Must NOT do: daemon 加载文件规则，建立 watch，事件触发后执行文件动作并写账本。必须支持 dry-run daemon 模式。不要加载 SQLite/AppOps/组件规则进高频 watcher。
  Parallelization: Can parallel Y | Wave 3 | Blocks T23/T25
  References: T8/T13/T14/T16 outputs
  Acceptance criteria (agent-executable): 临时 Android-like fixture 下启动 daemon dry-run，创建 `splashCache` 后输出计划动作；`cargo test -p puread-daemon file_rule_integration`
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-daemon file_rule_integration -- --nocapture | tee .omo/evidence/task-17-file-daemon.txt`
  Commit: N | feat(daemon): apply file rules from watcher events | Files `crates/puread-daemon/**`

- [ ] 18. 集成低频维护调度
  What to do / Must NOT do: 实现 boot_once、manual、low_frequency 调度，带 jitter 和退避；用于 SQLite 和补扫。默认参数：首次启动补扫一次，常规补扫间隔不低于 6 小时，失败指数退避到 24 小时上限，带随机 jitter。不要引入固定短周期扫描。
  Parallelization: Can parallel Y | Wave 3 | Blocks T23/T25
  References: Android 省电约束; `Example/ads288.zip/mod/ads_monitor_Check.sh`
  Acceptance criteria (agent-executable): 测试调度不会生成小于配置阈值的固定周期；`cargo test -p puread-daemon scheduler`
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-daemon scheduler -- --nocapture | tee .omo/evidence/task-18-scheduler.txt`
  Commit: N | feat(daemon): add low-power maintenance scheduler | Files `crates/puread-daemon/**`

- [ ] 19. 实现 Android 命令适配层
  What to do / Must NOT do: 封装 `pm`、`cmd appops`、`settings`、`getprop`、`chcon`、`chattr`、`lsattr` 命令，所有调用可 dry-run、可记录输出、可注入 fake runner。每个 adapter 必须有 `probe`、`apply`、`restore`、`dry_run`。`settings` 只允许 ROM profile 白名单键，禁止 `private_dns_mode`、`private_dns_specifier`、DNS/hosts/proxy 相关键。不要散落 shell 字符串。
  Parallelization: Can parallel Y | Wave 4 | Blocks T24/T25
  References: `Example/ads288.zip/mod/disable_app.sh:16`; `Example/ads288.zip/mod/APPOPS.sh:1`; `Example/ads288.zip/mod/miui_ad.sh:3`
  Acceptance criteria (agent-executable): fake runner 单元测试覆盖命令参数；`cargo test -p puread-android command_runner`
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-android command_runner -- --nocapture | tee .omo/evidence/task-19-command-runner.txt`
  Commit: N | feat(android): add injectable Android command runner | Files `crates/puread-android/**`

- [ ] 20. CLI 手动扫描和应用 profile
  What to do / Must NOT do: `puread-cli` 支持 `status`、`scan`、`apply-profile`、`restore`、`dump-report`，默认 dry-run 提示，真实执行需要显式 `--execute`。所有修改动作必须获取全局锁 `/data/adb/modules/<id>/run/puread.lock`。不要让 action.sh 默认执行破坏性动作，不实现 Web UI。
  Parallelization: Can parallel Y | Wave 4 | Blocks T25/T26
  References: `Example/ads288.zip/action.sh`; T10/T13/T14/T15/T16 outputs
  Acceptance criteria (agent-executable): CLI e2e 测试覆盖 dry-run 默认和 `--execute` 分支；`cargo test -p puread-cli cli_profiles`
  QA scenarios (name the exact tool + invocation): `cargo run -p puread-cli -- scan --dry-run --root tests/fixtures/android-fs > .omo/evidence/task-20-cli-scan.json`
  Commit: N | feat(cli): manage scans and profiles safely | Files `crates/puread-cli/**`

- [ ] 21. 上游规则同步流程实现
  What to do / Must NOT do: 完成 `scripts/update-upstream.sh` 和/或 CLI 子命令，能刷新上游压缩包与 AdGuard 示例、解包到 `upstream/`、生成 `upstream_manifest.json` 和非域名规则候选 diff。同步快照可以包含上游文件用于审查，但规则导入只允许 `file_path`、`sdk_cache`、`sqlite`、`component`、`appops`、`rom_profile` 候选。硬拒绝 `hosts`、`Host/`、`host.sh`、`mount_hosts*`、`iptables.sh`、`ad_reward*`、DNS/代理配置文件。不要自动合并候选规则。
  Parallelization: Can parallel Y | Wave 4 | Blocks T26/T29
  References: `AGENTS.md`; `Example/ads288.zip`; `Example/Adguard-Home-For-Magisk-Mod`
  Acceptance criteria (agent-executable): `scripts/update-upstream.sh --dry-run` 和 `scripts/update-upstream.sh --from-local Example --report-only` 生成报告；报告包含 accepted/rejected 分类，任何命中 hosts/DNS/proxy/iptables/network/ad_reward 关键词的条目必须进入 rejected 或导致失败。
  QA scenarios (name the exact tool + invocation): `scripts/update-upstream.sh --from-local Example --report-only | tee .omo/evidence/task-21-upstream-report.txt`; `jq '.rejected' upstream/upstream_manifest.json > .omo/evidence/task-21-rejected.json`
  Commit: N | chore(upstream): report non-domain rule update candidates | Files `scripts/update-upstream.sh`, `upstream/**`

- [ ] 22. 编写模块模板和生命周期脚本
  What to do / Must NOT do: 在 `module/` 创建 `module.prop`、`customize.sh`、`service.sh`、`uninstall.sh`、`action.sh`、`scripts/`；使用 `MODDIR=${0%/*}`；按 ARCH 选择 binary；适配 Magisk/APatch 环境变量。不要硬编码 `/data/adb/modules/PureAD` 为唯一来源。
  Parallelization: Can parallel Y | Wave 4 | Blocks T25/T28
  References: Magisk 官方约束; APatch 官方约束; `Example/ads288.zip/service.sh:1`; `Example/Adguard-Home-For-Magisk-Mod/Adguardhome/service.sh`
  Acceptance criteria (agent-executable): `sh -n module/*.sh module/scripts/*.sh`; `rg -n "MODDIR=\\$\\{0%/\\*\\}" module`; 脚本包含 Magisk/KSU/APatch 环境变量检测和 ABI 选择逻辑，但不声称所有平台实机通过。
  QA scenarios (name the exact tool + invocation): `find module -maxdepth 3 -type f -print | sort > .omo/evidence/task-22-module-files.txt`; `sh -n module/service.sh module/uninstall.sh module/action.sh | tee .omo/evidence/task-22-shell-parse.txt`
  Commit: N | feat(module): add root module lifecycle scripts | Files `module/**`

- [ ] 23. 实现 AppOps 与组件 profile
  What to do / Must NOT do: 从 `APPOPS.sh`、`disable_app.sh`、`mi_market.sh`、`123pan.sh`、`com.luna.music.sh` 等提取 AppOps/组件 profile；通过命令适配层执行并写账本。组件禁用记录原 enabled/hidden 状态，AppOps 记录原 mode 或 default；`pm hide` 必须 capability-detect，不可用则跳过并记录。默认仅可选 profile。
  Parallelization: Can parallel Y | Wave 4 | Blocks T25/T27
  References: `Example/ads288.zip/mod/APPOPS.sh:1`; `Example/ads288.zip/mod/disable_app.sh:9`; `Example/ads288.zip/mod/mi_market.sh:10`; `Example/ads288.zip/mod/123pan.sh`
  Acceptance criteria (agent-executable): fake runner 验证 `pm disable/enable` 和 `cmd appops set` 参数；规则校验显示 `default_enabled=false`
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-android appops components -- --nocapture | tee .omo/evidence/task-23-appops-components.txt`; `rg -n "default_enabled = true" rules/appops rules/components > .omo/evidence/task-23-default-enabled-grep.txt || true`
  Commit: N | feat(profiles): add reversible AppOps and component profiles | Files `rules/appops/**`, `rules/components/**`, `crates/puread-android/**`

- [ ] 24. 实现 ROM profile 与 settings/XML 修改
  What to do / Must NOT do: 添加 MIUI/OPPO/Xiaomi profile，支持 `settings` 和 shared_prefs XML 修改，执行前检测 ROM。必须记录原值、原文件哈希、备份路径；默认不启用。ROM profile 限定广告相关组件/广告开关，禁止云备份、隐私保护、通知管理、网络限制等非广告目的改动。不要写通用跨 ROM 破坏性逻辑。
  Parallelization: Can parallel Y | Wave 4 | Blocks T25/T29
  References: `Example/ads288.zip/mod/miui_ad.sh:3`; `Example/ads288.zip/mod/modtify_xml.sh`; `Example/ads288.zip/package_extra.sh:125`
  Acceptance criteria (agent-executable): fixture XML 修改/恢复测试；fake `getprop` 检测非 MIUI 时跳过；`cargo test -p puread-android rom_profiles`
  QA scenarios (name the exact tool + invocation): `cargo test -p puread-android rom_profiles -- --nocapture | tee .omo/evidence/task-24-rom-profiles.txt`
  Commit: N | feat(profiles): add reversible ROM ad settings profiles | Files `rules/rom/**`, `crates/puread-android/**`

- [ ] 25. 集成完整 daemon/CLI/module 打包流
  What to do / Must NOT do: 构建 Rust binary，复制到 `module/bin/<arch>/`，生成刷入 zip。不要包含上游 zip 原件或域名规则文件。
  Parallelization: Can parallel Y | Wave 5 | Blocks F1-F4
  References: T16/T20/T22 outputs
  Acceptance criteria (agent-executable): `scripts/build-module.sh --dry-run`、`scripts/package-module.sh` 产出 zip；zip 根含 `module.prop`, `service.sh`, `uninstall.sh`, `action.sh`, `bin/<abi>/puread-daemon`, `bin/<abi>/puread-cli`, `rules/`；不得含 hosts 文件、AdGuardHome 二进制、代理配置。
  QA scenarios (name the exact tool + invocation): `scripts/package-module.sh | tee .omo/evidence/task-25-package.txt`; `unzip -l dist/*.zip > .omo/evidence/task-25-zip-list.txt`; `! unzip -l dist/*.zip | rg "(^|/)hosts$|AdGuardHome|iptables"`
  Commit: N | build(module): package PureAD root module | Files `scripts/build-module.sh`, `scripts/package-module.sh`, `module/**`, `dist/`

- [ ] 26. 添加本地全量验证脚本
  What to do / Must NOT do: `scripts/verify-local.sh` 串联 fmt、clippy、tests、规则校验、禁用能力 grep、zip 结构校验、pure LOC 检查。禁用能力扫描必须支持文档白名单上下文，禁止实现路径出现 DNS/hosts/proxy/iptables/private-DNS/IFW/root-hiding。不要让脚本吞掉失败码。
  Parallelization: Can parallel Y | Wave 5 | Blocks F1-F4
  References: T1/T20/T21/T25 outputs
  Acceptance criteria (agent-executable): `scripts/verify-local.sh` 退出 0，并生成 evidence。
  QA scenarios (name the exact tool + invocation): `scripts/verify-local.sh | tee .omo/evidence/task-26-verify-local.txt`
  Commit: N | chore(qa): add full local verification gate | Files `scripts/verify-local.sh`

- [ ] 27. 添加 Android smoke QA 脚本
  What to do / Must NOT do: 提供 `scripts/qa-android.sh`，通过 adb 推送 zip/binary 或在已安装模块上执行 `puread-cli status`、dry-run scan、profile dry-run、daemon 启停检查。最低真机验收面：至少一台 arm64 root Android 设备；安装 zip 后验证 `module.prop`、`service.sh` 启动 daemon、`action.sh status`、手动扫描一条沙箱规则、卸载恢复。KSU/APatch 若无设备则只做脚本兼容性静态校验，不声称实机通过。设备不可用时必须输出明确 artifact，不得假装通过。
  Parallelization: Can parallel Y | Wave 5 | Blocks F1-F4
  References: 外部 Android/Rust 交叉编译约束; T22/T25 outputs
  Acceptance criteria (agent-executable): `sh -n scripts/qa-android.sh`; 有设备时 `scripts/qa-android.sh` 运行并采集 `dumpsys batterystats`、`dumpsys alarm`、`dumpsys deviceidle`、`top -b -n 1`；无设备时 `scripts/qa-android.sh --dry-run` 生成命令计划和 device-unavailable artifact。
  QA scenarios (name the exact tool + invocation): `scripts/qa-android.sh --dry-run | tee .omo/evidence/task-27-android-qa-dry-run.txt`
  Commit: N | test(android): add root module smoke QA script | Files `scripts/qa-android.sh`

- [ ] 28. 编写 README 使用与边界说明
  What to do / Must NOT do: README 说明模块用途、安装、配置、默认安全配置、默认 profile、强力模式风险、Clash 分工、卸载恢复、上游规则更新入口指向 `AGENTS.md`。不要宣传 DNS/hosts 能力。
  Parallelization: Can parallel Y | Wave 5 | Blocks F1-F4
  References: `.omo/drafts/puread-rust-module.md`; `AGENTS.md`
  Acceptance criteria (agent-executable): `rg -n "Clash|非域名|低功耗|恢复|上游|AGENTS" README.md`; `! rg -n "DNS重定向|hosts挂载|AdGuard Home" README.md`
  QA scenarios (name the exact tool + invocation): `sed -n '1,260p' README.md > .omo/evidence/task-28-readme.txt`
  Commit: N | docs(readme): document PureAD scope and usage | Files `README.md`

- [ ] 29. 完成上游更新回归场景
  What to do / Must NOT do: 用本地 `Example/` 做一次 report-only 更新流程，生成候选规则报告；验证报告不会自动改规则，并标出需要人工/agent 审核的候选。报告必须显示 `ads288.zip` SHA256、AdGuard 嵌套仓库 commit、accepted/rejected 分类。不要联网下载作为唯一证据。
  Parallelization: Can parallel Y | Wave 5 | Blocks F1-F4
  References: T2/T21 outputs; `Example/ads288.zip`; `Example/Adguard-Home-For-Magisk-Mod`
  Acceptance criteria (agent-executable): `scripts/update-upstream.sh --from-local Example --report-only` 退出 0；`git` 不可用时脚本仍用文件快照报告。
  QA scenarios (name the exact tool + invocation): `scripts/update-upstream.sh --from-local Example --report-only | tee .omo/evidence/task-29-upstream-regression.txt`
  Commit: N | test(upstream): verify upstream rule update workflow | Files `scripts/update-upstream.sh`, `.omo/evidence/**`

- [ ] 30. 记录规划落实日志
  What to do / Must NOT do: 使用 `record-agent-log` 记录实现阶段完成内容和原因。不要手工编辑 AgentLogs。若执行者无法调用 MCP，必须在最终报告中说明未记录。
  Parallelization: Can parallel N | Wave 5 | Blocks F1-F4
  References: 用户 AGENTS.md 指令; `AGENTS.md`
  Acceptance criteria (agent-executable): MCP 返回日志路径；最终报告引用日志编号。
  QA scenarios (name the exact tool + invocation): `record-agent-log` MCP 输出记录到最终响应；无需文件命令。
  Commit: N | chore(log): record implementation rationale | Files none

## Final verification wave (after ALL todos)
> Runs in parallel. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete.
- [ ] F1. Plan compliance audit
  Verify every Must have exists, every Must NOT have is absent, and `AGENTS.md` includes upstream sync instructions. Evidence: `.omo/evidence/f1-plan-compliance.txt`
- [ ] F2. Code quality review
  Run `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, pure LOC check, and shell parse checks. Evidence: `.omo/evidence/f2-code-quality.txt`
- [ ] F3. Real manual QA
  Run package zip validation, CLI dry-run against fixtures, daemon tempdir event scenario, Android smoke dry-run or device run, and power evidence capture when device exists. Evidence: `.omo/evidence/f3-manual-qa.txt`
- [ ] F4. Scope fidelity
  Grep and inspect for forbidden DNS/hosts/domain/proxy/iptables/private-DNS/IFW/root-hiding behavior. Evidence: `.omo/evidence/f4-scope-fidelity.txt`

## Commit strategy
- The user explicitly requested timely commits and pushes during development.
- Current top-level workspace is not a Git repository; before implementation, initialize Git if still absent, then set `origin` to `git@github.com:Lynricsy/PureADRemoverModule.git` and verify remote connectivity. Do not set local git identity config.
- If commits are later requested, use the user’s required format: `<type>(<scope>): <gitmoji> <subject>` and include `Co-authored-by: Wine Fox <fox@ling.plus>`.
- Keep commits atomic by wave or cohesive subsystem:
  - `build(rust): 🏗️ scaffold strict workspace`
  - `feat(core): ✨ add reversible non-domain rule engine`
  - `feat(daemon): ⚡ add low-power file watcher`
  - `feat(module): ✨ add root module packaging`
  - `docs(project): 📝 add upstream sync agent guide`
- Push after each completed atomic commit or wave-level checkpoint when remote access succeeds; if push fails because authentication or remote setup is unavailable, record the exact failure and continue with local commits.

## Success criteria
- A downstream worker can execute this plan without asking new design questions.
- Rust workspace builds and tests locally.
- Module package can be generated and inspected.
- CLI dry-run proves file rules do not mutate by default.
- Daemon tempdir test proves event-driven behavior.
- `AGENTS.md` tells future agents how to refresh upstream files and sync only allowed non-domain rules.
- Forbidden DNS/hosts/domain/proxy/iptables/private-DNS/IFW/root-hiding behavior is absent by grep and review.
- Android smoke QA command is available and either runs on a device or produces an explicit device-unavailable dry-run artifact.
