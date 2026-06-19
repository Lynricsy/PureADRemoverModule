//! Android 路径模板的只读展开与危险路径拒绝。

mod error;
mod expander;
mod glob;
mod resolved;
mod template;
mod validation;

pub use error::PathExpansionError;
pub use expander::{ExpandedPath, PathExpander};
