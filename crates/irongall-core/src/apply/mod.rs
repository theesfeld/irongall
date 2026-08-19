pub mod writers;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::backup::Session;
use crate::catalog::{self, CatalogEntry};
use crate::config::Config;
use crate::discovery::{self, AppState, Discovered, Effective};
use crate::error::{Error, Result};
use crate::font;
use crate::paths::{write_string, Paths};
use crate::scheme::{self, Scheme};

#[derive(Clone, Debug, Default)]
pub struct ApplyOptions {
    pub dry_run: bool,
    /// Restrict to a single catalog id.
    pub only: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ApplyRequest {
    pub theme: Option<String>,
    pub font: Option<String>,
    pub size: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct ApplyOutcome {
    pub theme: String,
    pub font: String,
    pub size: f32,
    pub rows: Vec<TargetRow>,
    pub failed: bool,
}

#[derive(Clone, Debug)]
pub struct TargetRow {
    pub id: String,
    pub status: TargetStatus,
}

#[derive(Clone, Debug)]
pub enum TargetStatus {
    Ok { detail: Option<String> },
    Skipped { reason: String },
    Failed { error: String },
    DryRun { summary: String },
}

impl TargetStatus {
    pub fn render(&self) -> String {
        match self {
            Self::Ok { detail: None } => "ok".into(),
            Self::Ok {
                detail: Some(d),
            } => format!("ok ({d})"),
            Self::Skipped { reason } => format!("skipped ({reason})"),
            Self::Failed { error } => format!("failed ({error})"),
            Self::DryRun { summary } => format!("dry-run ({summary})"),
        }
    }
}

impl ApplyOutcome {
    pub fn report(&self) -> String {
        let mut s = format!(
            "applied  theme={}  font={}  size={}\n",
            self.theme,
            self.font,
            crate::config::format_pt(self.size)
        );
        let width = self
            .rows
            .iter()
            .map(|r| r.id.len())
            .max()
            .unwrap_or(8);
        for r in &self.rows {
            s.push_str(&format!("  {:width$}  {}\n", r.id, r.status.render()));
        }
        s
    }
}

pub struct ApplyCtx<'a> {
    pub paths: &'a Paths,
    pub cfg: &'a Config,
    pub scheme: &'a Scheme,
    pub effective: Effective,
    pub dry_run: bool,
    pub session: &'a mut Session,
    pub diffs: Vec<String>,
}

impl ApplyCtx<'_> {
    pub fn write_file(&mut self, path: &std::path::Path, contents: &str) -> Result<()> {
        if self.dry_run {
            let old = fs::read_to_string(path).unwrap_or_default();
            if old == contents {
                self.diffs
                    .push(format!("{}: unchanged", path.display()));
            } else {
                self.diffs.push(format!(
                    "{}: {} → {} bytes",
                    path.display(),
                    old.len(),
                    contents.len()
                ));
            }
            return Ok(());
        }
        self.session.backup_file(path)?;
        write_string(path, contents)
    }

    pub fn patch_file(
        &mut self,
        path: &std::path::Path,
        body: &str,
        style: crate::markers::CommentStyle,
        create: bool,
    ) -> Result<bool> {
        if !path.exists() && !create {
            return Ok(false);
        }
        let old = if path.exists() {
            fs::read_to_string(path).map_err(|e| Error::io(e, path))?
        } else {
            String::new()
        };
        let new = crate::markers::patch(&old, body, style);
        if old == new {
            return Ok(true);
        }
        self.write_file(path, &new)?;
        Ok(true)
    }
}

/// Last successful write per app, used by `follow = false` (hold).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LastApplied {
    pub apps: BTreeMap<String, LastApp>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LastApp {
    pub theme: String,
    pub font: String,
    pub size: f32,
}

fn last_applied_path(paths: &Paths) -> PathBuf {
    paths.irongall_data.join("last-applied.json")
}

