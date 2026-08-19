//! Keyboard-only TUI for irongall.

use std::cell::Cell;
use std::collections::HashSet;
use std::io::{self, stdout};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use irongall_core::apply::{self, ApplyOptions, ApplyRequest};
use irongall_core::catalog::CatalogEntry;
use irongall_core::config::{self, Config};
use irongall_core::discovery::{self, AppRow};
use irongall_core::error::{Error, Result};
use irongall_core::font::{self, FontFamily};
use irongall_core::market::{self, FontEntry, Index, SchemeEntry};
use irongall_core::paths::Paths;
use irongall_core::scheme::{self, Scheme};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Terminal;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pane {
    Themes,
    Fonts,
    Size,
    Apps,
    Market,
    Status,
}

impl Pane {
    const ALL: [Pane; 6] = [
        Pane::Themes,
        Pane::Fonts,
        Pane::Size,
        Pane::Apps,
        Pane::Market,
        Pane::Status,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Themes => "Themes",
            Self::Fonts => "Fonts",
            Self::Size => "Size",
            Self::Apps => "Apps",
            Self::Market => "Market",
            Self::Status => "Status",
        }
    }

    fn next(self) -> Self {
        let i = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    fn prev(self) -> Self {
        let i = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MarketTab {
    Schemes,
    Fonts,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shade {
    All,
    Dark,
    Light,
}

impl Shade {
    fn next(self) -> Self {
        match self {
            Self::All => Self::Dark,
            Self::Dark => Self::Light,
            Self::Light => Self::All,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    fn matches(self, dark: bool) -> bool {
        match self {
            Self::All => true,
            Self::Dark => dark,
            Self::Light => !dark,
        }
    }
}

#[derive(Clone, Copy)]
enum Hit {
    HeaderDark,
    HeaderLight,
    Item(usize),
}

impl Hit {
    fn item(self) -> Option<usize> {
        match self {
            Self::Item(i) => Some(i),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FontSlot {
    Family,
    Sans,
    Serif,
    Mono,
}

impl FontSlot {
    fn label(self) -> &'static str {
        match self {
            Self::Family => "family",
            Self::Sans => "sans",
            Self::Serif => "serif",
            Self::Mono => "mono",
        }
    }
}

enum Modal {
    None,
    Help,
    ConfirmApply,
    #[allow(dead_code)]
    Message(String),
    Filter,
    PickTheme { for_app: Option<String> },
    PickFont { for_app: Option<String> },
    PickSize { for_app: Option<String>, value: f32 },
}

struct App {
    paths: Paths,
    cfg: Config,
    pane: Pane,
    schemes: Vec<Scheme>,
    fonts: Vec<FontFamily>,
    apps: Vec<AppRow>,
    index: Index,
    theme_sel: usize,
    font_sel: usize,
    app_sel: usize,
    market_sel: usize,
    market_tab: MarketTab,
    show_missing: bool,
    show_nowriter: bool,
    filter: String,
    filtering: bool,
    size_draft: f32,
    modal: Modal,
    status_msg: String,
    last_report: String,
    installed_fonts: HashSet<String>,
    theme_hits: Vec<Hit>,
    font_hits: Vec<usize>,
    app_hits: Vec<usize>,
    market_hits: Vec<Hit>,
    shade: Shade,
    font_slot: FontSlot,
    list_h: Cell<u16>,
    quit: bool,
}

pub fn run() -> Result<()> {
    let paths = Paths::resolve()?;
    paths.ensure_dirs()?;
    let cfg = Config::load(&paths)?;
    let paths_f = paths.clone();
    let cfg_f = cfg.clone();
    let (schemes, fonts, apps, index) = std::thread::scope(|s| {
        let schemes = s.spawn(|| scheme::load_all(&paths_f));
        let fonts = s.spawn(|| font::list_installed());
        let apps = s.spawn(|| discovery::rows(&paths_f, &cfg_f, true));
        let index = s.spawn(|| market::load_index(&paths_f));
        (
            schemes.join().unwrap().unwrap_or_default(),
            fonts.join().unwrap().unwrap_or_default(),
            apps.join().unwrap().unwrap_or_default(),
            index.join().unwrap().unwrap_or(Index {
                version: 1,
                schemes: Vec::new(),
                fonts: Vec::new(),
            }),
        )
    });

    let theme_sel = schemes
        .iter()
        .position(|s| s.slug == cfg.theme.name)
        .unwrap_or(0);
    let font_sel = fonts
        .iter()
        .position(|f| f.family == cfg.font.family)
        .unwrap_or(0);

    let installed_fonts = fonts.iter().map(|f| f.family.clone()).collect();
    let mut app = App {
        size_draft: cfg.font.size,
        paths,
        cfg,
        pane: Pane::Themes,
        schemes,
        fonts,
        apps,
        index,
        theme_sel,
        font_sel,
        app_sel: 0,
        market_sel: 0,
        market_tab: MarketTab::Schemes,
        show_missing: false,
        show_nowriter: true,
        filter: String::new(),
        filtering: false,
        modal: Modal::None,
        status_msg: "Tab section · Enter set · a set+apply · A apply all · ? help · q quit".into(),
        last_report: String::new(),
        installed_fonts,
        theme_hits: Vec::new(),
        font_hits: Vec::new(),
        app_hits: Vec::new(),
        market_hits: Vec::new(),
        shade: Shade::All,
        font_slot: FontSlot::Family,
        list_h: Cell::new(12),
        quit: false,
    };
    rebuild_hits(&mut app);
    if let Some(pos) = app
        .theme_hits
        .iter()
        .position(|h| h.item() == Some(theme_sel))
    {
        app.theme_sel = pos;
    }

    enable_raw_mode().map_err(|e| Error::user(format!("terminal: {e}")))?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| Error::user(format!("terminal: {e}")))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| Error::user(format!("tui: {e}")))?;

    let res = event_loop(&mut terminal, &mut app);

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();
    res
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    terminal
        .draw(|f| draw(f, app))
        .map_err(|e| Error::user(format!("draw: {e}")))?;
    while !app.quit {
        match event::read() {
            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                handle(app, key)?;
                if app.quit {
                    break;
                }
                terminal
                    .draw(|f| draw(f, app))
                    .map_err(|e| Error::user(format!("draw: {e}")))?;
            }
            Ok(Event::Resize(_, _)) => {
                terminal
                    .draw(|f| draw(f, app))
                    .map_err(|e| Error::user(format!("draw: {e}")))?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn handle(app: &mut App, key: KeyEvent) -> Result<()> {
    if matches!(app.modal, Modal::Help) {
        if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q')) {
            app.modal = Modal::None;
        }
        return Ok(());
    }
    if let Modal::Message(_) = app.modal {
        app.modal = Modal::None;
        return Ok(());
    }
    if let Modal::ConfirmApply = app.modal {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                app.modal = Modal::None;
                do_apply(app, None)?;
            }
            KeyCode::Char('n') | KeyCode::Esc => app.modal = Modal::None,
            _ => {}
        }
        return Ok(());
    }
    if let Modal::PickSize { for_app, value } = &app.modal {
        let mut v = *value;
        let for_app = for_app.clone();
        match key.code {
            KeyCode::Left | KeyCode::Char('[') => v = (v - 0.5).max(8.0),
            KeyCode::Right | KeyCode::Char(']') => v = (v + 0.5).min(24.0),
            KeyCode::Enter => {
                if let Some(id) = for_app {
                    app.cfg.app_mut(&id).size = Some(v);
                    app.cfg.save(&app.paths)?;
                    refresh_apps(app);
                    app.status_msg = format!("{id} size → {}", config::format_pt(v));
                } else {
                    app.cfg.font.size = v;
                    app.size_draft = v;
                    app.cfg.save(&app.paths)?;
                    app.status_msg = format!("size → {}", config::format_pt(v));
                }
                app.modal = Modal::None;
                return Ok(());
            }
            KeyCode::Char('a') => {
                if let Some(id) = for_app {
                    app.cfg.app_mut(&id).size = Some(v);
                    app.cfg.save(&app.paths)?;
                    app.modal = Modal::None;
                    return do_apply(app, Some(id));
                } else {
                    app.cfg.font.size = v;
                    app.size_draft = v;
                    app.cfg.save(&app.paths)?;
                    app.modal = Modal::None;
                    return do_apply(app, None);
                }
            }
            KeyCode::Esc => {
                app.modal = Modal::None;
                return Ok(());
            }
            _ => {}
        }
        app.modal = Modal::PickSize { for_app, value: v };
        return Ok(());
    }

    if app.filtering || matches!(app.modal, Modal::Filter) {
        match key.code {
            KeyCode::Esc => {
                app.filtering = false;
                app.modal = Modal::None;
                app.filter.clear();
                rebuild_hits(app);
            }
            KeyCode::Enter => {
                app.filtering = false;
                app.modal = Modal::None;
            }
            KeyCode::Backspace => {
                app.filter.pop();
                rebuild_hits(app);
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.filter.push(c);
                rebuild_hits(app);
            }
            _ => {}
        }
        return Ok(());
    }

    match key.code {
        KeyCode::Char('q') => app.quit = true,
        KeyCode::Char('?') => app.modal = Modal::Help,
        KeyCode::Tab => {
            app.pane = app.pane.next();
            app.filter.clear();
            rebuild_hits(app);
        }
        KeyCode::BackTab => {
            app.pane = app.pane.prev();
            app.filter.clear();
            rebuild_hits(app);
        }
        KeyCode::Char('1') => app.pane = Pane::Themes,
        KeyCode::Char('2') => app.pane = Pane::Fonts,
        KeyCode::Char('3') => app.pane = Pane::Size,
        KeyCode::Char('4') => app.pane = Pane::Apps,
        KeyCode::Char('5') => app.pane = Pane::Market,
        KeyCode::Char('6') => app.pane = Pane::Status,
        KeyCode::Char('j') | KeyCode::Down => move_sel(app, 1),
        KeyCode::Char('k') | KeyCode::Up => move_sel(app, -1),
        KeyCode::PageDown => move_sel(app, app.page()),
        KeyCode::PageUp => move_sel(app, -app.page()),
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            move_sel(app, app.page());
        }
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            move_sel(app, -app.page());
        }
        KeyCode::Home => move_sel_to(app, 0),
        KeyCode::End => move_sel_to(app, usize::MAX),
        KeyCode::Char('/') => {
            app.filtering = true;
            app.modal = Modal::Filter;
        }
        KeyCode::Char('v') => {
            app.shade = app.shade.next();
            rebuild_hits(app);
            app.status_msg = format!("themes · {}", app.shade.label());
        }
        KeyCode::Char('A') => request_apply(app),
        KeyCode::Char('R') => {
            match irongall_core::backup::rollback(&app.paths) {
                Ok(done) => {
                    app.status_msg = format!("rolled back {} files", done.len());
                    app.last_report = done.join("\n");
                }
                Err(e) => app.status_msg = format!("rollback: {e}"),
            }
        }
        other => handle_pane(app, other)?,
    }
    Ok(())
}

fn handle_pane(app: &mut App, code: KeyCode) -> Result<()> {
    match app.pane {
        Pane::Themes => match code {
            KeyCode::Enter => set_theme(app, false)?,
            KeyCode::Char('a') => set_theme(app, true)?,
            _ => {}
        },
        Pane::Fonts => match code {
            KeyCode::Enter => set_font(app, false)?,
            KeyCode::Char('a') => set_font(app, true)?,
            KeyCode::Char('g') => {
                app.font_slot = FontSlot::Family;
                app.status_msg = "slot · family (all three unless overridden)".into();
            }
            KeyCode::Char('s') => {
                app.font_slot = FontSlot::Sans;
                app.status_msg = "slot · sans".into();
            }
            KeyCode::Char('e') => {
                app.font_slot = FontSlot::Serif;
                app.status_msg = "slot · serif".into();
            }
            KeyCode::Char('m') => {
                app.font_slot = FontSlot::Mono;
                app.status_msg = "slot · mono".into();
            }
            KeyCode::Char('c') => {
                match app.font_slot {
                    FontSlot::Family => {}
                    FontSlot::Sans => app.cfg.font.sans = None,
                    FontSlot::Serif => app.cfg.font.serif = None,
                    FontSlot::Mono => app.cfg.font.mono = None,
                }
                app.cfg.save(&app.paths)?;
                app.status_msg = format!("{} cleared (inherits family)", app.font_slot.label());
            }
            KeyCode::Char('i') => {
                if let Some(f) = current_font(app) {
                    match market::install_font(&app.paths, &f.family) {
                        Ok(p) => app.status_msg = format!("installed → {}", p.display()),
                        Err(e) => app.status_msg = format!("install: {e}"),
                    }
                }
            }
            _ => {}
        },
        Pane::Size => match code {
            KeyCode::Left | KeyCode::Char('[') => {
                app.size_draft = (app.size_draft - 0.5).max(8.0);
            }
            KeyCode::Right | KeyCode::Char(']') => {
                app.size_draft = (app.size_draft + 0.5).min(24.0);
            }
            KeyCode::Enter => {
                app.cfg.font.size = app.size_draft;
                app.cfg.save(&app.paths)?;
                app.status_msg = format!("size → {}", config::format_pt(app.size_draft));
            }
            KeyCode::Char('a') => {
                app.cfg.font.size = app.size_draft;
                app.cfg.save(&app.paths)?;
                do_apply(app, None)?;
            }
            _ => {}
        },
        Pane::Apps => handle_apps(app, code)?,
        Pane::Market => match code {
            KeyCode::Char('s') => {
                app.market_tab = MarketTab::Schemes;
                app.market_sel = 0;
                rebuild_hits(app);
            }
            KeyCode::Char('f') => {
                app.market_tab = MarketTab::Fonts;
                app.market_sel = 0;
                rebuild_hits(app);
            }
            KeyCode::Char('i') | KeyCode::Enter => market_install(app)?,
            KeyCode::Char('u') => match market::update(&app.paths, None) {
                Ok(idx) => {
                    app.status_msg = format!(
                        "index: {} schemes, {} fonts",
                        idx.schemes.len(),
                        idx.fonts.len()
                    );
                    app.index = idx;
                    rebuild_hits(app);
                }
                Err(e) => app.status_msg = format!("update: {e}"),
            },
            _ => {}
        },
        Pane::Status => match code {
            KeyCode::Enter | KeyCode::Char('a') => request_apply(app),
            _ => {}
        },
    }
    Ok(())
}

fn handle_apps(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Char('m') => {
            app.show_missing = !app.show_missing;
            rebuild_hits(app);
        }
        KeyCode::Char('w') => {
            app.show_nowriter = !app.show_nowriter;
            rebuild_hits(app);
        }
        KeyCode::Char('t') => {
            if let Some(id) = current_app_id(app) {
                app.modal = Modal::PickTheme {
                    for_app: Some(id),
                };
                app.pane = Pane::Themes;
            }
        }
        KeyCode::Char('f') => {
            if let Some(id) = current_app_id(app) {
                app.modal = Modal::PickFont {
                    for_app: Some(id),
                };
                app.pane = Pane::Fonts;
            }
        }
        KeyCode::Char('s') => {
            if let Some(row) = current_app(app) {
                let id = row.id.clone();
                let v = row.size.unwrap_or(app.cfg.font.size);
                app.modal = Modal::PickSize {
                    for_app: Some(id),
                    value: v,
                };
            }
        }
        KeyCode::Char('c') => {
            if let Some(id) = current_app_id(app) {
                app.cfg.reset_app(&id);
                app.cfg.save(&app.paths)?;
                refresh_apps(app);
                app.status_msg = format!("reset {id}");
            }
        }
        KeyCode::Char('x') => {
            if let Some(id) = current_app_id(app) {
                let ov = app.cfg.app_mut(&id);
                ov.skip = Some(!ov.is_skip());
                let skip = ov.is_skip();
                app.cfg.save(&app.paths)?;
                refresh_apps(app);
                app.status_msg = format!("{} skip={}", id, skip);
            }
        }
        KeyCode::Char('h') => {
            if let Some(id) = current_app_id(app) {
                let ov = app.cfg.app_mut(&id);
                ov.follow = Some(ov.is_hold()); // toggle: hold → follow, else hold
                let hold = ov.is_hold();
                app.cfg.save(&app.paths)?;
                refresh_apps(app);
                app.status_msg = format!("{} hold={}", id, hold);
            }
        }
        KeyCode::Char('a') => {
            if let Some(id) = current_app_id(app) {
                do_apply(app, Some(id))?;
            }
        }
        KeyCode::Enter => {
            if let Some(row) = current_app(app) {
                app.status_msg = format!("{} · {} · {}", row.name, row.state, row.kind);
            }
        }
        _ => {}
    }
    Ok(())
}

fn set_theme(app: &mut App, apply_now: bool) -> Result<()> {
    let Some(s) = app
        .theme_hits
        .get(app.theme_sel)
        .and_then(|h| h.item())
        .and_then(|i| app.schemes.get(i))
        .cloned()
    else {
        return Ok(());
    };
    if let Modal::PickTheme { for_app: Some(id) } = &app.modal {
        let id = id.clone();
        app.cfg.app_mut(&id).theme = Some(s.slug.clone());
        app.cfg.save(&app.paths)?;
        app.modal = Modal::None;
        app.pane = Pane::Apps;
        refresh_apps(app);
        app.status_msg = format!("{id} theme → {}", s.slug);
        if apply_now {
            do_apply(app, Some(id))?;
        }
        return Ok(());
    }
    app.cfg.theme.name = s.slug.clone();
    app.cfg.theme.variant = if s.palette.prefer_dark() {
        irongall_core::config::Variant::Dark
    } else {
        irongall_core::config::Variant::Light
    };
    app.cfg.save(&app.paths)?;
    app.status_msg = format!("theme → {}", s.slug);
    if apply_now {
        do_apply(app, None)?;
    }
    Ok(())
}

fn set_font(app: &mut App, apply_now: bool) -> Result<()> {
    let Some(family) = current_font(app).map(|f| f.family.clone()) else {
        return Ok(());
    };
    if let Modal::PickFont { for_app: Some(id) } = &app.modal {
        let id = id.clone();
        app.cfg.app_mut(&id).font = Some(family.clone());
        app.cfg.save(&app.paths)?;
        app.modal = Modal::None;
        app.pane = Pane::Apps;
        refresh_apps(app);
        app.status_msg = format!("{id} font → {family}");
        if apply_now {
            do_apply(app, Some(id))?;
        }
        return Ok(());
    }
    match app.font_slot {
        FontSlot::Family => app.cfg.font.family = family.clone(),
        FontSlot::Sans => app.cfg.font.sans = Some(family.clone()),
        FontSlot::Serif => app.cfg.font.serif = Some(family.clone()),
        FontSlot::Mono => app.cfg.font.mono = Some(family.clone()),
    }
    app.cfg.save(&app.paths)?;
    app.status_msg = format!("{} → {family}", app.font_slot.label());
    if apply_now {
        do_apply(app, None)?;
    }
    Ok(())
}

fn market_install(app: &mut App) -> Result<()> {
    match app.market_tab {
        MarketTab::Schemes => {
            if let Some(s) = current_market_scheme(app) {
                match market::install_scheme(&app.paths, &s.name) {
                    Ok(p) => {
                        app.status_msg = format!("scheme {} → {}", s.name, p.display());
                        app.schemes = scheme::load_all(&app.paths).unwrap_or_default();
                        rebuild_hits(app);
                    }
                    Err(e) => app.status_msg = format!("install: {e}"),
                }
            }
        }
        MarketTab::Fonts => {
            if let Some(f) = current_market_font(app) {
                match market::install_font(&app.paths, &f.family) {
                    Ok(p) => {
                        app.status_msg = format!("font {} → {}", f.family, p.display());
                        font::invalidate_cache();
                        app.fonts = font::list_installed().unwrap_or_default();
                        app.installed_fonts =
                            app.fonts.iter().map(|fam| fam.family.clone()).collect();
                        rebuild_hits(app);
                    }
                    Err(e) => app.status_msg = format!("install: {e}"),
                }
            }
        }
    }
    Ok(())
}

fn request_apply(app: &mut App) {
    app.modal = Modal::ConfirmApply;
}

fn do_apply(app: &mut App, only: Option<String>) -> Result<()> {
    match apply::apply(
        &app.paths,
        &mut app.cfg,
        ApplyRequest {
            theme: None,
            font: None,
            size: None,
        },
        ApplyOptions {
            dry_run: false,
            only,
        },
    ) {
        Ok(o) => {
            app.last_report = o.report();
            app.status_msg = format!(
                "applied {} / {} {}",
                o.rows
                    .iter()
                    .filter(|r| matches!(r.status, apply::TargetStatus::Ok { .. }))
                    .count(),
                o.rows.len(),
                if o.failed { "(partial)" } else { "" }
            );
            refresh_apps(app);
        }
        Err(e) => app.status_msg = format!("apply: {e}"),
    }
    Ok(())
}

fn refresh_apps(app: &mut App) {
    app.apps = discovery::rows(&app.paths, &app.cfg, true).unwrap_or_default();
    rebuild_hits(app);
}

impl App {
    fn page(&self) -> i32 {
        i32::from(self.list_h.get().saturating_sub(1).max(1))
    }
}

fn list_len(app: &App) -> usize {
    match app.pane {
        Pane::Themes => app.theme_hits.len(),
        Pane::Fonts => app.font_hits.len(),
        Pane::Apps => app.app_hits.len(),
        Pane::Market => app.market_hits.len(),
        Pane::Size | Pane::Status => 0,
    }
}

fn skip_headers(app: &mut App, dir: i32) {
    let hits: &[Hit] = match app.pane {
        Pane::Themes => &app.theme_hits,
        Pane::Market if matches!(app.market_tab, MarketTab::Schemes) => &app.market_hits,
        _ => return,
    };
    let n = hits.len();
    if n == 0 {
        return;
    }
    let sel = match app.pane {
        Pane::Themes => &mut app.theme_sel,
        Pane::Market => &mut app.market_sel,
        _ => return,
    };
    let mut i = *sel;
    let step = if dir >= 0 { 1isize } else { -1isize };
    for _ in 0..n {
        if hits[i].item().is_some() {
            *sel = i;
            return;
        }
        let next = i as isize + step;
        if next < 0 || next >= n as isize {
            break;
        }
        i = next as usize;
    }
    if let Some(pos) = hits.iter().position(|h| h.item().is_some()) {
        *sel = pos;
    }
}

fn move_sel_to(app: &mut App, idx: usize) {
    let n = list_len(app);
    if n == 0 {
        return;
    }
    let sel = match app.pane {
        Pane::Themes => &mut app.theme_sel,
        Pane::Fonts => &mut app.font_sel,
        Pane::Apps => &mut app.app_sel,
        Pane::Market => &mut app.market_sel,
        _ => return,
    };
    *sel = idx.min(n - 1);
    skip_headers(app, 1);
}

fn move_sel(app: &mut App, delta: i32) {
    let n = list_len(app);
    if n == 0 {
        return;
    }
    let sel = match app.pane {
        Pane::Themes => &mut app.theme_sel,
        Pane::Fonts => &mut app.font_sel,
        Pane::Apps => &mut app.app_sel,
        Pane::Market => &mut app.market_sel,
        _ => return,
    };
    let next = *sel as i32 + delta;
    *sel = next.clamp(0, n as i32 - 1) as usize;
    skip_headers(app, delta);
}

fn group_hits(dark: Vec<usize>, light: Vec<usize>, shade: Shade) -> Vec<Hit> {
    let mut out = Vec::new();
    match shade {
        Shade::Dark => out.extend(dark.into_iter().map(Hit::Item)),
        Shade::Light => out.extend(light.into_iter().map(Hit::Item)),
        Shade::All => {
            if !dark.is_empty() {
                out.push(Hit::HeaderDark);
                out.extend(dark.into_iter().map(Hit::Item));
            }
            if !light.is_empty() {
                out.push(Hit::HeaderLight);
                out.extend(light.into_iter().map(Hit::Item));
            }
        }
    }
    out
}

fn rebuild_hits(app: &mut App) {
    let q = app.filter.to_ascii_lowercase();
    let mut dark = Vec::new();
    let mut light = Vec::new();
    for (i, s) in app.schemes.iter().enumerate() {
        if !(q.is_empty()
            || s.slug.to_ascii_lowercase().contains(&q)
            || s.name.to_ascii_lowercase().contains(&q))
        {
            continue;
        }
        if !app.shade.matches(s.is_dark()) {
            continue;
        }
        if s.is_dark() {
            dark.push(i);
        } else {
            light.push(i);
        }
    }
    app.theme_hits = group_hits(dark, light, app.shade);

    app.font_hits = app
        .fonts
        .iter()
        .enumerate()
        .filter(|(_, f)| q.is_empty() || f.family.to_ascii_lowercase().contains(&q))
        .map(|(i, _)| i)
        .collect();
    app.app_hits = app
        .apps
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            if !app.show_missing && r.state == "missing" {
                return false;
            }
            if !app.show_nowriter && r.state == "no-writer" {
                return false;
            }
            q.is_empty() || r.id.contains(&q) || r.name.to_ascii_lowercase().contains(&q)
        })
        .map(|(i, _)| i)
        .collect();
    app.market_hits = match app.market_tab {
        MarketTab::Schemes => {
            let mut d = Vec::new();
            let mut l = Vec::new();
            for (i, s) in app.index.schemes.iter().enumerate() {
                if !(q.is_empty() || s.name.to_ascii_lowercase().contains(&q)) {
                    continue;
                }
                if !app.shade.matches(s.is_dark()) {
                    continue;
                }
                if s.is_dark() {
                    d.push(i);
                } else {
                    l.push(i);
                }
            }
            group_hits(d, l, app.shade)
        }
        MarketTab::Fonts => app
            .index
            .fonts
            .iter()
            .enumerate()
            .filter(|(_, f)| q.is_empty() || f.family.to_ascii_lowercase().contains(&q))
            .map(|(i, _)| Hit::Item(i))
            .collect(),
    };
    let clamp = |sel: &mut usize, n: usize| {
        if n == 0 {
            *sel = 0;
        } else if *sel >= n {
            *sel = n - 1;
        }
    };
    clamp(&mut app.theme_sel, app.theme_hits.len());
    clamp(&mut app.font_sel, app.font_hits.len());
    clamp(&mut app.app_sel, app.app_hits.len());
    clamp(&mut app.market_sel, app.market_hits.len());
    skip_headers(app, 1);
}

