#![doc = "apply-profile 恢复安全边界测试支撑。"]
#![allow(
    clippy::redundant_pub_crate,
    reason = "integration test support helpers are imported from their parent test module"
)]

use std::error::Error;
use std::fs;
use std::os::unix::fs as unix_fs;

use crate::support::{
    ANDROID_FS_FIXTURE, TempFixture, appops_rules, assert_failed_without_runner_mutation,
    run_puread_with_profile_test,
};

pub(crate) fn apply_profile_state_symlink_is_rejected_before_android_mutation()
-> Result<(), Box<dyn Error>> {
    let fixture = TempFixture::new("apply-state-link")?;
    let outside = fixture.root().join("outside-state");
    fs::create_dir_all(&outside)?;
    fs::remove_dir_all(fixture.module_root().join("state"))?;
    unix_fs::symlink(&outside, fixture.module_root().join("state"))?;

    let output = run_puread_with_profile_test(
        [
            "apply-profile",
            "appops",
            "--execute",
            "--rules",
            appops_rules(),
            "--root",
            ANDROID_FS_FIXTURE,
            "--module-root",
            fixture.module_root_str(),
            "--test-profile-runner",
            "--profile-runner-log",
            fixture.runner_log_str(),
        ],
        &fixture,
    )?;

    assert_failed_without_runner_mutation(&output, &fixture)
}
