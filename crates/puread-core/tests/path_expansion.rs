#![doc = "路径展开安全边界测试。"]

use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use puread_core::path_expansion::{ExpandedPath, PathExpander, PathExpansionError};

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    fn new() -> Result<Self, Box<dyn Error>> {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("puread-path-expansion-{}-{id}", std::process::id()));
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn mkdir(&self, android_path: &str) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(self.host_path(android_path))?;
        Ok(())
    }

    fn touch(&self, android_path: &str) -> Result<(), Box<dyn Error>> {
        let host_path = self.host_path(android_path);
        let Some(parent) = host_path.parent() else {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "fixture path has no parent").into(),
            );
        };
        fs::create_dir_all(parent)?;
        fs::write(host_path, b"fixture")?;
        Ok(())
    }

    fn host_path(&self, android_path: &str) -> PathBuf {
        let trimmed = android_path.trim_start_matches('/');
        self.path.join(trimmed)
    }

    fn expander(&self) -> Result<PathExpander, PathExpansionError> {
        PathExpander::new(&self.path, "/data/adb/modules/puread")
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn path_expansion_rejects_dangerous_paths_when_boundary_input_is_malformed()
-> Result<(), Box<dyn Error>> {
    // Given: paths that would be destructive if accepted as deletion targets.
    let root = TestRoot::new()?;
    let expander = root.expander()?;
    let dangerous = [
        "",
        "/",
        "/data",
        "/sdcard",
        "/storage",
        "/system",
        "/vendor",
        "/data/adb",
        "/data/adb/other-module/cache",
        "/data/data/com.example.app/../com.android.systemui",
        "data/data/com.example.app",
        "/*/data/com.example.app",
        "/data/*/com.example.app",
    ];

    // When / Then: each dangerous target is rejected before expansion.
    for template in dangerous {
        let result = expander.expand_template(template, "com.example.app");
        println!("rejected_path={template} result={result:?}");
        assert!(result.is_err(), "{template} should be rejected");
    }
    Ok(())
}

#[test]
fn path_expansion_expands_data_user_numeric_package_glob_when_users_exist()
-> Result<(), Box<dyn Error>> {
    // Given: Android multi-user data directories with one non-numeric decoy.
    let root = TestRoot::new()?;
    root.mkdir("/data/user/0/com.example.app/cache")?;
    root.mkdir("/data/user/10/com.example.app/cache")?;
    root.mkdir("/data/user/de/com.example.app/cache")?;
    root.mkdir("/data/user/0/com.other.app/cache")?;
    let expander = root.expander()?;

    // When: the controlled numeric user template is expanded for the package.
    let resolved = expander.expand_template("/data/user/[0-9]*/<pkg>/cache", "com.example.app")?;
    let android_paths = android_paths(&resolved);

    // Then: only numeric user directories for the requested package are returned.
    println!("expanded_data_user={android_paths:?}");
    assert_eq!(
        android_paths,
        vec![
            PathBuf::from("/data/user/0/com.example.app/cache"),
            PathBuf::from("/data/user/10/com.example.app/cache"),
        ]
    );
    Ok(())
}

#[test]
fn path_expansion_expands_android_data_and_package_relative_paths_when_package_exists()
-> Result<(), Box<dyn Error>> {
    // Given: an app-scoped external storage directory and a cache file under it.
    let root = TestRoot::new()?;
    root.touch("/sdcard/Android/data/com.example.app/cache/splash_ad")?;
    let expander = root.expander()?;

    // When: exact and package-relative expansion target the same package scope.
    let exact = expander.expand_template("/sdcard/Android/data/<pkg>", "com.example.app")?;
    let relative = expander.expand_package_relative("com.example.app", "cache/splash_ad")?;

    // Then: the controlled external storage paths are returned without broad sdcard deletion.
    println!(
        "expanded_android_data={:?} package_relative={:?}",
        android_paths(&exact),
        android_paths(&relative)
    );
    assert_eq!(
        android_paths(&exact),
        vec![PathBuf::from("/sdcard/Android/data/com.example.app")]
    );
    assert!(
        android_paths(&relative)
            .iter()
            .any(|path| path == Path::new("/sdcard/Android/data/com.example.app/cache/splash_ad"))
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn path_expansion_rejects_symlink_escape_when_target_points_outside_root()
-> Result<(), Box<dyn Error>> {
    // Given: a package-local path that is a symlink to a host path outside the fake Android root.
    let root = TestRoot::new()?;
    let outside = std::env::temp_dir().join(format!(
        "puread-path-expansion-outside-{}",
        std::process::id()
    ));
    if outside.exists() {
        fs::remove_dir_all(&outside)?;
    }
    fs::create_dir_all(&outside)?;
    root.mkdir("/data/data/com.example.app/cache")?;
    std::os::unix::fs::symlink(
        &outside,
        root.host_path("/data/data/com.example.app/cache/escape"),
    )?;
    let expander = root.expander()?;

    // When: expansion resolves the target.
    let result = expander.expand_template("/data/data/<pkg>/cache/escape", "com.example.app");

    // Then: symlink escape is rejected and the fixture outside root is cleaned up.
    println!("symlink_escape_result={result:?}");
    assert!(matches!(
        result,
        Err(PathExpansionError::SymlinkEscape { .. })
    ));
    fs::remove_dir_all(outside)?;
    Ok(())
}

#[test]
fn path_expansion_finds_name_matches_only_inside_package_scopes() -> Result<(), Box<dyn Error>> {
    // Given: matching file names inside and outside the requested package scope.
    let root = TestRoot::new()?;
    root.touch("/data/data/com.example.app/cache/GDTDOWNLOAD")?;
    root.touch("/data/data/com.other.app/cache/GDTDOWNLOAD")?;
    let expander = root.expander()?;

    // When: name-match expansion searches the requested package.
    let resolved = expander.expand_name_match("com.example.app", "GDTDOWNLOAD")?;

    // Then: only the requested package-owned match is returned.
    let android_paths = android_paths(&resolved);
    println!("name_matches={android_paths:?}");
    assert_eq!(
        android_paths,
        vec![PathBuf::from(
            "/data/data/com.example.app/cache/GDTDOWNLOAD"
        )]
    );
    Ok(())
}

fn android_paths(paths: &[ExpandedPath]) -> Vec<PathBuf> {
    let mut values = paths
        .iter()
        .map(|expanded| expanded.android_path().to_path_buf())
        .collect::<Vec<_>>();
    values.sort();
    values
}
