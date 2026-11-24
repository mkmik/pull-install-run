//! pir: pull a git repo on a timer; when HEAD changes, run a build command and,
//! if it succeeds, restart a long-running "run" command. See README.md.

use std::{
    fs::File,
    io::{self, IsTerminal, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread,
    time::{Duration, Instant},
};

use clap::Parser as _;
use crossterm::event::{
    self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};
use tui_term::{
    vt100,
    widget::{Cursor, PseudoTerminal},
};

#[derive(clap::Parser)]
#[command(
    about = "Pull a git repo periodically; on a new commit rebuild, and restart the tool if the build succeeds"
)]
struct Args {
    /// Shell command that compiles and installs (exit 0 = success)
    #[arg(long)]
    build: String,
    /// Shell command that runs the tool
    #[arg(long)]
    run: String,
    /// Time between pulls (e.g. 30s, 1m, 1h)
    #[arg(long, default_value = "1m", value_parser = parse_dur)]
    every: Duration,
    /// How long to wait after SIGTERM before SIGKILL
    #[arg(long, default_value = "10s", value_parser = parse_dur)]
    stop_timeout: Duration,
    /// No split screen; print prefixed lines (automatic when stdout is not a terminal)
    #[arg(long)]
    plain: bool,
    /// Repo directory
    #[arg(short = 'C', default_value = ".")]
    dir: PathBuf,
    /// Also write each build's and each run's output to a timestamped file here
    #[arg(long)]
    log_dir: Option<PathBuf>,
}

fn parse_dur(s: &str) -> Result<Duration, String> {
    let (n, unit) = s.split_at(s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len()));
    let n: u64 = n.parse().map_err(|_| format!("bad duration {s:?}"))?;
    let mul = match unit {
        "" | "s" => 1,
        "m" => 60,
        "h" => 3600,
        _ => return Err(format!("bad unit in {s:?}; use s, m or h")),
    };
    Ok(Duration::from_secs(n * mul))
}

const SCROLLBACK: usize = 10_000;
const PLAIN_SIZE: (u16, u16) = (50, 200);
const BUILD: usize = 0;
const RUN: usize = 1;
const NAMES: [&str; 2] = ["build", "run"];

enum Ev {
    Out(usize, u64, Vec<u8>),
    Exit(usize, u64, String, bool),
    Pulled(Pull),
    Key(event::KeyEvent),
    Mouse(MouseEvent),
    Signal,
}

struct Pull {
    lines: Vec<String>,
    head: Option<String>,
    branch: String,
}

/// A running build or run command. Its pid is also its process group id
/// (portable-pty calls setsid), so signals go to the whole group.
struct Proc {
    id: u64,
    pid: i32,
    master: Box<dyn MasterPty + Send>,
    started: Instant,
    kill_at: Option<Instant>,
    log: Option<File>,
    log_buf: Vec<u8>,
}

fn killpg(pid: i32, sig: i32) {
    if pid > 0 {
        unsafe { libc::kill(-pid, sig) };
    }
}

fn now() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

/// Appends `data` to `buf` and returns every complete line. A line holding `\r`
/// (progress bars) keeps only its last segment.
fn take_lines(buf: &mut Vec<u8>, data: &[u8]) -> Vec<String> {
    buf.extend_from_slice(data);
    let mut out = vec![];
    while let Some(i) = buf.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = buf.drain(..=i).collect();
        let line = String::from_utf8_lossy(&line[..i]);
        let line = line.trim_end_matches('\r');
        out.push(line.rsplit('\r').next().unwrap_or("").to_string());
    }
    out
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr)
            .trim()
            .replace('\n', " | "))
    }
}