fn current_font(app: &App) -> Option<&FontFamily> {
    app.font_hits
        .get(app.font_sel)
        .and_then(|&i| app.fonts.get(i))
}

fn current_app(app: &App) -> Option<&AppRow> {
    app.app_hits.get(app.app_sel).and_then(|&i| app.apps.get(i))
}

fn current_app_id(app: &App) -> Option<String> {
    current_app(app).map(|r| r.id.clone())
}

fn current_market_scheme(app: &App) -> Option<&SchemeEntry> {
    app.market_hits
        .get(app.market_sel)
        .and_then(|h| h.item())
        .and_then(|i| app.index.schemes.get(i))
}

fn current_market_font(app: &App) -> Option<&FontEntry> {
    app.market_hits
        .get(app.market_sel)
        .and_then(|h| h.item())
        .and_then(|i| app.index.fonts.get(i))
}

fn selected_scheme(app: &App) -> Option<&Scheme> {
    let name = &app.cfg.theme.name;
    app.schemes
        .iter()
        .find(|s| s.slug == *name)
        .or(app.schemes.first())
}

fn browsing_scheme(app: &App) -> Option<&Scheme> {
    match app.pane {
        Pane::Themes => app
            .theme_hits
            .get(app.theme_sel)
            .and_then(|h| h.item())
            .and_then(|i| app.schemes.get(i)),
        _ => selected_scheme(app),
    }
}

