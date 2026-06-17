# PureAD Rust 模块规划草案

status: approved
pending_action: write .omo/plans/puread-rust-module.md
mode: ulw-plan
tier: Architecture / HEAVY

## 用户目标

主人要求规划开发一个纯净、强力、低功耗的 Android Root 去广告模块：

- 已有代理模块，域名相关去广告由 Clash/代理处理。
- 新模块不要重复做域名、hosts、DNS 去广告。
- 之前建议合入的能力、谨慎合入的能力都合入。
- 之前明确不建议合入的能力不合入。
- 使用 Rust 编写核心逻辑。
- 尽量优化以降低模块功耗。
- 同时在目录里写一个 `AGENTS.md`，让未来 agent 知道如何获取最新上游文件，以及上游相关规则更新时如何同步到本模块。

## 分级依据

选择 Architecture / HEAVY，因为本计划涉及：

- 新 Rust workspace 与 Android native daemon。
- Magisk / KernelSU / APatch 模块生命周期。
- 文件系统事件监控、规则引擎、SQLite 清理、组件禁用、AppOps 降权、ROM 专项配置。
- 低功耗事件驱动设计和 Android 端真实验证。
- 当前顶层工作区不是 Git 仓库，需要计划显式处理变更基线风险。

已检查并确认不属于小改动：

- 不是单文件修改。
- 不是已有代码内的窄修复。
- 当前仓库没有现成 Rust 项目骨架。

## 本地证据

### 工作区现状

- 顶层路径：`/root/Projects/Ling/PureADRemoverModule`。
- 顶层不是 Git 仓库；`git status` 返回 `fatal: not a git repository`。
- `Example/Adguard-Home-For-Magisk-Mod` 自身是 Git 仓库，分支 `main...origin/main`。
- 当前没有 `.omo/`，本草案首次创建 `.omo/drafts/`。
- 当前可用素材主要是：
  - `Example/Adguard-Home-For-Magisk-Mod/Adguardhome/*`
  - `Example/ads288.zip`
  - `AgentLogs/0001-0003`

### AdGuard Home 示例可借鉴点

- `Example/Adguard-Home-For-Magisk-Mod/Adguardhome/scripts/NoAdsService.sh:7` 定义 `block_ad`，通过清空/删除广告路径并 `chattr +i` 锁定。
- `NoAdsService.sh:14` 到 `NoAdsService.sh:186` 列出大量应用广告缓存、开屏、广告资源路径。
- `NoAdsService.sh:188` 强制关闭私人 DNS，本计划明确排除。
- `NoAdsService.sh:191` 清理 IFW，本计划明确排除。
- `NoAdsService.sh:194` 清理 `/data/data/*==deleted==`，本计划不默认合入。

### ads288 / GGAT 示例可借鉴点

- `ads288.zip/service.sh:9` 到 `service.sh:16` 启动顶层 `mod/*.sh`，说明其能力是多脚本组合运行。
- `ads288.zip/配置.prop:25` 到 `配置.prop:31` 显示 `chattr锁定=否` 是默认策略，强锁应做成可选强力模式。
- `ads288.zip/配置.prop:37` 到 `配置.prop:41` 显示广告文件监视器默认启用。
- `ads288.zip/配置.prop:42` 到 `配置.prop:47` 显示动态 hosts 挂载默认禁用，本计划完全不合入 hosts 能力。
- `ads288.zip/mod/util_functions.sh:16` 到 `util_functions.sh:68` 提供删除、空文件占位、`chmod 000`、可选 `chattr +i`、恢复路径的原型。
- `ads288.zip/mod/ad.sh:27` 到 `ad.sh:150` 提供大量应用广告路径规则。
- `ads288.zip/mod/ads_monitor.sh:5` 到 `ads_monitor.sh:23` 展示 native 二进制广告文件监视器的启动方式。
- `ads288.zip/mod/sqlite_clean_up.sh:9` 到 `sqlite_clean_up.sh:43` 展示 SQLite 广告库清空/禁写能力。
- `ads288.zip/mod/disable_app.sh:9` 到 `disable_app.sh:40` 展示系统广告包禁用/恢复。
- `ads288.zip/mod/APPOPS.sh:1` 到 `APPOPS.sh:42` 展示 AppOps 降权。
- `ads288.zip/mod/mi_market.sh:10` 到 `mi_market.sh:40` 展示小米应用商店组件禁用和日志占位。
- `ads288.zip/mod/miui_ad.sh:3` 到 `miui_ad.sh:7` 展示 MIUI 广告开关与主题广告 Activity 禁用。
- `ads288.zip/package_extra.sh:102` 到 `package_extra.sh:132` 展示系统广告包 `.replace`、`pm disable`、`pm hide`、缓存清理。

