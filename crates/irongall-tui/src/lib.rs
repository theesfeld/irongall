//! Keyboard-only TUI for irongall.

use std::io::{self, stdout};
use std::time::Duration;

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

enum Modal {
    None,
    Help,
    ConfirmApply { files: usize },
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
    quit: bool,
}

pub fn run() -> Result<()> {
    let paths = Paths::resolve()?;
    paths.ensure_dirs()?;
    let cfg = Config::load(&paths)?;
    let schemes = scheme::load_all(&paths)?;
    let fonts = font::list_installed().unwrap_or_default();
    let apps = discovery::rows(&paths, &cfg, true).unwrap_or_default();
    let index = market::load_index(&paths).unwrap_or(Index {
        version: 1,
        schemes: Vec::new(),
        fonts: Vec::new(),
    });

    let theme_sel = schemes
        .iter()
        .position(|s| s.slug == cfg.theme.name)
        .unwrap_or(0);
    let font_sel = fonts
        .iter()
        .position(|f| f.family == cfg.font.family)
        .unwrap_or(0);

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
        status_msg: "Enter set · a set+apply · A apply all · ? help · q quit".into(),
        last_report: String::new(),
        quit: false,
    };

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
    while !app.quit {
        terminal
            .draw(|f| draw(f, app))
            .map_err(|e| Error::user(format!("draw: {e}")))?;
        if event::poll(Duration::from_millis(200)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press {
                    handle(app, key)?;
                }
            }
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
    if let Modal::ConfirmApply { .. } = app.modal {
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
            }
            KeyCode::Enter => {
                app.filtering = false;
                app.modal = Modal::None;
            }
            KeyCode::Backspace => {
                app.filter.pop();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.filter.push(c);
            }
            _ => {}
        }
        return Ok(());
    }

    match key.code {
        KeyCode::Char('q') => app.quit = true,
        KeyCode::Char('?') => app.modal = Modal::Help,
        KeyCode::Tab | KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.pane = app.pane.next();
        }
        KeyCode::BackTab => app.pane = app.pane.prev(),
        KeyCode::Char('1') => app.pane = Pane::Themes,
        KeyCode::Char('2') => app.pane = Pane::Fonts,
        KeyCode::Char('3') => app.pane = Pane::Size,
        KeyCode::Char('4') => app.pane = Pane::Apps,
        KeyCode::Char('5') => app.pane = Pane::Market,
        KeyCode::Char('6') => app.pane = Pane::Status,
        KeyCode::Char('j') | KeyCode::Down => move_sel(app, 1),
        KeyCode::Char('k') | KeyCode::Up => move_sel(app, -1),
        KeyCode::Char('/') => {
            app.filtering = true;
            app.modal = Modal::Filter;
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
            }
            KeyCode::Char('f') => {
                app.market_tab = MarketTab::Fonts;
                app.market_sel = 0;
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
            clamp_app_sel(app);
        }
        KeyCode::Char('w') => {
            app.show_nowriter = !app.show_nowriter;
            clamp_app_sel(app);
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
    let Some(s) = filtered_schemes(app).get(app.theme_sel).cloned() else {
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
    let Some(f) = current_font(app) else {
        return Ok(());
    };
    if let Modal::PickFont { for_app: Some(id) } = &app.modal {
        let id = id.clone();
        app.cfg.app_mut(&id).font = Some(f.family.clone());
        app.cfg.save(&app.paths)?;
        app.modal = Modal::None;
        app.pane = Pane::Apps;
        refresh_apps(app);
        app.status_msg = format!("{id} font → {}", f.family);
        if apply_now {
            do_apply(app, Some(id))?;
        }
        return Ok(());
    }
    app.cfg.font.family = f.family.clone();
    app.cfg.save(&app.paths)?;
    app.status_msg = format!("font → {}", f.family);
    if apply_now {
        do_apply(app, None)?;
    }
    Ok(())
}

fn current_font(app: &App) -> Option<FontFamily> {
    filtered_fonts(app).get(app.font_sel).cloned()
}

fn market_install(app: &mut App) -> Result<()> {
    match app.market_tab {
        MarketTab::Schemes => {
            if let Some(s) = filtered_market_schemes(app).get(app.market_sel) {
                match market::install_scheme(&app.paths, &s.name) {
                    Ok(p) => {
                        app.status_msg = format!("scheme {} → {}", s.name, p.display());
                        app.schemes = scheme::load_all(&app.paths).unwrap_or_default();
                    }
                    Err(e) => app.status_msg = format!("install: {e}"),
                }
            }
        }
        MarketTab::Fonts => {
            if let Some(f) = filtered_market_fonts(app).get(app.market_sel) {
                match market::install_font(&app.paths, &f.family) {
                    Ok(p) => {
                        app.status_msg = format!("font {} → {}", f.family, p.display());
                        app.fonts = font::list_installed().unwrap_or_default();
                    }
                    Err(e) => app.status_msg = format!("install: {e}"),
                }
            }
        }
    }
    Ok(())
}

fn request_apply(app: &mut App) {
    let n = app
        .apps
        .iter()
        .filter(|a| matches!(a.state.as_str(), "global" | "tweak" | "hold"))
        .count();
    if n > 8 {
        app.modal = Modal::ConfirmApply { files: n };
    } else {
        let _ = do_apply(app, None);
    }
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
    clamp_app_sel(app);
}

fn clamp_app_sel(app: &mut App) {
    let n = visible_apps(app).len();
    if n == 0 {
        app.app_sel = 0;
    } else if app.app_sel >= n {
        app.app_sel = n - 1;
    }
}

fn move_sel(app: &mut App, delta: i32) {
    let n = match app.pane {
        Pane::Themes => filtered_schemes(app).len(),
        Pane::Fonts => filtered_fonts(app).len(),
        Pane::Apps => visible_apps(app).len(),
        Pane::Market => match app.market_tab {
            MarketTab::Schemes => filtered_market_schemes(app).len(),
            MarketTab::Fonts => filtered_market_fonts(app).len(),
        },
        Pane::Size | Pane::Status => 0,
    };
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
}

fn filtered_schemes(app: &App) -> Vec<Scheme> {
    let q = app.filter.to_ascii_lowercase();
    app.schemes
        .iter()
        .filter(|s| {
            q.is_empty()
                || s.slug.to_ascii_lowercase().contains(&q)
                || s.name.to_ascii_lowercase().contains(&q)
        })
        .cloned()
        .collect()
}

fn filtered_fonts(app: &App) -> Vec<FontFamily> {
    let q = app.filter.to_ascii_lowercase();
    app.fonts
        .iter()
        .filter(|f| q.is_empty() || f.family.to_ascii_lowercase().contains(&q))
        .cloned()
        .collect()
}

fn visible_apps(app: &App) -> Vec<AppRow> {
    let q = app.filter.to_ascii_lowercase();
    app.apps
        .iter()
        .filter(|r| {
            if !app.show_missing && r.state == "missing" {
                return false;
            }
            if !app.show_nowriter && r.state == "no-writer" {
                return false;
            }
            q.is_empty()
                || r.id.contains(&q)
                || r.name.to_ascii_lowercase().contains(&q)
        })
        .cloned()
        .collect()
}

fn current_app(app: &App) -> Option<AppRow> {
    visible_apps(app).get(app.app_sel).cloned()
}

fn current_app_id(app: &App) -> Option<String> {
    current_app(app).map(|r| r.id)
}

fn filtered_market_schemes(app: &App) -> Vec<SchemeEntry> {
    let q = app.filter.to_ascii_lowercase();
    app.index
        .schemes
        .iter()
        .filter(|s| q.is_empty() || s.name.to_ascii_lowercase().contains(&q))
        .cloned()
        .collect()
}

fn filtered_market_fonts(app: &App) -> Vec<FontEntry> {
    let q = app.filter.to_ascii_lowercase();
    app.index
        .fonts
        .iter()
        .filter(|f| q.is_empty() || f.family.to_ascii_lowercase().contains(&q))
        .cloned()
        .collect()
}

fn selected_scheme<'a>(app: &'a App) -> Option<&'a Scheme> {
    let name = &app.cfg.theme.name;
    app.schemes.iter().find(|s| s.slug == *name).or(app.schemes.first())
}

