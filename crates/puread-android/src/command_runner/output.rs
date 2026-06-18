/// Android 命令输出。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    status: i32,
    stdout: String,
    stderr: String,
}

impl CommandOutput {
    /// 构造成功输出。
    #[must_use]
    pub fn success(stdout: impl Into<String>, stderr: impl Into<String>) -> Self {
        Self {
            status: 0,
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    /// 构造失败输出。
    #[must_use]
    pub fn failure(status: i32, stdout: impl Into<String>, stderr: impl Into<String>) -> Self {
        Self {
            status,
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    /// 按真实进程退出码构造输出。
    #[must_use]
    pub fn from_status(status: i32, stdout: impl Into<String>, stderr: impl Into<String>) -> Self {
        Self {
            status,
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    /// 返回退出码。
    #[must_use]
    pub const fn status(&self) -> i32 {
        self.status
    }

    /// 返回标准输出。
    #[must_use]
    pub const fn stdout(&self) -> &str {
        self.stdout.as_str()
    }

    /// 返回标准错误。
    #[must_use]
    pub const fn stderr(&self) -> &str {
        self.stderr.as_str()
    }

    pub(crate) const fn is_success(&self) -> bool {
        self.status == 0
    }
}
