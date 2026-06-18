/// 单次 Android 命令调用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    program: String,
    args: Vec<String>,
}

impl CommandInvocation {
    /// 构造一个不经 shell 展开的 argv 调用。
    #[must_use]
    pub fn new<I, S>(program: &str, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            program: program.to_owned(),
            args: args
                .into_iter()
                .map(|arg| arg.as_ref().to_owned())
                .collect(),
        }
    }

    /// 返回命令绝对路径。
    #[must_use]
    pub const fn program(&self) -> &str {
        self.program.as_str()
    }

    /// 返回命令参数。
    #[must_use]
    pub const fn args(&self) -> &[String] {
        self.args.as_slice()
    }

    /// 返回完整 argv，便于 dry-run 和证据记录。
    #[must_use]
    pub fn argv(&self) -> Vec<String> {
        let mut argv = Vec::with_capacity(self.args.len().saturating_add(1));
        argv.push(self.program.clone());
        argv.extend(self.args.iter().cloned());
        argv
    }
}
