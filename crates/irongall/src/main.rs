use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use irongall_core::apply::{self, ApplyOptions, ApplyRequest};
use irongall_core::catalog::CatalogEntry;
use irongall_core::config::{self, Config};
use irongall_core::discovery::{self, AppState};
use irongall_core::error::Error;
use irongall_core::font;
use irongall_core::market;
use irongall_core::paths::Paths;
use irongall_core::scheme;

#[derive(Parser)]
#[command(
    name = "irongall",
    version,
    about = "One 16-color theme, one typeface, one font size — applied across Linux",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the TUI (default when no subcommand is given)
    Tui,
    /// Print global theme/font/size, fc-match, and an apps summary
    Status,
    /// Apply the current (or given) selection to every discovered program
    Apply {
        #[arg(long)]
        theme: Option<String>,
        #[arg(long)]
        font: Option<String>,
        #[arg(long)]
        size: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Restore files from the last apply session
    Rollback,
    /// Browse, preview, and apply color schemes
    #[command(subcommand)]
    Theme(ThemeCmd),
    /// Browse installed / market fonts
    #[command(subcommand)]
    Font(FontCmd),
    /// Global size
    #[command(subcommand)]
    Size(SizeCmd),
    /// Discover installed themable programs
    Apps {
        #[arg(long)]
        json: bool,
        /// Include programs that are not installed
        #[arg(long)]
        all: bool,
    },
    /// Per-program tweaks
    #[command(subcommand)]
    App(AppCmd),
    /// Marketplace index (no money)
    #[command(subcommand)]
    Market(MarketCmd),
    /// Print a 16-color ANSI preview without opening the TUI
    #[command(subcommand)]
    Preview(PreviewCmd),
    /// Generate shell completions
    Completions {
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum ThemeCmd {
    List,
    Show { name: String },
    Apply { name: String },
    Search { query: String },
    Install { name: String },
}

#[derive(Subcommand)]
enum FontCmd {
    List,
    Show { family: String },
    Apply { family: String },
    Search { query: String },
    Install { family: String },
    /// Copy a directory of fonts you already own into the user font dir
    Import { path: PathBuf },
}

#[derive(Subcommand)]
enum SizeCmd {
    Set { pt: String },
}

#[derive(Subcommand)]
enum AppCmd {
    List {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        all: bool,
    },
    Show {
        id: String,
    },
    Set {
        id: String,
        #[arg(long)]
        theme: Option<String>,
        #[arg(long)]
        font: Option<String>,
        #[arg(long)]
        size: Option<String>,
        #[arg(long, group = "follow_hold")]
        follow: bool,
        #[arg(long, group = "follow_hold")]
        hold: bool,
        #[arg(long)]
        dry_run: bool,
    },
    Reset {
        id: String,
    },
    Skip {
        id: String,
    },
}

#[derive(Subcommand)]
enum MarketCmd {
    Update {
        #[arg(long)]
        url: Option<String>,
    },
}

#[derive(Subcommand)]
enum PreviewCmd {
    Theme { name: String },
}

#[derive(Clone, Copy, ValueEnum)]
enum Unused {}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("irongall: {e}");
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

fn run() -> irongall_core::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Commands::Tui) => irongall_tui::run(),
        Some(Commands::Status) => cmd_status(),
        Some(Commands::Apply {
            theme,
            font,
            size,
            dry_run,
        }) => cmd_apply(theme, font, size, dry_run, None),
        Some(Commands::Rollback) => cmd_rollback(),
        Some(Commands::Theme(c)) => cmd_theme(c),
        Some(Commands::Font(c)) => cmd_font(c),
        Some(Commands::Size(SizeCmd::Set { pt })) => {
            let size = Some(pt);
            cmd_apply(None, None, size, false, None)
        }
        Some(Commands::Apps { json, all }) => cmd_apps(json, all),
        Some(Commands::App(c)) => cmd_app(c),
        Some(Commands::Market(MarketCmd::Update { url })) => cmd_market_update(url),
        Some(Commands::Preview(PreviewCmd::Theme { name })) => cmd_preview(&name),
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "irongall", &mut io::stdout());
            Ok(())
        }
    }
}

fn load() -> irongall_core::Result<(Paths, Config)> {
    let paths = Paths::resolve()?;
    paths.ensure_dirs()?;
    let cfg = Config::load(&paths)?;
    Ok((paths, cfg))
}

fn cmd_status() -> irongall_core::Result<()> {
    let (paths, cfg) = load()?;
    print!("{}", irongall_core::status_text(&paths, &cfg)?);
    Ok(())
}