pub fn load_last(paths: &Paths) -> LastApplied {
    fs::read_to_string(last_applied_path(paths))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_last(paths: &Paths, last: &LastApplied) -> Result<()> {
    let raw = serde_json::to_string_pretty(last).map_err(|e| Error::parse("last-applied", e))?;
    write_string(&last_applied_path(paths), &raw)
}

pub fn apply(
    paths: &Paths,
    cfg: &mut Config,
    req: ApplyRequest,
    opts: ApplyOptions,
) -> Result<ApplyOutcome> {
    if !cfg!(target_os = "linux") {
        return Err(Error::user(
            "irongall apply is Linux-only (fontconfig, GTK, Qt, Hyprland, …)",
        ));
    }
    paths.ensure_dirs()?;

    if let Some(t) = &req.theme {
        let _ = scheme::load_named(paths, t)?;
        cfg.theme.name = t.clone();
    }
    if let Some(f) = &req.font {
        if !font::family_installed(f) {
            return Err(Error::user(format!(
                "font family '{f}' is not installed (import it or `irongall font install`)"
            )));
        }
        cfg.font.family = f.clone();
    }
    if let Some(sz) = req.size {
        cfg.font.size = sz;
    }
    if !opts.dry_run {
        cfg.save(paths)?;
    }

    let discovered = discovery::scan_and_cache(paths)?;
    let last = load_last(paths);
    let mut session = Session::begin(paths)?;
    let mut rows = Vec::new();
    let mut failed = false;
    let mut new_last = last.clone();

    let targets: Vec<(&CatalogEntry, &Discovered)> = catalog::all()
        .iter()
        .filter_map(|e| {
            if let Some(only) = &opts.only {
                if e.id != only.as_str() {
                    return None;
                }
            }
            let d = discovered.iter().find(|d| d.id == e.id)?;
            Some((e, d))
        })
        .collect();

    for (entry, disc) in targets {
        let mut eff = discovery::effective(cfg, entry, disc.present);
        if matches!(eff.state, AppState::Hold) {
            if let Some(prev) = last.apps.get(entry.id) {
                eff.theme = prev.theme.clone();
                eff.font = prev.font.clone();
                eff.size = prev.size;
            }
        }

        if eff.skip {
            rows.push(TargetRow {
                id: entry.id.into(),
                status: TargetStatus::Skipped {
                    reason: "skip".into(),
                },
            });
            continue;
        }
        if !disc.present {
            rows.push(TargetRow {
                id: entry.id.into(),
                status: TargetStatus::Skipped {
                    reason: "not installed".into(),
                },
            });
            continue;
        }
        if !entry.has_writer {
            rows.push(TargetRow {
                id: entry.id.into(),
                status: TargetStatus::Skipped {
                    reason: "no-writer".into(),
                },
            });
            continue;
        }

        let scheme = match scheme::load_named(paths, &eff.theme) {
            Ok(s) => s,
            Err(e) => {
                failed = true;
                rows.push(TargetRow {
                    id: entry.id.into(),
                    status: TargetStatus::Failed {
                        error: e.to_string(),
                    },
                });
                continue;
            }
        };

        let mut ctx = ApplyCtx {
            paths,
            cfg,
            scheme: &scheme,
            effective: eff.clone(),
            dry_run: opts.dry_run,
            session: &mut session,
            diffs: Vec::new(),
        };

        let status = match writers::apply_id(entry.id, &mut ctx) {
            Ok(s) => {
                if !opts.dry_run && matches!(s, TargetStatus::Ok { .. }) {
                    new_last.apps.insert(
                        entry.id.to_string(),
                        LastApp {
                            theme: eff.theme.clone(),
                            font: eff.font.clone(),
                            size: eff.size,
                        },
                    );
                }
                s
            }
            Err(e) => {
                failed = true;
                TargetStatus::Failed {
                    error: e.to_string(),
                }
            }
        };
        if matches!(status, TargetStatus::Failed { .. }) {
            failed = true;
        }
        rows.push(TargetRow {
            id: entry.id.into(),
            status,
        });
    }

    if !opts.dry_run {
        session.finish(paths)?;
        save_last(paths, &new_last)?;
    }

    let outcome = ApplyOutcome {
        theme: cfg.theme.name.clone(),
        font: cfg.font.family.clone(),
        size: cfg.font.size,
        rows,
        failed,
    };
    if failed {
        // Still return the report to the caller; they print it then exit 2.
        return Ok(outcome);
    }
    Ok(outcome)
}

pub fn linux_only() -> Result<()> {
    if cfg!(target_os = "linux") {
        Ok(())
    } else {
        Err(Error::user(
            "irongall apply is Linux-only (fontconfig, GTK, Qt, Hyprland, …)",
        ))
    }
}
