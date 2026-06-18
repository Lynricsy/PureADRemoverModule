#![doc = "`PureAD` Android 适配 crate 的最小脚手架。"]

/// `chattr +i` 强力 profile 适配层。
pub mod chattr;
/// 可注入的 Android 命令适配层。
pub mod command_runner;
/// 可恢复文件动作执行器。
pub mod file_actions;
/// Android profile 执行层。
pub mod profiles;
/// `SQLite` 广告库动作执行器。
pub mod sqlite_actions;

#[doc(hidden)]
pub mod secure_fs;

/// Android 适配 crate 标识。
#[must_use]
pub const fn crate_name() -> &'static str {
    "puread-android"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn crate_name_returns_android_crate_identifier_when_smoke_test_runs() {
        // Given: the Android adapter crate is compiled as a workspace member.
        // When: its smoke-test API is called.
        let name = crate_name();

        // Then: the observable identifier is stable.
        assert_eq!(name, "puread-android");
    }
}
