#![doc = "`PureAD` CLI crate 的最小脚手架。"]

/// CLI crate 标识。
#[must_use]
pub const fn crate_name() -> &'static str {
    "puread-cli"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn crate_name_returns_cli_crate_identifier_when_smoke_test_runs() {
        // Given: the CLI crate is compiled as a workspace member.
        // When: its smoke-test API is called.
        let name = crate_name();

        // Then: the observable identifier is stable.
        assert_eq!(name, "puread-cli");
    }
}