fn browsing_scheme(app: &App) -> Option<Scheme> {
    match app.pane {
        Pane::Themes => filtered_schemes(app).get(app.theme_sel).cloned(),
        _ => selected_scheme(app).cloned(),
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
        .as_ref()
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
        Modal::ConfirmApply { files } => draw_confirm(f, *files, bg, fg, acc),
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
        Pane::Themes => " themes ",
        Pane::Fonts => " fonts ",
        Pane::Size => " size ",
        Pane::Apps => " apps ",
        Pane::Market => match app.market_tab {
            MarketTab::Schemes => " market · schemes ",
            MarketTab::Fonts => " market · fonts ",
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
    f.render_widget(block, area);

    match app.pane {
        Pane::Themes => draw_list(
            f,
            inner,
            &filtered_schemes(app)
                .iter()
                .map(|s| {
                    let src = match s.source {
                        scheme::SchemeSource::Vendored => "·",
                        scheme::SchemeSource::Installed => "+",
                    };
                    let cur = if s.slug == app.cfg.theme.name { "*" } else { " " };
                    format!("{cur}{src} {:<20} {}", s.slug, s.name)
                })
                .collect::<Vec<_>>(),
            app.theme_sel,
            acc,
            fg,
            bg,
        ),
        Pane::Fonts => draw_list(
            f,
            inner,
            &filtered_fonts(app)
                .iter()
                .map(|fam| {
                    let cur = if fam.family == app.cfg.font.family {
                        "*"
                    } else {
                        " "
                    };
                    format!("{cur} {}", fam.family)
                })
                .collect::<Vec<_>>(),
            app.font_sel,
            acc,
            fg,
            bg,
        ),
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
        Pane::Apps => {
            let rows = visible_apps(app);
            let lines: Vec<String> = rows
                .iter()
                .map(|r| {
                    format!(
                        "{:<10} {:<10} {:<10} {:<16} {}",
                        trunc(&r.id, 10),
                        r.kind,
                        r.state,
                        trunc(&r.theme, 16),
                        r.size
                            .map(config::format_pt)
                            .unwrap_or_else(|| "—".into())
                    )
                })
                .collect();
            draw_list(f, inner, &lines, app.app_sel, acc, fg, bg);
        }
        Pane::Market => match app.market_tab {
            MarketTab::Schemes => {
                let lines: Vec<String> = filtered_market_schemes(app)
                    .iter()
                    .map(|s| format!("{}  [{}]", s.name, s.license))
                    .collect();
                draw_list(f, inner, &lines, app.market_sel, acc, fg, bg);
            }
            MarketTab::Fonts => {
                let lines: Vec<String> = filtered_market_fonts(app)
                    .iter()
                    .map(|fam| {
                        let inst = if font::family_installed(&fam.family) {
                            "installed"
                        } else {
                            "market"
                        };
                        format!("{}  [{}]  {inst}", fam.family, fam.license)
                    })
                    .collect();
                draw_list(f, inner, &lines, app.market_sel, acc, fg, bg);
            }
        },
        Pane::Status => {
            let all = discovery::rows(&app.paths, &app.cfg, true).unwrap_or_default();
            let c = discovery::counts(&all);
            let mut text = format!(
                "theme  {}\nfont   {}\nsize   {} pt\n\n{}\n\n{}",
                app.cfg.theme.name,
                app.cfg.font.family,
                config::format_pt(app.cfg.font.size),
                c.one_line(),
                app.last_report
            );
            if text.len() > 2000 {
                text.truncate(2000);
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

fn draw_list(
    f: &mut ratatui::Frame,
    area: Rect,
    items: &[String],
    sel: usize,
    acc: Color,
    fg: Color,
    bg: Color,
) {
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if i == sel {
                Style::default().bg(acc).fg(bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(fg).bg(bg)
            };
            ListItem::new(s.as_str()).style(style)
        })
        .collect();
    f.render_widget(List::new(list_items), area);
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
                draw_scheme_preview(f, inner, &s, bg);
            }
        }
        Pane::Fonts => {
            if let Some(fam) = current_font(app) {
                let text = format!(
                    "{}\n\nstyles: {}\n\n{}\n\n(rendered in the current terminal font — apply to switch)",
                    fam.family,
                    fam.styles.join(", "),
                    font::PANGRAM
                );
                f.render_widget(
                    Paragraph::new(text)
                        .style(Style::default().fg(fg).bg(bg))
                        .wrap(Wrap { trim: true }),
                    inner,
                );
            }
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
        Pane::Market => {
            let text = match app.market_tab {
                MarketTab::Schemes => {
                    if let Some(s) = filtered_market_schemes(app).get(app.market_sel) {
                        format!(
                            "{}\nlicense  {}\nsystem   {}\n{}\n\nEnter / i  install",
                            s.name, s.license, s.system, s.url
                        )
                    } else {
                        "no schemes".into()
                    }
                }
                MarketTab::Fonts => {
                    if let Some(fam) = filtered_market_fonts(app).get(app.market_sel) {
                        format!(
                            "{}\nlicense  {}\nsource   {}\n{}\n\nEnter / i  install (libre only)",
                            fam.family,
                            fam.license,
                            fam.source,
                            fam.notes.clone().unwrap_or_default()
                        )
                    } else {
                        "no fonts".into()
                    }
                }
            };
            f.render_widget(
                Paragraph::new(text)
                    .style(Style::default().fg(fg).bg(bg))
                    .wrap(Wrap { trim: true }),
                inner,
            );
        }
    }
    let _ = acc;
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
  1–6 / Tab     panes\n\
  j/k ↑↓        move\n\
  /             filter\n\
  Enter         set current (no apply)\n\
  a             set + apply\n\
  A             apply all\n\
  R             rollback last session\n\
  q             quit\n\
\n\
apps pane\n\
  t/f/s         theme / font / size override\n\
  c             reset    x skip    h hold\n\
  a             apply this app    m missing    w no-writer\n\
\n\
size            ←/→ or [ ] in 0.5 pt (8–24)\n\
market          s schemes  f fonts  i install  u update";
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

fn draw_confirm(f: &mut ratatui::Frame, n: usize, bg: Color, fg: Color, acc: Color) {
    let area = centered(f.area(), 50, 6);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(format!(
            " apply will touch ~{n} programs\n\n y / Enter  apply     n / Esc  cancel"
        ))
        .block(
            Block::default()
                .title(" confirm ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(acc))
                .style(Style::default().bg(bg).fg(fg)),
        ),
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

fn trunc(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n.saturating_sub(1)])
    }
}
