use std::fmt;

use crate::error::RuleParseError;

const FORBIDDEN_CATEGORIES: &[&str] = &[
    "hosts",
    "dns",
    "domain",
    "proxy",
    "iptables_network",
    "ad_reward_domain",
    "ifw_clear",
];

/// 允许进入生产规则库的本地治理类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RuleCategory {
    /// 应用私有目录中的广告缓存或落地文件。
    FilePath,
    /// 广告 SDK 缓存目录或文件。
    SdkCache,
    /// 已知广告 SDK `SQLite` 数据库。
    Sqlite,
    /// 显式 profile 下的 Android 组件禁用。
    Component,
    /// 显式 profile 下的 `AppOps` 设置。
    AppOps,
    /// ROM 广告相关本地 profile。
    RomProfile,
}

impl RuleCategory {
    pub(super) fn parse(raw: &str) -> Result<Self, RuleParseError> {
        match raw {
            "file_path" => Ok(Self::FilePath),
            "sdk_cache" => Ok(Self::SdkCache),
            "sqlite" => Ok(Self::Sqlite),
            "component" => Ok(Self::Component),
            "appops" => Ok(Self::AppOps),
            "rom_profile" => Ok(Self::RomProfile),
            forbidden if is_forbidden_category(forbidden) => {
                Err(RuleParseError::ForbiddenCategory {
                    category: forbidden.to_owned(),
                })
            }
            unknown => Err(RuleParseError::UnsupportedCategory {
                category: unknown.to_owned(),
            }),
        }
    }

    /// 返回规则文件中使用的类别名。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FilePath => "file_path",
            Self::SdkCache => "sdk_cache",
            Self::Sqlite => "sqlite",
            Self::Component => "component",
            Self::AppOps => "appops",
            Self::RomProfile => "rom_profile",
        }
    }
}

fn is_forbidden_category(raw: &str) -> bool {
    FORBIDDEN_CATEGORIES.contains(&raw)
}

impl fmt::Display for RuleCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
