//! Fake PATH + temp config dirs: kitty present, foot missing.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use irongall_core::catalog::CatalogEntry;
use irongall_core::config::{AppOverride, Config};
use irongall_core::discovery::{self, AppState};
use irongall_core::paths::Paths;
use tempfile::TempDir;

fn write_exe(path: &PathBuf) {
    fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[test]
fn kitty_present_foot_missing() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    write_exe(&bin.join("kitty"));
    // no foot binary

    let cfg_home = root.join("config");
    fs::create_dir_all(cfg_home.join("kitty")).unwrap();
    fs::write(cfg_home.join("kitty/kitty.conf"), "# kitty\n").unwrap();

    let orig_path = std::env::var("PATH").unwrap_or_default();
    let orig_home = std::env::var("HOME").ok();
    let orig_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("PATH", bin.display().to_string());
    std::env::set_var("IRONGALL_TEST_ROOT", root);
    std::env::set_var("XDG_CONFIG_HOME", &cfg_home);
    std::env::set_var("HOME", root);

    let paths = Paths::isolated(root.to_path_buf());
    let found = discovery::scan(&paths).unwrap();

    std::env::set_var("PATH", orig_path);
    std::env::remove_var("IRONGALL_TEST_ROOT");
    match orig_home {
        Some(h) => std::env::set_var("HOME", h),
        None => std::env::remove_var("HOME"),
    }
    match orig_xdg {
        Some(h) => std::env::set_var("XDG_CONFIG_HOME", h),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }

    let kitty = found.iter().find(|d| d.id == "kitty").unwrap();
    let foot = found.iter().find(|d| d.id == "foot").unwrap();
    assert!(kitty.present, "kitty should be present: {:?}", kitty.matched);
    assert!(!foot.present, "foot should be missing: {:?}", foot.matched);
}

#[test]
fn skip_means_writer_not_selected() {
    let mut cfg = Config::default();
    cfg.apps.insert(
        "kitty".into(),
        AppOverride {
            skip: Some(true),
            ..Default::default()
        },
    );
    let kitty = CatalogEntry::get("kitty").unwrap();
    let eff = discovery::effective(&cfg, kitty, true);
    assert_eq!(eff.state, AppState::Skip);
    assert!(eff.skip);
}
