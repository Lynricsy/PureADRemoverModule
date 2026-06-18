use puread_core::model::ProfileKind;

/// immutable 能力 profile。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImmutableProfile {
    /// 显式强力 profile，可执行 `chattr +i`。
    Strong,
    /// 其他规则 profile，仅允许产生 skip/plan。
    Rule(ProfileKind),
}

impl ImmutableProfile {
    /// 返回是否允许执行 immutable mutation。
    #[must_use]
    pub const fn is_strong(self) -> bool {
        matches!(self, Self::Strong)
    }
}

impl From<ProfileKind> for ImmutableProfile {
    fn from(value: ProfileKind) -> Self {
        Self::Rule(value)
    }
}
