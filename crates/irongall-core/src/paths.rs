use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Filesystem locations irongall owns, plus the user's XDG dirs.
#[derive(Clone, Debug)]
pub struct Paths {
    pub home: PathBuf,
    pub config_home: PathBuf,
    pub data_home: PathBuf,
    pub irongall_config: PathBuf,
    pub irongall_data: PathBuf,
}

impl Paths {
    /// Resolve from the process environment. Tests inject via
    /// `IRONGALL_HOME` / `IRONGALL_CONFIG_DIR` / `IRONGALL_DATA_DIR`.
    pub fn resolve() -> Result<Self> {
        if let Ok(root) = env::var("IRONGALL_TEST_ROOT") {
            return Ok(Self::isolated(PathBuf::from(root)));
        }
        let home = env::var_os("IRONGALL_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(PathBuf::from))
            .or_else(dirs::home_dir)
            .ok_or_else(|| Error::user("cannot determine home directory"))?;

        let config_home = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        let data_home = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"));

        let irongall_config = env::var_os("IRONGALL_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| config_home.join("irongall"));
        let irongall_data = env::var_os("IRONGALL_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| data_home.join("irongall"));

        Ok(Self {
            home,
            config_home,
            data_home,
            irongall_config,
            irongall_data,
        })
    }

    /// Fully isolated tree under `root` (used by tests).
    pub fn isolated(root: PathBuf) -> Self {
        let config_home = root.join("config");
        let data_home = root.join("data");
        Self {
            home: root.clone(),
            irongall_config: config_home.join("irongall"),
            irongall_data: data_home.join("irongall"),
            config_home,
            data_home,
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.irongall_config.join("config.toml")
    }

    pub fn generated_dir(&self) -> PathBuf {
        self.irongall_config.join("generated")
    }

    pub fn schemes_dir(&self) -> PathBuf {
        self.irongall_data.join("schemes")
    }

    pub fn market_index(&self) -> PathBuf {
        self.irongall_data.join("market/index.json")
    }

    pub fn discovery_cache(&self) -> PathBuf {
        self.irongall_data.join("discovery.json")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.irongall_data.join("backups")
    }

    pub fn history_file(&self) -> PathBuf {
        self.irongall_data.join("history.json")
    }

    pub fn fonts_dir(&self) -> PathBuf {
        self.data_home.join("fonts/irongall")
    }

    pub fn expand(&self, glob: &str) -> PathBuf {
        if let Some(rest) = glob.strip_prefix("~/") {
            self.home.join(rest)
        } else if glob == "~" {
            self.home.clone()
        } else {
            PathBuf::from(glob)
        }
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        for dir in [
            &self.irongall_config,
            &self.irongall_data,
            &self.generated_dir(),
            &self.schemes_dir(),
            &self.backups_dir(),
            &self.fonts_dir(),
        ] {
            fs::create_dir_all(dir).map_err(|e| Error::io(e, dir))?;
        }
        Ok(())
    }
}

pub fn read_to_string(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| Error::io(e, path))
}

pub fn write_string(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io(e, parent))?;
    }
    fs::write(path, contents).map_err(|e| Error::io(e, path))
}

pub fn path_exists(path: &Path) -> bool {
    path.exists()
}
