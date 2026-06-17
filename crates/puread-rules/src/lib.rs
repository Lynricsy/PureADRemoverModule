#![doc = "`PureAD` 非域名规则 TOML 解析与 schema 校验。"]

#[doc(hidden)]
pub mod category;
#[doc(hidden)]
pub mod error;
#[doc(hidden)]
pub mod parse;
#[doc(hidden)]
pub mod raw;
#[doc(hidden)]
pub mod rollback;
#[doc(hidden)]
pub mod rule;
#[doc(hidden)]
pub mod source;
#[doc(hidden)]
pub mod target;
#[doc(hidden)]
pub mod validation;

pub use category::RuleCategory;
pub use error::RuleParseError;
pub use parse::{parse_rules_toml, parse_rules_toml_documents};
pub use rollback::RollbackStrategy;
pub use rule::{RuleDefinition, RuleDocument};
pub use source::RuleSource;
pub use target::RuleTarget;