fn theme_style(scheme: &Scheme) -> (Color, Color, Color, Color) {
    let p = &scheme.palette;
    let bg = rgb(p.base00());
    let fg = rgb(p.base05());
    let acc = rgb(p.accent());
    let dim = rgb(p.base03());
    (bg, fg, acc, dim)
}

fn rgb(c: irongall_core::Rgb) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

fn draw(f: &mut ratatui::Frame, app: &App) {
    let scheme = browsing_scheme(app);
    let (bg, fg, acc, dim) = scheme
        .map(theme_style)
        .unwrap_or((Color::Black, Color::White, Color::Magenta, Color::DarkGray));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(f.area());

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14),
            Constraint::Percentage(42),
            Constraint::Percentage(58),
        ])
        .split(chunks[0]);

    draw_nav(f, body[0], app, bg, fg, acc, dim);
    draw_center(f, body[1], app, bg, fg, acc, dim);
    draw_preview(f, body[2], app, bg, fg, acc, dim);
    draw_footer(f, chunks[1], app, bg, fg, acc);

    match &app.modal {
        Modal::Help => draw_help(f, bg, fg, acc),
        Modal::ConfirmApply => draw_confirm(f, app, bg, fg, acc),
        Modal::Message(m) => draw_msg(f, m, bg, fg, acc),
        Modal::PickSize { value, for_app } => {
            let who = for_app.as_deref().unwrap_or("global");
            draw_msg(
                f,
                &format!(
                    "{who} size  {}   ←/→  Enter set  a apply  Esc",
                    config::gtk_font_name(&app.cfg.font.family, *value)
                ),
                bg,
                fg,
                acc,
            );
        }
        _ => {}
    }
}