## 外部约束

- Magisk 模块目录为 `/data/adb/modules/<MODID>`，需要 `module.prop`，可选 `post-fs-data.sh`、`service.sh`、`uninstall.sh`、`action.sh`。
- Magisk `module.prop` 的 `id` 需要匹配 `^[a-zA-Z][a-zA-Z0-9._-]+$`，`versionCode` 必须是整数。
- Magisk 脚本运行在 BusyBox `ash` standalone mode，脚本里不能假设系统 `PATH`。
- Magisk 官方推荐模块脚本用 `MODDIR=${0%/*}` 定位模块目录。
- APatch 和 KernelSU 不能假设与 Magisk 完全同构；APatch 有 `APATCH=true` 环境信号，且没有内置 Zygisk。
- APatch 支持额外启动阶段，但大多数模块如果只需要一个启动脚本，仍优先用 `service.sh`。
- `inotify` 适合目录/文件变更感知，`epoll` 适合多个 fd 的统一事件循环。
- Rust 交叉编译应显式区分 host build 和 Android target build，不能只用宿主测试证明 Android 端可运行。
- Android Doze / App Standby 会限制后台 CPU 和网络活动；本模块应避免轮询、避免 wake lock、避免前台服务依赖。

## 范围 IN

计划写入正式计划时，必须覆盖这些能力：

- Rust workspace 与 native daemon。
- Magisk / KernelSU / APatch 模块打包目录。
- 安装、启动、卸载、动作按钮脚本。
- 规则 schema 与规则校验：
  - 文件/目录广告资源规则。
  - 通用广告 SDK 缓存规则。
  - SQLite 广告库规则。
  - AppOps 规则。
  - Android 组件禁用规则。
  - ROM profile 规则。
- 低功耗 daemon：
  - `inotify + epoll` 事件驱动。
  - 无 busy loop。
  - boot complete / storage 可访问后启动。
  - 低频补扫，带 jitter 和退避。
  - 无 wake lock 默认依赖。
- 文件处理能力：
  - 删除。
  - 空文件占位。
  - 空目录占位。
  - `chmod 000`。
  - 可选 `chattr +i` 强力模式。
  - 状态记录与卸载恢复。
- SQLite 处理能力：
  - 删除/重建占位。
  - 写入最小合法 SQLite 头。
  - 可选禁写占位。
  - 默认低频或手动任务，不常驻高频扫描。
- 谨慎能力全部合入，但必须可选、可回滚：
  - `pm disable` / `pm enable` / `pm hide` / `pm unhide`。
  - `cmd appops set` / 恢复。
  - MIUI/OPPO/Xiaomi ROM 专项 profile。
  - 系统广告包 `.replace`，仅作为显式 profile，不默认打开。
- 管理入口：
  - `action.sh` 提供命令菜单或状态/扫描入口。
  - 不做 Web UI，除非正式计划之后主人另行要求。
- 测试与 QA：
  - Rust 逻辑 TDD。
  - 宿主端单元测试、属性测试和 CLI 场景测试。
  - Android 端 smoke/real-surface 验证步骤。
  - 打包 zip 结构校验。
  - 功耗/唤醒行为的可观测证据。

