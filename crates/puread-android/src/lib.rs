#![doc = "`PureAD` Android 适配 crate 的最小脚手架。"]

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