fn draw_nav(
    f: &mut ratatui::Frame,
    area: Rect,
    app: &App,
    bg: Color,
    fg: Color,
    acc: Color,
    dim: Color,
) {
    let items: Vec<ListItem> = Pane::ALL
        .iter()
        .map(|p| {
            let sel = *p == app.pane;
            let mark = if sel { "▸ " } else { "  " };
            let style = if sel {
                Style::default().fg(acc).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(fg)
            };
            ListItem::new(Line::from(Span::styled(format!("{mark}{}", p.label()), style)))
        })
        .collect();
    let list = List::new(items).block(
        Block::default()
            .title(" irongall ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(dim))
            .style(Style::default().bg(bg).fg(fg)),
    );
    f.render_widget(list, area);
}

fn draw_center(
    f: &mut ratatui::Frame,
    area: Rect,
    app: &App,
    bg: Color,
    fg: Color,
    acc: Color,
    dim: Color,
) {
    let title = match app.pane {
        Pane::Themes => match app.shade {
            Shade::All => " themes · all  (v) ",
            Shade::Dark => " themes · dark  (v) ",
            Shade::Light => " themes · light  (v) ",
        },
        Pane::Fonts => match app.font_slot {
            FontSlot::Family => " fonts · family  (g/s/e/m) ",
            FontSlot::Sans => " fonts · sans  (g/s/e/m) ",
            FontSlot::Serif => " fonts · serif  (g/s/e/m) ",
            FontSlot::Mono => " fonts · mono  (g/s/e/m) ",
        },
        Pane::Size => " size ",
        Pane::Apps => " apps ",
        Pane::Market => match app.market_tab {
            MarketTab::Schemes => match app.shade {
                Shade::All => " market · schemes · all  (v) ",
                Shade::Dark => " market · schemes · dark  (v) ",
                Shade::Light => " market · schemes · light  (v) ",
            },
            MarketTab::Fonts => " market · fonts (OFL / GitHub) ",
        },
        Pane::Status => " status ",
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if matches!(app.pane, Pane::Size | Pane::Status) {
            dim
        } else {
            acc
        }))
        .style(Style::default().bg(bg).fg(fg));
    let inner = block.inner(area);
    app.list_h.set(inner.height.max(1));
    f.render_widget(block, area);

    match app.pane {
        Pane::Themes => draw_windowed(f, inner, app.theme_hits.len(), app.theme_sel, acc, fg, bg, |i| {
            match app.theme_hits[i] {
                Hit::HeaderDark => "── dark ──".into(),
                Hit::HeaderLight => "── light ──".into(),
                Hit::Item(j) => {
                    let s = &app.schemes[j];
                    let src = match s.source {
                        scheme::SchemeSource::Vendored => "·",
                        scheme::SchemeSource::Installed => "+",
                    };
                    let cur = if s.slug == app.cfg.theme.name { "*" } else { " " };
                    format!("{cur}{src} {:<20} {}", s.slug, s.name)
                }
            }
        }),
        Pane::Fonts => draw_windowed(f, inner, app.font_hits.len(), app.font_sel, acc, fg, bg, |i| {
            let fam = &app.fonts[app.font_hits[i]];
            let cur = if fam.family == slot_value(app) {
                "*"
            } else {
                " "
            };
            format!("{cur} {}", fam.family)
        }),
        Pane::Size => {
            let text = format!(
                "\n   {}\n\n   {}\n\n   UI {}    term {}\n\n   ←/→ or [ ]  ·  8–24 pt  ·  Enter set  ·  a apply",
                config::format_pt(app.size_draft),
                config::gtk_font_name(&app.cfg.font.family, app.size_draft),
                config::format_pt(app.cfg.font.ui_size()),
                config::format_pt(app.cfg.font.terminal_size()),
            );
            f.render_widget(
                Paragraph::new(text)
                    .style(Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD)),
                inner,
            );
        }
        Pane::Apps => draw_windowed(f, inner, app.app_hits.len(), app.app_sel, acc, fg, bg, |i| {
            let r = &app.apps[app.app_hits[i]];
            format!(
                "{:<10} {:<12} {:<14} {:>4}",
                trunc(&r.id, 10),
                app_chip(r),
                trunc(&r.theme, 14),
                r.size
                    .map(config::format_pt)
                    .unwrap_or_else(|| "—".into())
            )
        }),
        Pane::Market => match app.market_tab {
            MarketTab::Schemes => {
                draw_windowed(f, inner, app.market_hits.len(), app.market_sel, acc, fg, bg, |i| {
                    match app.market_hits[i] {
                        Hit::HeaderDark => "── dark ──".into(),
                        Hit::HeaderLight => "── light ──".into(),
                        Hit::Item(j) => {
                            let s = &app.index.schemes[j];
                            format!("{}  [{}]", s.name, s.license)
                        }
                    }
                });
            }
            MarketTab::Fonts => {
                draw_windowed(f, inner, app.market_hits.len(), app.market_sel, acc, fg, bg, |i| {
                    let Some(j) = app.market_hits[i].item() else {
                        return String::new();
                    };
                    let fam = &app.index.fonts[j];
                    let inst = if app.installed_fonts.contains(&fam.family) {
                        "installed"
                    } else {
                        "market"
                    };
                    format!("{}  [{}]  {inst}", fam.family, fam.license)
                });
            }
        },
        Pane::Status => {
            let mut text = apply_plan(app);
            if !app.last_report.is_empty() {
                text.push_str("\nlast apply\n");
                text.push_str(&app.last_report);
            }
            if text.len() > 4000 {
                text.truncate(4000);
            }
            f.render_widget(
                Paragraph::new(text)
                    .style(Style::default().fg(fg).bg(bg))
                    .wrap(Wrap { trim: false }),
                inner,
            );
        }
    }
}

