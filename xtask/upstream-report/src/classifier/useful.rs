use crate::classifier::{Findings, contains_any};
use crate::manifest::Category;

pub fn classify(path: &str, text: &str, findings: &mut Findings) {
    classify_path(path, findings);
    classify_content(text, findings);
}

fn classify_path(path: &str, findings: &mut Findings) {
    if contains_any(path, &["sqlite", ".db"]) {
        findings.add(
            Category::Sqlite,
            "path_sqlite",
            "path suggests local ad material",
        );
    }
    if path.contains("appops") {
        findings.add(
            Category::Appops,
            "path_appops",
            "path suggests local ad material",
        );
    }
    if contains_any(path, &["disable_app", "pm_disable", "component"]) {
        findings.add(
            Category::Component,
            "path_component",
            "path suggests local ad material",
        );
    }
    if contains_any(path, &["miui_ad", "rom", "settings"]) {
        findings.add(
            Category::RomProfile,
            "path_rom_profile",
            "path suggests local ad material",
        );
    }
    if contains_any(
        path,
        &["ad", "ads", "splash", "cache", "pangle", "gdt", "ttcache"],
    ) {
        findings.add(
            Category::FilePath,
            "path_local_ad_material",
            "path suggests local ad material",
        );
    }
    if contains_any(
        path,
        &[
            "pangle", "gdt", "ttcache", "beizi", "kwai", "anythink", "ksad",
        ],
    ) {
        findings.add(
            Category::SdkCache,
            "path_sdk_cache",
            "path suggests local ad material",
        );
    }
}

fn classify_content(text: &str, findings: &mut Findings) {
    if contains_any(
        text,
        &[
            "/data/data/",
            "/data/user/",
            "android/data",
            "block_ad",
            "rm -rf",
        ],
    ) {
        findings.add(
            Category::FilePath,
            "content_local_paths",
            "content suggests local ad material",
        );
    }
    if contains_any(
        text,
        &[
            "pangle",
            "gdtdownload",
            "ttcache",
            "beizi",
            "kwai",
            "anythink",
        ],
    ) {
        findings.add(
            Category::SdkCache,
            "content_sdk_cache",
            "content suggests local ad material",
        );
    }
    if contains_any(text, &["sqlite", "databases/", ".db"]) {
        findings.add(
            Category::Sqlite,
            "content_sqlite",
            "content suggests local ad material",
        );
    }
    if contains_any(text, &["appops", "cmd appops"]) {
        findings.add(
            Category::Appops,
            "content_appops",
            "content suggests local ad material",
        );
    }
    if contains_any(text, &["pm disable", "pm hide", "disable-user"]) {
        findings.add(
            Category::Component,
            "content_component",
            "content suggests local ad material",
        );
    }
    if contains_any(
        text,
        &[
            "settings put system",
            "settings put secure",
            "settings put global",
        ],
    ) {
        findings.add(
            Category::RomProfile,
            "content_rom_profile",
            "content suggests local ad material",
        );
    }
}
