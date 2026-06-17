use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::error::LedgerError;
use super::record::{LedgerKey, LedgerRecord, action_name, path_depth, validate_module_id};

/// 状态账本文件名。
pub const ACTIONS_LEDGER_FILE: &str = "actions.jsonl";

/// 账本 append 结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendOutcome {
    /// 写入了新记录。
    Appended,
    /// 该路径、动作和 profile 已经存在。
    AlreadyPresent,
}

/// 恢复尝试结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreStatus {
    /// 恢复成功，该记录可移除。
    Succeeded,
    /// 恢复失败，该记录必须保留。
    Failed,
}

/// 单条恢复尝试。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreAttempt {
    /// 恢复目标键。
    pub key: LedgerKey,
    /// 恢复结果。
    pub status: RestoreStatus,
}

/// JSONL 恢复账本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreLedger {
    path: PathBuf,
}

impl RestoreLedger {
    /// 创建指向固定模块状态目录的账本。
    pub fn for_module(module_id: &str) -> Result<Self, LedgerError> {
        validate_module_id(module_id)?;
        Ok(Self::at(
            PathBuf::from("/data/adb/modules")
                .join(module_id)
                .join("state")
                .join(ACTIONS_LEDGER_FILE),
        ))
    }

    /// 创建指向指定路径的账本，主要用于测试和宿主工具。
    pub const fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// 返回账本路径。
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// 读取并校验 JSONL 账本记录。
    pub fn read_records(&self) -> Result<Vec<LedgerRecord>, LedgerError> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return self.io_error(source),
        };
        let mut records = Vec::new();
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let raw = line.map_err(|source| self.with_io(source))?;
            if raw.trim().is_empty() {
                continue;
            }
            let record = serde_json::from_str::<LedgerRecord>(&raw).map_err(|source| {
                LedgerError::JsonLine {
                    line: index.saturating_add(1),
                    source,
                }
            })?;
            record.validate()?;
            records.push(record);
        }
        Ok(records)
    }

    /// 幂等追加一条记录。
    pub fn append_record(&self, record: &LedgerRecord) -> Result<AppendOutcome, LedgerError> {
        record.validate()?;
        let key = record.key();
        if self
            .read_records()?
            .iter()
            .any(|stored| stored.key() == key)
        {
            return Ok(AppendOutcome::AlreadyPresent);
        }
        self.append_json_line(record)?;
        Ok(AppendOutcome::Appended)
    }

    /// 返回反向恢复顺序：更深路径优先，同深度时较新的记录优先。
    pub fn records_for_restore(&self) -> Result<Vec<LedgerRecord>, LedgerError> {
        let mut records = self.read_records()?;
        records.sort_by(compare_restore_records);
        Ok(records)
    }

    /// 应用恢复尝试结果；失败记录始终保留。
    pub fn apply_restore_attempts(
        &self,
        attempts: &[RestoreAttempt],
    ) -> Result<usize, LedgerError> {
        let succeeded = attempt_keys(attempts, RestoreStatus::Succeeded);
        if succeeded.is_empty() {
            return Ok(0);
        }
        let records = self.read_records()?;
        let failed = attempt_keys(attempts, RestoreStatus::Failed);
        let original_len = records.len();
        let kept = records
            .into_iter()
            .filter(|record| {
                let key = record.key();
                !succeeded.contains(&key) || failed.contains(&key)
            })
            .collect::<Vec<_>>();
        let removed = original_len.saturating_sub(kept.len());
        self.write_records(&kept)?;
        Ok(removed)
    }

    fn append_json_line(&self, record: &LedgerRecord) -> Result<(), LedgerError> {
        ensure_parent_dir(&self.path)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|source| self.with_io(source))?;
        write_json_line(&mut file, record, &self.path)
    }

    fn write_records(&self, records: &[LedgerRecord]) -> Result<(), LedgerError> {
        ensure_parent_dir(&self.path)?;
        let mut file = File::create(&self.path).map_err(|source| self.with_io(source))?;
        for record in records {
            write_json_line(&mut file, record, &self.path)?;
        }
        Ok(())
    }

    fn io_error<T>(&self, source: std::io::Error) -> Result<T, LedgerError> {
        Err(self.with_io(source))
    }

    fn with_io(&self, source: std::io::Error) -> LedgerError {
        LedgerError::Io {
            path: self.path.clone(),
            source,
        }
    }
}

fn attempt_keys(attempts: &[RestoreAttempt], status: RestoreStatus) -> HashSet<LedgerKey> {
    attempts
        .iter()
        .filter(|attempt| attempt.status == status)
        .map(|attempt| attempt.key.clone())
        .collect()
}

fn ensure_parent_dir(path: &Path) -> Result<(), LedgerError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|source| LedgerError::Io {
        path: parent.to_path_buf(),
        source,
    })
}

fn write_json_line(file: &mut File, record: &LedgerRecord, path: &Path) -> Result<(), LedgerError> {
    serde_json::to_writer(&mut *file, record)
        .map_err(|source| LedgerError::JsonWrite { source })?;
    file.write_all(b"\n").map_err(|source| LedgerError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn compare_restore_records(left: &LedgerRecord, right: &LedgerRecord) -> Ordering {
    path_depth(&right.original_path)
        .cmp(&path_depth(&left.original_path))
        .then_with(|| right.timestamp.cmp(&left.timestamp))
        .then_with(|| left.original_path.cmp(&right.original_path))
        .then_with(|| action_name(left.action).cmp(action_name(right.action)))
        .then_with(|| left.profile.cmp(&right.profile))
}