fn draw_windowed(
    f: &mut ratatui::Frame,
    area: Rect,
    n: usize,
    sel: usize,
    acc: Color,
    fg: Color,
    bg: Color,
    mut fmt: impl FnMut(usize) -> String,
) {
    if n == 0 || area.height == 0 {
        return;
    }
    let h = area.height as usize;
    let start = sel.saturating_sub(h / 2).min(n.saturating_sub(h));
    let end = (start + h).min(n);
    let items: Vec<ListItem> = (start..end)
        .map(|i| {
            let text = fmt(i);
            let header = text.starts_with("──");
            let style = if header {
                Style::default().fg(fg).bg(bg).add_modifier(Modifier::DIM)
            } else if i == sel {
                Style::default().bg(acc).fg(bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(fg).bg(bg)
            };
            ListItem::new(text).style(style)
        })
        .collect();
    f.render_widget(List::new(items), area);
}

fn draw_preview(
    f: &mut ratatui::Frame,
    area: Rect,
    app: &App,
    bg: Color,
    fg: Color,
    acc: Color,
    dim: Color,
) {
    let block = Block::default()
        .title(" preview ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(dim))
        .style(Style::default().bg(bg).fg(fg));
    let inner = block.inner(area);
    f.render_widget(block, area);

    match app.pane {
        Pane::Themes | Pane::Status => {
            if let Some(s) = browsing_scheme(app) {
                draw_scheme_preview(f, inner, s, bg);
            }
        }
        Pane::Fonts => {
            let highlighted = current_font(app).map(|f| f.family.as_str()).unwrap_or("—");
            let mark = |slot: FontSlot, name: &str, ov: bool| {
                let cur = if app.font_slot == slot { "*" } else { " " };
                let tag = if ov { "  override" } else { "  (family)" };
                format!("{cur}{:<6}  {name}{tag}", slot.label())
            };
            let text = format!(
                "highlighted  {highlighted}\n\n{}\n{}\n{}\n{}\n\ng family   s sans   e serif   m mono\nEnter set this slot   c clear override",
                mark(FontSlot::Family, &app.cfg.font.family, false),
                mark(FontSlot::Sans, app.cfg.font.sans(), app.cfg.font.sans.is_some()),
                mark(FontSlot::Serif, app.cfg.font.serif(), app.cfg.font.serif.is_some()),
                mark(FontSlot::Mono, app.cfg.font.mono(), app.cfg.font.mono.is_some()),
            );
            f.render_widget(
                Paragraph::new(text)
                    .style(Style::default().fg(fg).bg(bg))
                    .wrap(Wrap { trim: false }),
                inner,
            );
        }
        Pane::Size => {
            let text = format!(
                "GTK string\n  {}\n\nterminals use {}\nUI / GTK / Qt use {}",
                config::gtk_font_name(&app.cfg.font.family, app.size_draft),
                config::format_pt(app.cfg.font.terminal_size()),
                config::format_pt(app.cfg.font.ui_size()),
            );
            f.render_widget(
                Paragraph::new(text).style(Style::default().fg(fg).bg(bg)),
                inner,
            );
        }
        Pane::Apps => {
            if let Some(row) = current_app(app) {
                let files = CatalogEntry::get(&row.id)
                    .map(|e| e.config_globs.join("\n  "))
                    .unwrap_or_default();
                let text = format!(
                    "{}\n{}\nstate  {}\ntheme  {}\nfont   {}\nsize   {}\n\nfiles\n  {}\n\nkeys  t/f/s override  c reset  x skip  h hold  a apply this",
                    row.name,
                    row.kind,
                    row.state,
                    row.theme,
                    row.font,
                    row.size.map(config::format_pt).unwrap_or_else(|| "—".into()),
                    files
                );
                f.render_widget(
                    Paragraph::new(text)
                        .style(Style::default().fg(fg).bg(bg))
                        .wrap(Wrap { trim: false }),
                    inner,
                );
            }
        }
        Pane::Market => match app.market_tab {
            MarketTab::Schemes => {
                if let Some(s) = current_market_scheme(app) {
                    if let Some(local) = app.schemes.iter().find(|x| x.slug == s.name) {
                        draw_scheme_preview(f, inner, local, bg);
                    } else {
                        draw_hex_preview(f, inner, s, bg, fg);
                    }
                }
            }
            MarketTab::Fonts => {
                if let Some(fam) = current_market_font(app) {
                    let installed = if app.installed_fonts.contains(&fam.family) {
                        "installed"
                    } else {
                        "not installed"
                    };
                    let text = format!(
                        "{}\nlicense  {}\nsource   {}\n{}\n{installed}\n\ni / Enter  install",
                        fam.family,
                        fam.license,
                        fam.source,
                        fam.notes.clone().unwrap_or_default(),
                    );
                    f.render_widget(
                        Paragraph::new(text)
                            .style(Style::default().fg(fg).bg(bg))
                            .wrap(Wrap { trim: true }),
                        inner,
                    );
                }
            }
        },
    }
    let _ = acc;
}

fn draw_hex_preview(
    f: &mut ratatui::Frame,
    area: Rect,
    s: &SchemeEntry,
    bg: Color,
    fg: Color,
) {
    let colors: Vec<irongall_core::Rgb> = s
        .preview
        .iter()
        .filter_map(|h| irongall_core::Rgb::parse(h).ok())
        .collect();
    let src = s.source.as_deref().unwrap_or("tinted-theming");
    let author = s.author.as_deref().unwrap_or("—");
    let variant = s.variant.as_deref().unwrap_or("");
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!("{}  {}", s.name, variant),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!("author  {author}"),
        Style::default().fg(fg).bg(bg),
    )));
    lines.push(Line::from(Span::styled(
        format!("license {src} · {}", s.license),
        Style::default().fg(fg).bg(bg),
    )));
    lines.push(Line::from(""));
    if colors.is_empty() {
        lines.push(Line::from("no palette preview (install to preview)"));
    } else {
        let mut spans = Vec::new();
        for (i, c) in colors.iter().enumerate() {
            spans.push(Span::styled("  ", Style::default().bg(rgb(*c))));
            if i == 7 {
                spans.push(Span::raw(" "));
            }
        }
        lines.push(Line::from(spans));
        if colors.len() >= 16 {
            let p0 = colors[0];
            let p1 = colors[1];
            let p2 = colors[2];
            let p3 = colors[3];
            let p5 = colors[5];
            let p8 = colors[8];
            let pb = colors[11];
            let pe = colors[14];
            let pd = colors[13];
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(" {} ", s.name),
                Style::default().bg(rgb(p1)).fg(rgb(p5)),
            )));
            lines.push(Line::from(Span::styled(
                " // a comment about the window",
                Style::default().fg(rgb(p3)).bg(rgb(p0)),
            )));
            lines.push(Line::from(vec![
                Span::styled(" fn ", Style::default().fg(rgb(pe)).bg(rgb(p0))),
                Span::styled("main", Style::default().fg(rgb(pd)).bg(rgb(p0))),
                Span::styled("() {", Style::default().fg(rgb(p5)).bg(rgb(p0))),
            ]));
            lines.push(Line::from(vec![
                Span::styled("   print(", Style::default().fg(rgb(p5)).bg(rgb(p0))),
                Span::styled("\"hello\"", Style::default().fg(rgb(pb)).bg(rgb(p0))),
                Span::styled(")", Style::default().fg(rgb(p5)).bg(rgb(p0))),
            ]));
            lines.push(Line::from(Span::styled(
                "   error: something went wrong",
                Style::default().fg(rgb(p8)).bg(rgb(p0)),
            )));
            lines.push(Line::from(Span::styled(
                " selected line of text ",
                Style::default().bg(rgb(p2)).fg(rgb(p5)),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from("i / Enter  install   u  refresh from tinted-theming"));
    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(bg).fg(fg))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_scheme_preview(f: &mut ratatui::Frame, area: Rect, s: &Scheme, bg: Color) {
    let p = &s.palette;
    let keys = [
        p.base00(),
        p.base01(),
        p.base02(),
        p.base03(),
        p.base04(),
        p.base05(),
        p.base06(),
        p.base07(),
        p.base08(),
        p.base09(),
        p.base0a(),
        p.base0b(),
        p.base0c(),
        p.base0d(),
        p.base0e(),
        p.base0f(),
    ];
    let mut spans: Vec<Span> = Vec::new();
    for (i, c) in keys.iter().enumerate() {
        spans.push(Span::styled(
            "  ",
            Style::default().bg(rgb(*c)),
        ));
        if i == 7 {
            spans.push(Span::raw(" "));
        }
    }
    let fake = vec![
        Line::from(spans),
        Line::from(""),
        Line::from(Span::styled(
            format!(" {} ", s.name),
            Style::default().bg(rgb(p.base01())).fg(rgb(p.base05())),
        )),
        Line::from(Span::styled(
            " // a comment about the window",
            Style::default().fg(rgb(p.base03())).bg(bg),
        )),
        Line::from(vec![
            Span::styled(" fn ", Style::default().fg(rgb(p.base0e())).bg(bg)),
            Span::styled("main", Style::default().fg(rgb(p.base0d())).bg(bg)),
            Span::styled("() {", Style::default().fg(rgb(p.base05())).bg(bg)),
        ]),
        Line::from(vec![
            Span::styled("   print(", Style::default().fg(rgb(p.base05())).bg(bg)),
            Span::styled("\"hello\"", Style::default().fg(rgb(p.base0b())).bg(bg)),
            Span::styled(")", Style::default().fg(rgb(p.base05())).bg(bg)),
        ]),
        Line::from(Span::styled(
            "   error: something went wrong",
            Style::default().fg(rgb(p.base08())).bg(bg),
        )),
        Line::from(Span::styled(
            " selected line of text ",
            Style::default().bg(rgb(p.base02())).fg(rgb(p.base05())),
        )),
    ];
    f.render_widget(Paragraph::new(fake), area);
}

fn draw_footer(f: &mut ratatui::Frame, area: Rect, app: &App, bg: Color, fg: Color, acc: Color) {
    let filter = if app.filtering {
        format!("  /{}", app.filter)
    } else {
        String::new()
    };
    let text = format!(" {}{}", app.status_msg, filter);
    f.render_widget(
        Paragraph::new(text).style(Style::default().bg(bg).fg(acc).add_modifier(Modifier::BOLD)),
        area,
    );
    let _ = fg;
}

fn draw_help(f: &mut ratatui::Frame, bg: Color, fg: Color, acc: Color) {
    let area = centered(f.area(), 64, 20);
    f.render_widget(Clear, area);
    let text = "\
irongall\n\
  Tab / Shift-Tab   next / prev section\n\
  1–6               Themes Fonts Size Apps Market Status\n\
  j/k ↑↓            move\n\
  PgUp/PgDn Home/End  page / jump\n\
  v                 dark / light / all\n\
  /                 filter\n\
  Enter             set current (no apply)\n\
  a                 set + apply\n\
  A                 apply all\n\
  R                 rollback last session\n\
  q                 quit\n\
\n\
fonts pane\n\
  g/s/e/m           family / sans / serif / mono slot\n\
  Enter             set highlighted into that slot\n\
  c                 clear sans/serif/mono override\n\
\n\
apps pane\n\
  t/f/s             theme / font / size override\n\
  c reset   x skip   h hold   a apply this\n\
  m missing   w no-writer\n\
\n\
size                ←/→ or [ ] in 0.5 pt (8–24)\n\
market              s schemes  f fonts  i install  u update\n\
A                   confirm + list of writes";
    f.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .title(" help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(acc))
                .style(Style::default().bg(bg).fg(fg)),
        ),
        area,
    );
}

