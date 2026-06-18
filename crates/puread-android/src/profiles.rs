//! Android AppOps、组件和 ROM profile 执行层。

mod error;
mod executor;
mod record;
mod rules;
mod xml;
mod xml_bool;
mod xml_hooks;

pub use error::ProfileError;
pub use executor::{AndroidProfileExecutor, ProfileLedgerSink};
pub use record::{ProfileOperation, ProfileOperationStatus};
pub use rules::{
    AppOpProfileRule, ComponentProfileRule, PmHidePolicy, RomMatcher, RomProfileRule,
    RomSettingsRule, SharedPrefsBoolRule,
};
