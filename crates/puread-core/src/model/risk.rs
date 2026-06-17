use crate::error::ModelError;

const RISK_FIELD: &str = "risk";

/// 规则风险等级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RiskLevel {
    /// 低风险。
    Low,
    /// 中风险。
    Medium,
    /// 高风险。
    High,
}

impl RiskLevel {
    /// 解析风险等级。
    pub fn parse(raw: &str) -> Result<Self, ModelError> {
        match raw {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(unsupported_risk(raw)),
        }
    }

    /// 返回规则文件使用的风险名。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

fn unsupported_risk(raw: &str) -> ModelError {
    if raw.is_empty() {
        return ModelError::Empty { field: RISK_FIELD };
    }
    ModelError::UnsupportedValue {
        field: RISK_FIELD,
        value: raw.to_owned(),
    }
}