fn draw_confirm(f: &mut ratatui::Frame, app: &App, bg: Color, fg: Color, acc: Color) {
    let body = format!(
        "{}\n y / Enter  apply     n / Esc  cancel",
        apply_plan(app)
    );
    let h = (body.lines().count() as u16 + 2).clamp(8, f.area().height.saturating_sub(2));
    let w = f.area().width.saturating_sub(4).min(76).max(40);
    let area = centered(f.area(), w, h);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(body).block(
            Block::default()
                .title(" apply? ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(acc))
                .style(Style::default().bg(bg).fg(fg)),
        ).wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_msg(f: &mut ratatui::Frame, msg: &str, bg: Color, fg: Color, acc: Color) {
    let area = centered(f.area(), 60, 5);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(format!(" {msg}")).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(acc))
                .style(Style::default().bg(bg).fg(fg)),
        ),
        area,
    );
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

fn slot_value(app: &App) -> &str {
    match app.font_slot {
        FontSlot::Family => &app.cfg.font.family,
        FontSlot::Sans => app.cfg.font.sans(),
        FontSlot::Serif => app.cfg.font.serif(),
        FontSlot::Mono => app.cfg.font.mono(),
    }
}

fn app_chip(r: &AppRow) -> String {
    match r.state.as_str() {
        "skip" => "[skip]".into(),
        "hold" => "[hold]".into(),
        "tweak" => match r.size {
            Some(sz) => format!("[tweak {}]", config::format_pt(sz)),
            None => "[tweak]".into(),
        },
        "no-writer" => "[no-write]".into(),
        "missing" => "[gone]".into(),
        _ => "[global]".into(),
    }
}