fn cmd_apply(
    theme: Option<String>,
    font: Option<String>,
    size: Option<String>,
    dry_run: bool,
    only: Option<String>,
) -> irongall_core::Result<()> {
    let (paths, mut cfg) = load()?;
    let size = match size {
        Some(s) => Some(config::parse_pt(&s)?),
        None => None,
    };
    let outcome = apply::apply(
        &paths,
        &mut cfg,
        ApplyRequest { theme, font, size },
        ApplyOptions { dry_run, only },
    )?;
    print!("{}", outcome.report());
    if outcome.failed {
        return Err(Error::PartialApply);
    }
    Ok(())
}

fn cmd_rollback() -> irongall_core::Result<()> {
    let paths = Paths::resolve()?;
    let done = irongall_core::backup::rollback(&paths)?;
    if done.is_empty() {
        println!("nothing to restore");
    } else {
        for line in done {
            println!("{line}");
        }
    }
    Ok(())
}

fn cmd_theme(cmd: ThemeCmd) -> irongall_core::Result<()> {
    let (paths, mut cfg) = load()?;
    match cmd {
        ThemeCmd::List => {
            for s in scheme::load_all(&paths)? {
                let src = match s.source {
                    scheme::SchemeSource::Vendored => "vendored",
                    scheme::SchemeSource::Installed => "installed",
                };
                let var = s
                    .variant
                    .clone()
                    .unwrap_or_else(|| {
                        if s.palette.prefer_dark() {
                            "dark".into()
                        } else {
                            "light".into()
                        }
                    });
                println!(
                    "{:<22} {:<28} {:<6} {src}",
                    s.slug, s.name, var
                );
            }
        }
        ThemeCmd::Show { name } => {
            let s = scheme::load_named(&paths, &name)?;
            print!("{}", s.ansi_preview());
        }
        ThemeCmd::Apply { name } => {
            cmd_apply(Some(name), None, None, false, None)?;
        }
        ThemeCmd::Search { query } => {
            for s in scheme::search(&paths, &query)? {
                println!("{}  {}", s.slug, s.name);
            }
        }
        ThemeCmd::Install { name } => {
            let dest = market::install_scheme(&paths, &name)?;
            println!("installed {} → {}", name, dest.display());
            let _ = &mut cfg;
        }
    }
    Ok(())
}

fn cmd_font(cmd: FontCmd) -> irongall_core::Result<()> {
    let (paths, _cfg) = load()?;
    match cmd {
        FontCmd::List => {
            let list = font::list_installed()?;
            println!("# installed (main)");
            for f in list.iter().filter(|f| f.group == font::FontGroup::Main) {
                println!("{}", f.family);
            }
            println!("\n# other (emoji / cjk / nerd / last-resort)");
            for f in list.iter().filter(|f| f.group == font::FontGroup::Other) {
                println!("{}", f.family);
            }
        }
        FontCmd::Show { family } => {
            let list = font::list_installed()?;
            let found = list
                .iter()
                .find(|f| f.family.eq_ignore_ascii_case(&family))
                .ok_or_else(|| Error::user(format!("font '{family}' is not installed")))?;
            println!("family:  {}", found.family);
            println!("styles:  {}", found.styles.join(", "));
            println!("group:   {:?}", found.group);
        }
        FontCmd::Apply { family } => {
            cmd_apply(None, Some(family), None, false, None)?;
        }
        FontCmd::Search { query } => {
            for f in font::search_installed(&query)? {
                println!("{}", f.family);
            }
            if let Ok(idx) = market::load_index(&paths) {
                for f in market::search_fonts(&idx, &query) {
                    let inst = if font::family_installed(&f.family) {
                        "installed"
                    } else {
                        "market"
                    };
                    println!("{}  [{inst}  {}]", f.family, f.license);
                }
            }
        }
        FontCmd::Install { family } => {
            let dest = market::install_font(&paths, &family)?;
            println!("installed {} → {}", family, dest.display());
        }
        FontCmd::Import { path } => {
            let dest = font::import_dir(&paths, &path)?;
            println!("imported {} → {}", path.display(), dest.display());
        }
    }
    Ok(())
}

fn cmd_apps(json: bool, all: bool) -> irongall_core::Result<()> {
    let (paths, cfg) = load()?;
    let rows = discovery::rows(&paths, &cfg, all)?;
    if json {
        serde_json::to_writer_pretty(io::stdout(), &rows)
            .map_err(|e| Error::parse("json", e))?;
        println!();
    } else {
        print!("{}", discovery::format_table(&rows));
    }
    Ok(())
}