fn pull(dir: &Path, old: Option<String>) -> Pull {
    let mut lines = vec![];
    let mut fetched = false;
    for i in 1..=3 {
        match git(dir, &["fetch", "--quiet"]) {
            Ok(_) => {
                fetched = true;
                break;
            }
            Err(e) => {
                lines.push(format!("git fetch failed (attempt {i}/3): {e}"));
                if i < 3 {
                    thread::sleep(Duration::from_secs(10));
                }
            }
        }
    }
    if fetched && let Err(e) = git(dir, &["merge", "--ff-only", "--quiet", "@{u}"]) {
        lines.push(format!("cannot fast-forward, skipped: {e}"));
    }
    let head = git(dir, &["rev-parse", "HEAD"]).ok();
    let branch = git(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
    match (&old, &head) {
        (None, Some(n)) => lines.push(format!("at {branch} @ {}", short(n))),
        (Some(o), Some(n)) if o != n => {
            lines.push(format!("pulled {}..{}", short(o), short(n)));
            if let Ok(log) = git(dir, &["log", "--oneline", &format!("{o}..{n}")]) {
                lines.extend(log.lines().map(|l| format!("  {l}")));
            }
        }
        _ => {}
    }
    Pull {
        lines,
        head,
        branch,
    }
}

fn short(sha: &str) -> &str {
    &sha[..sha.len().min(7)]
}

struct App {
    args: Args,
    tui: bool,
    tx: Sender<Ev>,
    id: u64,
    procs: [Option<Proc>; 2],
    panes: [vt100::Parser; 2],
    sizes: [(u16, u16); 2],
    plain_bufs: [Vec<u8>; 2],
    head: Option<String>,
    branch: String,
    last_built: Option<String>,
    last_pull_lines: Vec<String>,
    build_again: bool,
    builds: u32,
    build_status: String,
    run_status: String,
    restart_after_stop: bool,
    first_build_done: bool,
    pulling: bool,
    next_pull: Instant,
    quitting: bool,
    exit_code: i32,
    focus: usize,
    zoom: bool,
    /// Share of the screen height the build pane gets; dragged with the mouse.
    split: f32,
    dragging: bool,
    area: Rect,
}

impl App {
    fn log_to(&mut self, pane: usize, msg: &str) {
        let line = format!("{} {msg}", now());
        if !self.tui {
            println!("[pir] {line}");
            return;
        }
        let p = &mut self.panes[pane];
        let nl = if p.screen().cursor_position().1 != 0 {
            "\r\n"
        } else {
            ""
        };
        p.process(format!("\x1b[0m{nl}\x1b[36m{line}\x1b[0m\r\n").as_bytes());
    }

    fn log(&mut self, msg: &str) {
        self.log_to(BUILD, msg);
    }

    fn start_pull(&mut self) {
        self.pulling = true;
        let (dir, old, tx) = (self.args.dir.clone(), self.head.clone(), self.tx.clone());
        thread::spawn(move || tx.send(Ev::Pulled(pull(&dir, old))));
    }

    fn maybe_build(&mut self, force: bool) {
        let new = self.head.is_some() && self.head != self.last_built;
        if self.quitting || !(force || new) {
            return;
        }
        if self.procs[BUILD].is_some() {
            self.build_again = true;
            return;
        }
        self.last_built = self.head.clone();
        self.builds += 1;
        let msg = format!(
            "── build #{} @ {}: $ {}",
            self.builds,
            self.head.as_deref().map(short).unwrap_or("?"),
            self.args.build
        );
        self.log(&msg);
        self.build_status = "running".into();
        self.spawn(BUILD);
    }

    fn restart_run(&mut self) {
        if self.quitting {
            return;
        }
        if self.procs[RUN].is_some() {
            self.restart_after_stop = true;
            self.stop(RUN);
        } else {
            self.spawn(RUN);
        }
    }

    fn stop(&mut self, k: usize) {
        if let Some(p) = &mut self.procs[k]
            && p.kill_at.is_none()
        {
            killpg(p.pid, libc::SIGTERM);
            p.kill_at = Some(Instant::now() + self.args.stop_timeout);
        }
    }

    fn quit(&mut self, code: i32) {
        if self.quitting {
            // Second request: no more waiting.
            for p in self.procs.iter().flatten() {
                killpg(p.pid, libc::SIGKILL);
            }
            self.procs = [None, None];
            return;
        }
        self.quitting = true;
        self.exit_code = code;
        if self.procs.iter().any(Option::is_some) {
            self.log("stopping… (press again to kill now)");
        }
        self.stop(BUILD);
        self.stop(RUN);
    }

    fn spawn(&mut self, k: usize) {
        if let Err(e) = self.try_spawn(k) {
            self.log(&format!("cannot start {}: {e}", NAMES[k]));
            if k == RUN {
                self.run_status = "failed to start".into();
            } else {
                self.build_status = "failed to start".into();
            }
        }
    }

    fn try_spawn(&mut self, k: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (rows, cols) = if self.tui { self.sizes[k] } else { PLAIN_SIZE };
        let pair = native_pty_system().openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut cmd = CommandBuilder::new("/bin/sh");
        cmd.args([
            "-c",
            if k == BUILD {
                &self.args.build
            } else {
                &self.args.run
            },
        ]);
        cmd.cwd(&self.args.dir);
        let mut child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);
        let pid = child.process_id().unwrap_or(0) as i32;
        self.id += 1;
        let id = self.id;

        let mut reader = pair.master.try_clone_reader()?;
        let (tx, (done_tx, done_rx)) = (self.tx.clone(), mpsc::channel::<()>());
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 || tx.send(Ev::Out(k, id, buf[..n].to_vec())).is_err() {
                    break;
                }
            }
            let _ = done_tx.send(());
        });
        let tx = self.tx.clone();
        thread::spawn(move || {
            let (status, ok) = match child.wait() {
                Ok(s) => (s.to_string(), s.success()),
                Err(e) => (e.to_string(), false),
            };
            // Let the reader drain so "exited" is logged after the last output.
            // Bounded: a background grandchild may hold the pty open.
            let _ = done_rx.recv_timeout(Duration::from_millis(500));
            let _ = tx.send(Ev::Exit(k, id, status, ok));
        });

        let log = match &self.args.log_dir {
            Some(d) => {
                let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
                Some(File::create(d.join(format!("{}-{ts}-{id}.log", NAMES[k])))?)
            }
            None => None,
        };
        if k == RUN {
            self.log_to(
                RUN,
                &format!("── run started (pid {pid}): $ {}", self.args.run),
            );
            if self.tui {
                self.log(&format!("run started (pid {pid})"));
            }
        }
        self.procs[k] = Some(Proc {
            id,
            pid,
            master: pair.master,
            started: Instant::now(),
            kill_at: None,
            log,
            log_buf: vec![],
        });
        Ok(())
    }

    fn on_output(&mut self, k: usize, id: u64, data: &[u8]) {
        if self.tui {
            self.panes[k].process(data);
        } else {
            for line in take_lines(&mut self.plain_bufs[k], data) {
                println!("[{}] {line}", NAMES[k]);
            }
        }
        if let Some(p) = self.procs[k].as_mut().filter(|p| p.id == id)
            && let Some(f) = &mut p.log
        {
            for line in take_lines(&mut p.log_buf, data) {
                let _ = writeln!(f, "{} {line}", chrono::Local::now().format("%H:%M:%S%.3f"));
            }
        }
    }

    fn on_exit(&mut self, k: usize, id: u64, status: String, ok: bool) {
        let Some(p) = self.procs[k].take_if(|p| p.id == id) else {
            return;
        };
        killpg(p.pid, libc::SIGKILL); // leftovers in the group, if any
        if !self.tui && !self.plain_bufs[k].is_empty() {
            let rest = std::mem::take(&mut self.plain_bufs[k]);
            println!("[{}] {}", NAMES[k], String::from_utf8_lossy(&rest));
        }
        let secs = p.started.elapsed().as_secs_f32();
        if k == BUILD {
            self.build_status = if ok { "ok".into() } else { "FAILED".into() };
            self.log(&format!(
                "build {} ({status}, {secs:.1}s)",
                if ok { "ok" } else { "FAILED" }
            ));
            if self.quitting {
                return;
            }
            if ok {
                self.restart_run();
            } else if !self.first_build_done && self.procs[RUN].is_none() {
                self.log("starting the previously installed tool anyway");
                self.spawn(RUN);
            }
            self.first_build_done = true;
            if std::mem::take(&mut self.build_again) {
                self.maybe_build(true);
            }
        } else {
            self.run_status = format!("exited: {status}");
            self.log_to(RUN, &format!("── run exited: {status} after {secs:.0}s"));
            if self.tui {
                self.log(&format!("run exited: {status}"));
            }
            if std::mem::take(&mut self.restart_after_stop) && !self.quitting {
                self.spawn(RUN);
            }
        }
    }

    fn on_key(&mut self, key: event::KeyEvent) {
        let rows = self.sizes[self.focus].0 as usize;
        let scroll = |app: &mut Self, f: &dyn Fn(usize) -> usize| app.scroll(f);
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit(130),
            KeyCode::Char('q') => self.quit(0),
            KeyCode::Char('p') if !self.pulling => self.start_pull(),
            KeyCode::Char('b') => self.maybe_build(true),
            KeyCode::Char('r') => {
                self.log("restart requested");
                self.restart_run();
            }
            KeyCode::Tab => self.focus = 1 - self.focus,
            KeyCode::Char('z') => self.zoom = !self.zoom,
            KeyCode::PageUp => scroll(self, &|s| s + rows / 2),
            KeyCode::PageDown => scroll(self, &|s| s.saturating_sub(rows / 2)),
            KeyCode::Up => scroll(self, &|s| s + 1),
            KeyCode::Down => scroll(self, &|s| s.saturating_sub(1)),
            KeyCode::Home => scroll(self, &|_| usize::MAX),
            KeyCode::End => scroll(self, &|_| 0),
            _ => {}
        }
    }

    fn scroll(&mut self, f: &dyn Fn(usize) -> usize) {
        let s = self.panes[self.focus].screen_mut();
        let to = f(s.scrollback());
        s.set_scrollback(to);
    }

    fn on_mouse(&mut self, m: MouseEvent) {
        let (rects, divider, _) = self.pane_rects(self.area);
        let hit = [BUILD, RUN]
            .into_iter()
            .find(|&k| rects[k].is_some_and(|r| (r.y..r.bottom()).contains(&m.row)));
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) if m.row == divider.y && !self.zoom => {
                self.dragging = true;
            }
            MouseEventKind::Drag(MouseButton::Left) if self.dragging => {
                let h = self.area.height.saturating_sub(2).max(1);
                self.split = (m.row.saturating_sub(self.area.y) as f32 / h as f32).clamp(0.0, 1.0);
            }
            MouseEventKind::Up(_) => self.dragging = false,
            MouseEventKind::Down(_) => self.focus = hit.unwrap_or(self.focus),
            // Capturing the mouse takes the wheel away from the terminal, so scroll here.
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                self.focus = hit.unwrap_or(self.focus);
                let up = m.kind == MouseEventKind::ScrollUp;
                self.scroll(&|s| if up { s + 3 } else { s.saturating_sub(3) });
            }
            _ => {}
        }
    }

    fn handle(&mut self, ev: Ev) {
        match ev {
            Ev::Out(k, id, data) => self.on_output(k, id, &data),
            Ev::Exit(k, id, status, ok) => self.on_exit(k, id, status, ok),
            Ev::Pulled(p) => {
                self.pulling = false;
                self.next_pull = Instant::now() + self.args.every;
                // The same warning every minute (e.g. no upstream) is noise: log it once.
                if p.lines != self.last_pull_lines {
                    for l in &p.lines {
                        self.log(l);
                    }
                }
                self.last_pull_lines = p.lines;
                self.branch = p.branch;
                if p.head.is_some() {
                    self.head = p.head;
                }
                self.maybe_build(false);
            }
            Ev::Key(k) => self.on_key(k),
            Ev::Mouse(m) => self.on_mouse(m),
            Ev::Signal => self.quit(130),
        }
    }

    fn tick(&mut self) {
        if !self.pulling && !self.quitting && Instant::now() >= self.next_pull {
            self.start_pull();
        }
        for p in self.procs.iter_mut().flatten() {
            if p.kill_at.is_some_and(|t| Instant::now() >= t) {
                killpg(p.pid, libc::SIGKILL);
            }
        }
    }

    /// Build on top, a one-row divider, run below, status bar last.
    /// Zoomed, the divider sits on top and the focused pane fills the rest.
    /// Returns (pane rects, divider, status bar).
    fn pane_rects(&self, area: Rect) -> ([Option<Rect>; 2], Rect, Rect) {
        let [main, status] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);
        if self.zoom {
            let [divider, pane] =
                Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(main);
            let mut r = [None, None];
            r[self.focus] = Some(pane);
            return (r, divider, status);
        }
        let room = main.height.saturating_sub(1);
        let top =
            ((room as f32 * self.split).round() as u16).clamp(1, room.saturating_sub(1).max(1));
        let [b, divider, r] = Layout::vertical([
            Constraint::Length(top),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(main);
        ([Some(b), Some(r)], divider, status)
    }

    /// Keep each emulator and pty the size of its visible pane.
    fn sync_sizes(&mut self, area: Rect) {
        self.area = area;
        let (rects, _, _) = self.pane_rects(area);
        for k in [BUILD, RUN] {
            let Some(r) = rects[k] else { continue };
            let size = (r.height.max(1), r.width.max(1));
            if size != self.sizes[k] {
                self.sizes[k] = size;
                self.panes[k].screen_mut().set_size(size.0, size.1);
                if let Some(p) = &self.procs[k] {
                    let _ = p.master.resize(PtySize {
                        rows: size.0,
                        cols: size.1,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
            }
        }
    }

    fn status_line(&self) -> String {
        let run = match &self.procs[RUN] {
            Some(p) if p.kill_at.is_some() => "stopping".to_string(),
            Some(p) => format!("up {}s", p.started.elapsed().as_secs()),
            None => self.run_status.clone(),
        };
        let next = if self.pulling {
            "pulling".to_string()
        } else {
            format!(
                "next pull {}s",
                self.next_pull
                    .saturating_duration_since(Instant::now())
                    .as_secs()
            )
        };
        let head = self.head.as_deref().map(short).unwrap_or("?");
        let q = if self.quitting { " · STOPPING" } else { "" };
        format!(
            " {} @ {head} · build: {} · run: {run} · {next}{q} │ p pull  b build  r restart  tab focus  z zoom  PgUp/PgDn scroll  drag ── resize  q quit",
            self.branch, self.build_status
        )
    }

    fn draw(&self, term: &mut DefaultTerminal) -> io::Result<()> {
        term.draw(|f| {
            let (rects, divider, status) = self.pane_rects(f.area());
            let line = Style::new().fg(Color::DarkGray);
            let mut spans = vec![];
            for k in [BUILD, RUN] {
                let screen = self.panes[k].screen();
                let cmd = if k == BUILD {
                    &self.args.build
                } else {
                    &self.args.run
                };
                let arrow = if k == BUILD { '▲' } else { '▼' };
                let mut title = format!(" {arrow} {} · {cmd} ", NAMES[k]);
                if screen.scrollback() > 0 {
                    title += &format!("[scrolled {}] ", screen.scrollback());
                }
                let color = if k == self.focus {
                    Color::Yellow
                } else {
                    Color::DarkGray
                };
                spans.extend([Span::styled("──", line), Span::styled(title, color)]);
                let Some(r) = rects[k] else { continue };
                let mut cursor = Cursor::default();
                cursor.hide();
                f.render_widget(PseudoTerminal::new(screen).cursor(cursor), r);
            }
            let used: usize = spans.iter().map(Span::width).sum();
            spans.push(Span::styled(
                "─".repeat((divider.width as usize).saturating_sub(used)),
                line,
            ));
            f.render_widget(Line::from(spans), divider);
            f.render_widget(Paragraph::new(self.status_line()).reversed(), status);
        })?;
        Ok(())
    }

    fn done(&self) -> bool {
        self.quitting && self.procs.iter().all(Option::is_none)
    }
}

fn main() {
    let mut args = Args::parse();
    args.dir = match args.dir.canonicalize() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("pir: {}: {e}", args.dir.display());
            std::process::exit(2);
        }
    };
    if let Some(d) = &args.log_dir
        && let Err(e) = std::fs::create_dir_all(d)
    {
        eprintln!("pir: --log-dir {}: {e}", d.display());
        std::process::exit(2);
    }
    let tui = !args.plain && io::stdout().is_terminal();
    let (tx, rx) = mpsc::channel();

    let sig_tx = tx.clone();
    let mut signals =
        signal_hook::iterator::Signals::new([libc::SIGINT, libc::SIGTERM, libc::SIGHUP])
            .expect("signal handler");
    thread::spawn(move || {
        for _ in signals.forever() {
            let _ = sig_tx.send(Ev::Signal);
        }
    });

    let mut app = App {
        args,
        tui,
        tx: tx.clone(),
        id: 0,
        procs: [None, None],
        panes: [
            vt100::Parser::new(24, 80, SCROLLBACK),
            vt100::Parser::new(24, 80, SCROLLBACK),
        ],
        sizes: [(24, 80), (24, 80)],
        plain_bufs: [vec![], vec![]],
        head: None,
        branch: String::new(),
        last_built: None,
        last_pull_lines: vec![],
        build_again: false,
        builds: 0,
        build_status: "-".into(),
        run_status: "not started".into(),
        restart_after_stop: false,
        first_build_done: false,
        pulling: false,
        next_pull: Instant::now(),
        quitting: false,
        exit_code: 0,
        focus: RUN,
        zoom: false,
        split: 0.5,
        dragging: false,
        area: Rect::default(),
    };

    let mut term = if tui {
        let t = ratatui::init();
        let _ = crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture);
        thread::spawn(move || {
            loop {
                match event::read() {
                    Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => {
                        if tx.send(Ev::Key(k)).is_err() {
                            break;
                        }
                    }
                    Ok(Event::Mouse(m)) => {
                        if tx.send(Ev::Mouse(m)).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });
        Some(t)
    } else {
        None
    };

    run_loop(&mut app, &rx, term.as_mut());
    if tui {
        let _ = crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture);
        ratatui::restore();
    }
    std::process::exit(app.exit_code);
}

fn run_loop(app: &mut App, rx: &Receiver<Ev>, mut term: Option<&mut DefaultTerminal>) {
    loop {
        if let Some(t) = term.as_deref_mut() {
            if let Ok(size) = t.size() {
                app.sync_sizes(Rect::new(0, 0, size.width, size.height));
            }
            if app.draw(t).is_err() {
                app.quit(1);
            }
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ev) => app.handle(ev),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        // Drain a bounded batch so a chatty tool can't starve the redraw.
        for ev in rx.try_iter().take(1000) {
            app.handle(ev);
        }
        app.tick();
        if app.done() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(parse_dur("30s"), Ok(Duration::from_secs(30)));
        assert_eq!(parse_dur("2m"), Ok(Duration::from_secs(120)));
        assert_eq!(parse_dur("1h"), Ok(Duration::from_secs(3600)));
        assert_eq!(parse_dur("5"), Ok(Duration::from_secs(5)));
        assert!(parse_dur("5d").is_err() && parse_dur("m").is_err());
    }

    #[test]
    fn lines() {
        let mut b = vec![];
        assert!(take_lines(&mut b, b"par").is_empty());
        assert_eq!(
            take_lines(&mut b, b"tial\r\n10%\r50%\r100%\r\nx"),
            vec!["partial", "100%"]
        );
        assert_eq!(b, b"x");
    }

    #[test]
    fn drag_divider() {
        let args = Args::parse_from(["pir", "--build", "true", "--run", "true"]);
        let (tx, _rx) = mpsc::channel();
        let mut app = App {
            args,
            tui: true,
            tx,
            id: 0,
            procs: [None, None],
            panes: [vt100::Parser::new(1, 1, 0), vt100::Parser::new(1, 1, 0)],
            sizes: [(1, 1); 2],
            plain_bufs: [vec![], vec![]],
            head: None,
            branch: String::new(),
            last_built: None,
            last_pull_lines: vec![],
            build_again: false,
            builds: 0,
            build_status: String::new(),
            run_status: String::new(),
            restart_after_stop: false,
            first_build_done: false,
            pulling: false,
            next_pull: Instant::now(),
            quitting: false,
            exit_code: 0,
            focus: RUN,
            zoom: false,
            split: 0.5,
            dragging: false,
            area: Rect::default(),
        };
        // 42 rows: 40 for panes, 1 divider, 1 status bar.
        app.sync_sizes(Rect::new(0, 0, 80, 42));
        assert_eq!((app.sizes[BUILD].0, app.sizes[RUN].0), (20, 20));
        let mouse = |kind, row| MouseEvent {
            kind,
            column: 5,
            row,
            modifiers: KeyModifiers::NONE,
        };
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 20));
        app.on_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 10));
        app.on_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 10));
        app.sync_sizes(app.area);
        assert_eq!((app.sizes[BUILD].0, app.sizes[RUN].0), (10, 30));
        // Dragging past an edge leaves each pane at least a row.
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 10));
        app.on_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 0));
        app.sync_sizes(app.area);
        assert_eq!((app.sizes[BUILD].0, app.sizes[RUN].0), (1, 39));
        // A click in a pane focuses it.
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 0));
        assert_eq!(app.focus, BUILD);
    }
}