## 范围 OUT

正式计划必须明确禁止这些能力：

- hosts 生成、合并、挂载、动态切换。
- DNS 重定向。
- AdGuard Home。
- Clash / Box / Mihomo / 代理配置改写。
- 私人 DNS 强制关闭。
- 域名名单维护。
- 广告奖励域名切换。
- iptables 域名/字符串/IP/TLS 阻断。
- IFW 清空。
- 全局 `iptables -F`。
- Zygisk 隐藏、Root 环境隐藏、反检测。
- 前台服务实现。
- 默认启用 `chattr +i`。
- 默认启用系统包 `.replace`。

## 拟定架构

### Rust workspace

建议正式计划创建：

- `Cargo.toml`
- `.cargo/config.toml`
- `crates/puread-core`
- `crates/puread-daemon`
- `crates/puread-cli`
- `crates/puread-rules`
- `crates/puread-android`
- `module/`
- `rules/`
- `scripts/`
- `tests/`
- `README.md`

### 组件职责

- `puread-core`：规则类型、路径展开、计划动作、状态模型。
- `puread-rules`：TOML/JSON 规则解析、schema 校验、示例规则库。
- `puread-android`：Android/root 命令适配层；封装 `pm`、`cmd appops`、`settings`、`chcon`、`chattr` 等命令执行。
- `puread-daemon`：低功耗常驻守护，负责 inotify/epoll、低频补扫、事件去抖、状态记录。
- `puread-cli`：动作按钮调用的管理入口，支持状态、手动扫描、应用/撤销 profile、恢复。
- `module/`：Magisk/KSU/APatch 可刷入模块模板。
- `rules/`：默认规则；按通用 SDK、App profile、ROM profile 分层。
- `scripts/`：构建、打包、Android smoke QA 辅助脚本。
- `AGENTS.md`：项目级 agent 指南，记录上游来源、更新流程、规则同步流程、禁做事项和验证要求。

## 默认决策

这些默认决策将写入正式计划，除非主人修改：

- 测试策略：TDD。
- Rust 代码规则：禁止 `unwrap` / `expect` 于非测试代码，禁止未证明的 `unsafe`，单文件纯代码行不超过 250。
- 常驻方式：root native daemon，由 `service.sh` 启动。
- 省电策略：事件驱动 + 低频补扫，不使用固定 5 秒轮询，不申请 wake lock。
- 默认规则强度：
  - 文件占位默认 `chmod 000`。
  - `chattr +i` 仅强力 profile。
  - 组件禁用、AppOps、ROM 修改默认可选 profile，不一刀切。
- 默认平台目标：
  - 优先 `arm64`。
  - 同时保留 `arm`、`x86_64`、`x86` 打包路径。
  - `riscv64` 作为后续可选目标，除非工具链现成。
- 当前仓库不是 Git 仓库，计划不要求提交；若后续要 commit，需要先明确初始化/迁移 Git 策略。

## 待审批方案

如果主人批准，我将生成 `.omo/plans/puread-rust-module.md`，其中包含：

- 明确目录树。
- 逐 wave 执行策略。
- 每个 todo 的实现范围、引用证据、测试命令、QA 场景、验收标准。
- 不合入功能的硬性 guardrails。
- 最终验证 wave。
- 是否提交的策略说明。
- 根目录 `AGENTS.md` 的编写任务，要求说明未来如何从上游 Example/Adguard Home/ads288 来源获取新文件、如何 diff 规则、如何同步非域名规则、如何验证不引入域名/DNS/hosts 能力。

## 需要主人确认

推荐默认：采用 **TDD + Android smoke QA**，优先实现 `arm64` 可用模块，同时保留多架构打包位；把组件禁用/AppOps/ROM 修改做成显式 profile，不默认大范围启用。

主人已批准。下一步允许写正式 `.omo/plans/puread-rust-module.md`，但仍不允许实现。