fn apply_plan(app: &App) -> String {
    let c = discovery::counts(&app.apps);
    let mut s = format!(
        "theme  {}\nfamily {}\n  sans  {}{}\n  serif {}{}\n  mono  {}{}\nsize   {} pt\n\n{}\n\n",
        app.cfg.theme.name,
        app.cfg.font.family,
        app.cfg.font.sans(),
        if app.cfg.font.sans.is_some() { "  *" } else { "" },
        app.cfg.font.serif(),
        if app.cfg.font.serif.is_some() { "  *" } else { "" },
        app.cfg.font.mono(),
        if app.cfg.font.mono.is_some() { "  *" } else { "" },
        config::format_pt(app.cfg.font.size),
        c.one_line(),
    );
    for r in &app.apps {
        if r.state == "missing" {
            continue;
        }
        let files = CatalogEntry::get(&r.id)
            .map(|e| e.config_globs.join(" "))
            .unwrap_or_default();
        match r.state.as_str() {
            "skip" | "no-writer" => {
                s.push_str(&format!("  {:<12} {}\n", r.id, app_chip(r)));
            }
            _ => {
                s.push_str(&format!(
                    "  {:<12} {:<12} {:<16} {:>4}  {}\n",
                    r.id,
                    app_chip(r),
                    trunc(&r.theme, 16),
                    r.size
                        .map(config::format_pt)
                        .unwrap_or_else(|| "—".into()),
                    files,
                ));
            }
        }
    }
    s.push_str("\nA / y  writes only IRONGALL markers.  R  rollback.");
    s
}

fn trunc(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n.saturating_sub(1)])
    }
}