fn cmd_app(cmd: AppCmd) -> irongall_core::Result<()> {
    match cmd {
        AppCmd::List { json, all } => cmd_apps(json, all),
        AppCmd::Show { id } => cmd_app_show(&id),
        AppCmd::Set {
            id,
            theme,
            font,
            size,
            follow,
            hold,
            dry_run,
        } => cmd_app_set(&id, theme, font, size, follow, hold, dry_run),
        AppCmd::Reset { id } => {
            require_id(&id)?;
            let (paths, mut cfg) = load()?;
            cfg.reset_app(&id);
            cfg.save(&paths)?;
            println!("reset {id} — now inherits global");
            Ok(())
        }
        AppCmd::Skip { id } => {
            require_id(&id)?;
            let (paths, mut cfg) = load()?;
            cfg.app_mut(&id).skip = Some(true);
            cfg.save(&paths)?;
            println!("skip {id} — irongall will not touch its files");
            Ok(())
        }
    }
}

fn require_id(id: &str) -> irongall_core::Result<&'static CatalogEntry> {
    CatalogEntry::get(id).ok_or_else(|| {
        Error::user(format!(
            "unknown app id '{id}' — see `irongall app list --all`"
        ))
    })
}

fn cmd_app_show(id: &str) -> irongall_core::Result<()> {
    let entry = require_id(id)?;
    let (paths, cfg) = load()?;
    let discovered = discovery::scan(&paths)?;
    let disc = discovered.iter().find(|d| d.id == id);
    let present = disc.map(|d| d.present).unwrap_or(false);
    let eff = discovery::effective(&cfg, entry, present);
    println!("id:       {}", entry.id);
    println!("name:     {}", entry.name);
    println!("kind:     {}", entry.kind.as_str());
    println!("state:    {}", eff.state.as_str());
    println!("present:  {present}");
    println!("writer:   {}", entry.has_writer);
    println!("theme:    {}", eff.theme);
    println!("font:     {}", eff.font);
    println!("size:     {}", config::format_pt(eff.size));
    println!("knobs:    theme={} font={} size={}", entry.theme, entry.font, entry.size);
    if let Some(d) = disc {
        if !d.matched.is_empty() {
            println!("matched:");
            for m in &d.matched {
                println!("  {m}");
            }
        }
    }
    println!("files:");
    for g in entry.config_globs {
        println!("  {}", g);
    }
    if let Some(ov) = cfg.app(id) {
        println!("override:");
        if let Some(t) = &ov.theme {
            println!("  theme = {t}");
        }
        if let Some(f) = &ov.font {
            println!("  font  = {f}");
        }
        if let Some(s) = ov.size {
            println!("  size  = {}", config::format_pt(s));
        }
        if ov.is_hold() {
            println!("  follow = false (hold)");
        }
        if ov.is_skip() {
            println!("  skip = true");
        }
    }
    let _ = AppState::Global;
    Ok(())
}

fn cmd_app_set(
    id: &str,
    theme: Option<String>,
    fontf: Option<String>,
    size: Option<String>,
    follow: bool,
    hold: bool,
    dry_run: bool,
) -> irongall_core::Result<()> {
    require_id(id)?;
    let (paths, mut cfg) = load()?;
    {
        let ov = cfg.app_mut(id);
        if let Some(t) = theme {
            let _ = scheme::load_named(&paths, &t)?;
            ov.theme = Some(t);
        }
        if let Some(f) = fontf {
            if !font::family_installed(&f) {
                return Err(Error::user(format!("font family '{f}' is not installed")));
            }
            ov.font = Some(f);
        }
        if let Some(s) = size {
            ov.size = Some(config::parse_pt(&s)?);
        }
        if follow {
            ov.follow = Some(true);
        }
        if hold {
            ov.follow = Some(false);
        }
    }
    if dry_run {
        println!("dry-run: would write [apps.{id}] and apply only {id}");
        return cmd_apply(None, None, None, true, Some(id.to_string()));
    }
    cfg.save(&paths)?;
    cmd_apply(None, None, None, false, Some(id.to_string()))
}

fn cmd_market_update(url: Option<String>) -> irongall_core::Result<()> {
    let (paths, cfg) = load()?;
    let url = url
        .or_else(|| cfg.market.index_url.clone())
        .unwrap_or_else(|| market::DEFAULT_INDEX_URL.to_string());
    match market::update(&paths, Some(&url)) {
        Ok(idx) => {
            println!(
                "updated index from {url}\n  {} schemes  {} fonts",
                idx.schemes.len(),
                idx.fonts.len()
            );
            Ok(())
        }
        Err(e) => {
            // Offline: keep the bundled index usable.
            writeln!(
                io::stderr(),
                "irongall: market update failed ({e}); using bundled index"
            )
            .ok();
            let idx = market::bundled_index()?;
            println!(
                "bundled index\n  {} schemes  {} fonts",
                idx.schemes.len(),
                idx.fonts.len()
            );
            Ok(())
        }
    }
}

fn cmd_preview(name: &str) -> irongall_core::Result<()> {
    let (paths, _) = load()?;
    let s = scheme::load_named(&paths, name)?;
    print!("{}", s.ansi_preview());
    Ok(())
}
