#![doc = "`file_rule_integration` 测试规则 fixture。"]

/// 返回同时包含文件规则与非文件规则的集成测试 fixture。
pub const fn mixed_rules() -> &'static str {
    r#"
[[rules]]
id = "video-splash-cache"
category = "file_path"
package = "com.example.video"
action = "empty_dir"
target_template = "/sdcard/Android/data/<pkg>/splashCache"
risk_level = "low"
default_enabled = true
profile = "conservative"
observed_behavior = "app-local splash cache is recreated under external app storage"
rollback_strategy = "restore_original"
introduced_by = "task-17"
reviewed_at = "2026-06-17"

[rules.source]
source = "task-17-fixture"
source_file = "tests/file_rule_integration.rs"
source_line_or_pattern = "video-splash-cache"

[[rules]]
id = "video-ad-database"
category = "sqlite"
package = "com.example.video"
action = "minimal_sqlite"
target_template = "/data/data/<pkg>/databases/ad.db"
risk_level = "medium"
default_enabled = false
profile = "sqlite"
observed_behavior = "ad SDK stores state in SQLite"
rollback_strategy = "restore_original"
introduced_by = "task-17"
reviewed_at = "2026-06-17"

[rules.source]
source = "task-17-fixture"
source_file = "tests/file_rule_integration.rs"
source_line_or_pattern = "video-ad-database"

[[rules]]
id = "video-ad-component"
category = "component"
package = "com.example.video"
action = "disable_component"
target_component = "com.example.video/com.example.video.ads.AdActivity"
risk_level = "high"
default_enabled = false
profile = "component"
observed_behavior = "ad component is only allowed in explicit profile"
rollback_strategy = "reenable_component"
introduced_by = "task-17"
reviewed_at = "2026-06-17"

[rules.source]
source = "task-17-fixture"
source_file = "tests/file_rule_integration.rs"
source_line_or_pattern = "video-ad-component"

[[rules]]
id = "video-background-appop"
category = "appops"
package = "com.example.video"
action = "set_appop"
appop = "RUN_IN_BACKGROUND"
appop_mode = "ignore"
risk_level = "high"
default_enabled = false
profile = "appops"
observed_behavior = "appops rule is not watched at high frequency"
rollback_strategy = "restore_appop"
introduced_by = "task-17"
reviewed_at = "2026-06-17"

[rules.source]
source = "task-17-fixture"
source_file = "tests/file_rule_integration.rs"
source_line_or_pattern = "video-background-appop"
"#
}
