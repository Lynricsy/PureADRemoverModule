mod appops_components;
mod rom;
mod shared;

use crate::command_runner::AndroidCommandRunner;
use crate::profiles::error::ProfileError;
use crate::profiles::record::{ProfileOperation, ProfileOperationStatus, ProfileRecord};

/// Profile 恢复记录接收端。
pub trait ProfileLedgerSink {
    /// 追加 JSON 格式恢复记录。
    fn append(&self, record: String) -> Result<(), ProfileError>;
}

/// Android profile 执行器。
#[derive(Debug)]
pub struct AndroidProfileExecutor<'a, R, L>
where
    R: AndroidCommandRunner,
    L: ProfileLedgerSink,
{
    pub(crate) runner: &'a R,
    ledger: &'a L,
}

impl<'a, R, L> AndroidProfileExecutor<'a, R, L>
where
    R: AndroidCommandRunner,
    L: ProfileLedgerSink,
{
    /// 构造 Android profile 执行器。
    #[must_use]
    pub const fn new(runner: &'a R, ledger: &'a L) -> Self {
        Self { runner, ledger }
    }

    pub(super) fn applied(&self, record: &ProfileRecord) -> Result<ProfileOperation, ProfileError> {
        self.operation(ProfileOperationStatus::Applied, record)
    }

    pub(super) fn skipped(&self, record: &ProfileRecord) -> Result<ProfileOperation, ProfileError> {
        self.operation(ProfileOperationStatus::Skipped, record)
    }

    fn operation(
        &self,
        status: ProfileOperationStatus,
        record: &ProfileRecord,
    ) -> Result<ProfileOperation, ProfileError> {
        let record = serde_json::to_string(record).map_err(ProfileError::json)?;
        self.ledger.append(record.clone())?;
        Ok(ProfileOperation { status, record })
    }
}
