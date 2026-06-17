use crate::error::ModelError;

const PROFILE_FIELD: &str = "profile";

/// 规则启用画像。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProfileKind {
    /// 默认保守文件类画像。
    Conservative,
    /// SDK 缓存画像。
    SdkCache,
    /// `SQLite` 手动或低频画像。
    Sqlite,
    /// 显式组件画像。
    Component,
    /// 显式 `AppOps` 画像。
    AppOps,
    /// 显式 ROM 配置画像。
    Rom,
}

impl ProfileKind {
    /// 解析规则启用画像。
    pub fn parse(raw: &str) -> Result<Self, ModelError> {
        match raw {
            "conservative" => Ok(Self::Conservative),
            "sdk_cache" => Ok(Self::SdkCache),
            "sqlite" => Ok(Self::Sqlite),
            "component" => Ok(Self::Component),
            "appops" => Ok(Self::AppOps),
            "rom" => Ok(Self::Rom),
            _ => Err(unsupported_profile(raw)),
        }
    }

    /// 返回规则文件使用的画像名。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Conservative => "conservative",
            Self::SdkCache => "sdk_cache",
            Self::Sqlite => "sqlite",
            Self::Component => "component",
            Self::AppOps => "appops",
            Self::Rom => "rom",
        }
    }
}

fn unsupported_profile(raw: &str) -> ModelError {
    if raw.is_empty() {
        return ModelError::Empty {
            field: PROFILE_FIELD,
        };
    }
    ModelError::UnsupportedValue {
        field: PROFILE_FIELD,
        value: raw.to_owned(),
    }
}
