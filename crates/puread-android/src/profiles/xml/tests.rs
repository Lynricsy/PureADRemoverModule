use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{commit_bool, plan_bool};
use crate::profiles::xml_hooks::test_hooks;

#[test]
fn commit_bool_rejects_regular_file_replacement_between_plan_and_commit() {
    // Given: a planned XML mutation and a hook that swaps the target regular file.
    let root = unique_temp_dir("identity-replace");
    let prefs = root.join("prefs.xml");
    let backup_dir = root.join("backups");
    fs::write(
        &prefs,
        r#"<map><boolean name="key_content_promotion" value="true" /></map>"#,
    )
    .expect("write prefs fixture");
    let plan = plan_bool(
        &prefs,
        "key_content_promotion",
        false,
        &backup_dir,
        "miui-weather-content-promotion",
    )
    .expect("plan XML bool mutation");
    test_hooks::set_before_commit_open(Box::new(|path| {
        fs::remove_file(path)?;
        fs::write(path, "<map></map>")
    }));

    // When: commit observes the same path after replacement.
    let error = commit_bool(&prefs, &backup_dir, &plan).expect_err("replacement rejected");

    // Then: replacement remains unmodified and no backup was created.
    assert!(error.to_string().contains("profile file I/O failed"));
    assert_eq!(
        fs::read_to_string(&prefs).expect("read replacement"),
        "<map></map>"
    );
    assert!(
        !backup_dir
            .join("miui-weather-content-promotion.xml.bak")
            .exists()
    );
}

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let path =
        std::env::temp_dir().join(format!("puread-xml-{name}-{}-{nanos}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("create temp dir");
    path
}
