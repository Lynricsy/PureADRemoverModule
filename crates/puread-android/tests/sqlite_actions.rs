#![doc = "`SQLite` 动作执行器行为测试。"]

#[path = "sqlite_actions/boundary.rs"]
mod boundary;
#[path = "sqlite_actions/execution.rs"]
mod execution;
include!("sqlite_actions/support.rs");
