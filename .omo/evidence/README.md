# PureADRemoverModule Evidence

本目录用于保存任务执行过程中的可复查证据，例如基线状态、验证命令输出、人工检查摘录和同步审查报告。

## 写入规则

- 证据文件按任务编号命名，例如 `task-6-baseline-before.txt`。
- 不删除其他 worker 已生成的证据。
- 命令输出应尽量包含执行命令、关键结果和人工判断依据。
- 失败证据也要保留，不能用后续成功结果覆盖问题现场。
- 大型日志或临时缓存不应放入本目录，除非任务明确要求保存。

## T6 证据

- `task-6-baseline-before.txt`：创建 `scripts/verify-local.sh` 前的 failing-first 基线。
- `task-6-shell-parse.txt`：`sh -n scripts/verify-local.sh` 的解析检查输出。
- `task-6-verify-script.txt`：验证脚本表面内容快照，用于人工确认脚本没有伪造成功证据。

