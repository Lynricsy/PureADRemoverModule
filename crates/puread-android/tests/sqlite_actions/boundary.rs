use std::error::Error;
use std::fs;

use puread_android::sqlite_actions::SqliteActionTarget;

use super::{TestDir, missing_parent};

#[test]
fn sqlite_actions_reject_protected_and_non_package_targets_before_runner()
-> Result<(), Box<dyn Error>> {
    // Given: paths outside conservative Android app database directories.
    let dir = TestDir::new()?;
    let protected_host = dir.path().join("system/build.prop");
    let data_root = dir.path().join("data");
    let module_state = dir.adb_db_path("foo.db");
    let loose_db = dir.path().join("tmp/not-package.db");
    let app_cache_db = dir.cache_db_path("ad.db");
    let invalid_extension = dir.host_path("/data/data/com.example.video/databases/ad.txt");
    fs::create_dir_all(protected_host.parent().ok_or_else(missing_parent)?)?;
    fs::write(&protected_host, b"system")?;
    fs::create_dir_all(&data_root)?;
    fs::create_dir_all(module_state.parent().ok_or_else(missing_parent)?)?;
    fs::write(&module_state, b"module")?;
    fs::create_dir_all(loose_db.parent().ok_or_else(missing_parent)?)?;
    fs::write(&loose_db, b"loose")?;
    fs::create_dir_all(app_cache_db.parent().ok_or_else(missing_parent)?)?;
    fs::write(&app_cache_db, b"cache")?;
    fs::create_dir_all(invalid_extension.parent().ok_or_else(missing_parent)?)?;
    fs::write(&invalid_extension, b"text")?;

    // When / Then: construction rejects every unsafe target before execution exists.
    for (host, android) in [
        (&protected_host, "/system/build.prop"),
        (&data_root, "/data"),
        (&module_state, "/data/adb/foo.db"),
        (&loose_db, "/tmp/not-package.db"),
        (&app_cache_db, "/data/data/com.example.video/cache/ad.db"),
        (
            &invalid_extension,
            "/data/data/com.example.video/databases/ad.txt",
        ),
    ] {
        let result = SqliteActionTarget::from_android_path(android, host, dir.path());
        println!("sqlite_target_boundary android={android} result={result:?}");
        assert!(result.is_err(), "{android} should be rejected");
    }
    Ok(())
}

#[test]
fn sqlite_actions_reject_host_shaped_tmp_path_not_mapped_under_filesystem_root()
-> Result<(), Box<dyn Error>> {
    // Given: an attacker-controlled host path that merely contains an Android-shaped suffix.
    let dir = TestDir::new()?;
    let attacker_root =
        std::env::temp_dir().join(format!("puread-sqlite-attacker-{}", std::process::id()));
    let attacker_db = attacker_root.join("data/data/com.example.video/databases/ad.db");
    fs::create_dir_all(attacker_db.parent().ok_or_else(missing_parent)?)?;
    fs::write(&attacker_db, b"attacker")?;

    // When: the target is parsed through the public Android path boundary.
    let result = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/ad.db",
        &attacker_db,
        dir.path(),
    );

    // Then: host-shaped paths outside the explicit filesystem root are rejected.
    println!("sqlite_host_shaped_escape_result={result:?}");
    assert!(result.is_err());
    assert_eq!(fs::read(&attacker_db)?, b"attacker");
    fs::remove_dir_all(attacker_root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn sqlite_actions_reject_symlink_database_target_before_runner() -> Result<(), Box<dyn Error>> {
    // Given: an app database target that is a symlink.
    let dir = TestDir::new()?;
    let outside = dir.path().join("outside.db");
    fs::write(&outside, b"outside")?;
    let link = dir.db_path("ad.db");
    fs::create_dir_all(link.parent().ok_or_else(missing_parent)?)?;
    std::os::unix::fs::symlink(&outside, &link)?;

    // When: a safe SQLite target is constructed.
    let result = SqliteActionTarget::from_android_path(
        "/data/data/com.example.video/databases/ad.db",
        &link,
        dir.path(),
    );

    // Then: symlink targets are rejected before the runner can mutate them.
    println!("sqlite_symlink_target_result={result:?}");
    assert!(result.is_err());
    assert_eq!(fs::read(&outside)?, b"outside");
    Ok(())
}

#[test]
fn sqlite_actions_accept_conservative_app_database_target() -> Result<(), Box<dyn Error>> {
    // Given: a controlled host path mapped to an Android app database path.
    let dir = TestDir::new()?;
    let db_path = dir.nested_db_path(10, "ad.db");
    fs::create_dir_all(db_path.parent().ok_or_else(missing_parent)?)?;
    fs::write(&db_path, b"db")?;

    // When: the target is constructed through the safe boundary parser.
    let target = SqliteActionTarget::from_android_path(
        "/data/user/10/com.example.video/databases/ad.db",
        &db_path,
        dir.path(),
    )?;

    // Then: the host path is preserved for host-runnable execution.
    assert_eq!(target.path(), &db_path);
    Ok(())
}
