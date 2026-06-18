/// 文件动作后需要封装处理的元数据变更。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MetadataChange {
    /// 设置目标 uid/gid。
    SetOwner {
        /// 目标 uid。
        uid: u32,
        /// 目标 gid。
        gid: u32,
    },
    /// 设置显式给出的 `SELinux` context。
    SetSelinuxContext {
        /// 目标 context。
        context: String,
    },
    /// 尝试恢复动作前已知的 `SELinux` context。
    RestoreSelinuxContext,
}

impl MetadataChange {
    /// 创建 chown 封装请求。
    #[must_use]
    pub const fn set_owner(uid: u32, gid: u32) -> Self {
        Self::SetOwner { uid, gid }
    }

    /// 创建显式 chcon 封装请求。
    #[must_use]
    pub fn set_selinux_context(context: impl Into<String>) -> Self {
        Self::SetSelinuxContext {
            context: context.into(),
        }
    }

    /// 创建基于动作前 context 的 chcon 封装请求。
    #[must_use]
    pub const fn restore_selinux_context() -> Self {
        Self::RestoreSelinuxContext
    }
}

/// 元数据封装的可观测执行结果。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MetadataOperation {
    /// 已设置 uid/gid。
    SetOwner {
        /// 目标 uid。
        uid: u32,
        /// 目标 gid。
        gid: u32,
    },
    /// 已设置 `SELinux` context。
    Chcon {
        /// 已设置 context。
        context: String,
    },
    /// 动作前 context 不可确定，因此跳过 chcon。
    SkippedChconUnknownContext,
}
