use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame, style::Stylize, text::Line};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
const CONFIG_FILE: &str = "multimr.toml";
const DEFAULT_BRANCHES: [&str; 2] = ["main", "master"];

fn main() -> color_eyre::Result<()> {
    ensure_glab_installed();

    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}

#[derive(Debug, Default)]
enum Screen {
    #[default]
    RepoSelection,
    CreateMR,
    ReviewerSelection,
    Finalize,
}

impl Screen {
    fn help(&self) -> &'static str {
        match self {
            Screen::RepoSelection => "↑/↓: Move  Space: Select  Enter: Next  q/Esc: Quit",
            Screen::CreateMR => "Tab: Switch field  ↑/↓: Select Label  Enter: Next  Esc: Back",
            Screen::ReviewerSelection => "↑/↓: Move  Space: Select  Enter: Finish  Esc: Back",
            Screen::Finalize => "y: Confirm  n: Back",
        }
    }
}

#[derive(Debug, Default)]
pub struct Config {
    /// The root directory for the repositories.
    pub working_dir: PathBuf,
    /// List of reviewers.
    pub reviewers: Vec<String>,
    /// List of labels.
    pub labels: HashMap<String, String>, // (key, value)
    /// Assignee for the merge request.
    pub assignee: String,
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,
    /// List of directories in the current working directory.
    dirs: Vec<String>,
    /// Indices of selected directories
    selected_repos: HashSet<usize>,
    /// Currently highlighted directory index
    selected_index: usize,
    screen: Screen,
    /// For CreateMR screen
    mr_title: String,
    mr_description: String,
    input_focus: InputFocus,
    /// Indices of selected reviewers
    selected_reviewers: HashSet<usize>,
    /// Currently highlighted reviewer index
    reviewer_index: usize,
    selected_label: usize,
    cfg: Config,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum InputFocus {
    #[default]
    Label,
    Title,
    Description,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        let mut app = Self {
            selected_label: 0,
            selected_index: 0,
            ..Default::default()
        };
        // Load reviewers from reviewers.toml
        let cfg = load_config_from_toml();
        app.cfg = cfg;

        // Populate dirs with all directories in the current working directory
        if let Ok(entries) = fs::read_dir(&app.cfg.working_dir) {
            app.dirs = entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let path = entry.path();
                    if path.is_dir() {
                        path.file_name().map(|n| n.to_string_lossy().to_string())
                    } else {
                        None
                    }
                })
                .collect();
        }
        app
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    /// Renders the user interface.
    fn render(&mut self, frame: &mut Frame) {
        match self.screen {
            Screen::RepoSelection => self.render_repo_selection(frame),
            Screen::CreateMR => self.render_create_mr(frame),
            Screen::ReviewerSelection => self.render_reviewer_selection(frame),
            Screen::Finalize => self.render_overview(frame),
        }
    }

    fn render_repo_selection(&mut self, frame: &mut Frame) {
        let title = Line::from("Multi MR").bold().blue().centered();
        let items: Vec<ListItem> = self
            .dirs
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let line = if self.selected_repos.contains(&i) {
                    format!("[x] {}", d)
                } else {
                    format!("[ ] {}", d)
                };
                let mut item = ListItem::new(line);
                if i == self.selected_index {
                    item = item.style(Style::default().fg(Color::Yellow).bg(Color::Blue));
                }
                item
            })
            .collect();
        let list = List::new(items).block(Block::bordered().title(title));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(1), // for description
                Constraint::Length(1), // for key help bar
                Constraint::Length(1), // for directory info
            ])
            .split(frame.area());
        frame.render_widget(list, chunks[0]);
        let desc = Paragraph::new("Select repositories to create MR for").centered();
        frame.render_widget(desc, chunks[1]);
        let dir_info = Paragraph::new(format!(
            "Current directory: {} (Selected: {})",
            self.cfg.working_dir.display(),
            self.selected_repos.len()
        ))
        .centered();
        frame.render_widget(dir_info, chunks[2]);
        let help = Paragraph::new(self.screen.help())
            .centered()
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[3]);
    }

    fn render_create_mr(&mut self, frame: &mut Frame) {
        let selected_dirs: Vec<&String> = self
            .selected_repos
            .iter()
            .copied()
            .filter_map(|i| self.dirs.get(i))
            .collect();
        let dirs_text = if selected_dirs.is_empty() {
            "No repositories selected".to_string()
        } else {
            selected_dirs
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let title =
            Paragraph::new("Create Merge Request").style(Style::default().fg(Color::Blue).bold());
        let dirs = Paragraph::new(format!("Repositories:\n{}", dirs_text));
        let title_input = if self.input_focus == InputFocus::Title {
            Paragraph::new(self.mr_title.as_str())
                .style(Style::default().bg(Color::Blue).fg(Color::White))
                .block(Block::bordered().title("Title"))
        } else {
            Paragraph::new(self.mr_title.as_str()).block(Block::bordered().title("Title"))
        };
        let desc_input = if self.input_focus == InputFocus::Description {
            Paragraph::new(self.mr_description.as_str())
                .style(Style::default().bg(Color::Blue).fg(Color::White))
                .block(Block::bordered().title("Description"))
        } else {
            Paragraph::new(self.mr_description.as_str())
                .block(Block::bordered().title("Description"))
        };
        // Label selection box
        let label_items: Vec<ListItem> = self
            .cfg
            .labels
            .iter()
            .enumerate()
            .map(|(i, (k, v))| {
                let marker = if i == self.selected_label {
                    "(x)"
                } else {
                    "( )"
                };
                let line = format!("{} {}: {}", marker, k, v);
                let mut item = ListItem::new(line);
                if self.input_focus == InputFocus::Label && i == self.selected_label {
                    item = item.style(Style::default().fg(Color::Yellow).bg(Color::Blue));
                } else if i == self.selected_label {
                    item = item.style(Style::default().fg(Color::Yellow));
                }
                item
            })
            .collect();
        let label_list = List::new(label_items).block(Block::bordered().title("Label"));
        // Layout: title, dirs, title input, desc input, label select, help
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(5), // label box
                Constraint::Length(1), // help
            ])
            .split(frame.area());
        frame.render_widget(title, layout[0]);
        frame.render_widget(dirs, layout[1]);
        frame.render_widget(title_input, layout[2]);
        frame.render_widget(desc_input, layout[3]);
        frame.render_widget(label_list, layout[4]);
        let help = Paragraph::new(self.screen.help())
            .centered()
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, layout[5]);
    }

    fn render_reviewer_selection(&mut self, frame: &mut Frame) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Style};
        use ratatui::widgets::{ListItem, Paragraph};
        let title = Line::from("Select Reviewers").bold().blue().centered();
        let items: Vec<ListItem> = self
            .cfg
            .reviewers
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let line = if self.selected_reviewers.contains(&i) {
                    format!("[x] {}", r)
                } else {
                    format!("[ ] {}", r)
                };
                let mut item = ListItem::new(line);
                if i == self.reviewer_index {
                    item = item.style(Style::default().fg(Color::Yellow).bg(Color::Blue));
                }
                item
            })
            .collect();
        let list = List::new(items).block(Block::bordered().title(title));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(frame.area());
        frame.render_widget(list, chunks[0]);
        let desc = Paragraph::new("Select reviewers for the MR").centered();
        frame.render_widget(desc, chunks[1]);
        let help = Paragraph::new(self.screen.help())
            .centered()
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[2]);
    }

    fn render_overview(&mut self, frame: &mut Frame) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Style};
        use ratatui::widgets::Paragraph;
        let selected_dirs: Vec<&String> = self
            .selected_repos
            .iter()
            .copied()
            .filter_map(|i| self.dirs.get(i))
            .collect();
        let selected_reviewers: Vec<&String> = self
            .selected_reviewers
            .iter()
            .copied()
            .filter_map(|i| self.cfg.reviewers.get(i))
            .collect();
        let dirs_text = if selected_dirs.is_empty() {
            "No repositories selected".to_string()
        } else {
            selected_dirs
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let reviewers_text = if selected_reviewers.is_empty() {
            "No reviewers selected".to_string()
        } else {
            selected_reviewers
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let overview = format!(
            "Overview\n\nRepositories: {}\nTitle: {}\nDescription: {}\nReviewers: {}\n\nPress 'y' to confirm, 'n' to go back.",
            dirs_text, self.mr_title, self.mr_description, reviewers_text
        );
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(frame.area());
        let para = Paragraph::new(overview);
        frame.render_widget(para, layout[0]);
        let help = Paragraph::new(self.screen.help())
            .centered()
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, layout[1]);
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match self.screen {
            Screen::RepoSelection => self.on_key_event_selection(key),
            Screen::CreateMR => self.on_key_event_create_mr(key),
            Screen::ReviewerSelection => self.on_key_event_select_reviewers(key),
            Screen::Finalize => self.on_key_event_overview(key),
        }
    }

    fn on_key_event_selection(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Down | KeyCode::Char('j')) => {
                if !self.dirs.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.dirs.len();
                }
            }
            (_, KeyCode::Up | KeyCode::Char('k')) => {
                if !self.dirs.is_empty() {
                    if self.selected_index == 0 {
                        self.selected_index = self.dirs.len() - 1;
                    } else {
                        self.selected_index -= 1;
                    }
                }
            }
            (_, KeyCode::Char(' ')) => {
                if self.selected_repos.contains(&self.selected_index) {
                    self.selected_repos.remove(&self.selected_index);
                } else {
                    self.selected_repos.insert(self.selected_index);
                }
            }
            (_, KeyCode::Enter) => {
                if !self.selected_repos.is_empty() {
                    self.screen = Screen::CreateMR;
                }
            }
            _ => {}
        }
    }

    fn on_key_event_create_mr(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.input_focus = match self.input_focus {
                    InputFocus::Title => InputFocus::Description,
                    InputFocus::Description => InputFocus::Label,
                    InputFocus::Label => InputFocus::Title,
                };
            }
            KeyCode::Backspace => match self.input_focus {
                InputFocus::Title => {
                    self.mr_title.pop();
                }
                InputFocus::Description => {
                    self.mr_description.pop();
                }
                InputFocus::Label => {}
            },
            KeyCode::Char(c) => match self.input_focus {
                InputFocus::Title => self.mr_title.push(c),
                InputFocus::Description => self.mr_description.push(c),
                InputFocus::Label => match c {
                    'j' => {
                        if !self.cfg.labels.is_empty() {
                            let idx = self.selected_label;
                            self.selected_label = (idx + 1) % self.cfg.labels.len();
                        }
                    }
                    'k' => {
                        if !self.cfg.labels.is_empty() {
                            let idx = self.selected_label;
                            self.selected_label = if idx == 0 {
                                self.cfg.labels.len() - 1
                            } else {
                                idx - 1
                            };
                        }
                    }
                    _ => {
                        // Ignore other characters in label input
                    }
                },
            },
            KeyCode::Down => {
                if self.input_focus == InputFocus::Label && !self.cfg.labels.is_empty() {
                    let idx = self.selected_label;
                    self.selected_label = (idx + 1) % self.cfg.labels.len();
                }
            }
            KeyCode::Up => {
                if self.input_focus == InputFocus::Label && !self.cfg.labels.is_empty() {
                    let idx = self.selected_label;
                    self.selected_label = if idx == 0 {
                        self.cfg.labels.len() - 1
                    } else {
                        idx - 1
                    };
                }
            }
            KeyCode::Enter => {
                self.screen = Screen::ReviewerSelection;
            }
            KeyCode::Esc => {
                self.screen = Screen::RepoSelection;
            }
            _ => {}
        }
    }

    fn on_key_event_select_reviewers(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.cfg.reviewers.is_empty() {
                    self.reviewer_index = (self.reviewer_index + 1) % self.cfg.reviewers.len();
                }
            }
            KeyCode::Up | KeyCode::Char('h') => {
                if !self.cfg.reviewers.is_empty() {
                    if self.reviewer_index == 0 {
                        self.reviewer_index = self.cfg.reviewers.len() - 1;
                    } else {
                        self.reviewer_index -= 1;
                    }
                }
            }
            KeyCode::Char(' ') => {
                if self.selected_reviewers.contains(&self.reviewer_index) {
                    self.selected_reviewers.remove(&self.reviewer_index);
                } else {
                    self.selected_reviewers.insert(self.reviewer_index);
                }
            }
            KeyCode::Enter => {
                self.screen = Screen::Finalize;
            }
            KeyCode::Esc => {
                self.screen = Screen::CreateMR;
            }
            _ => {}
        }
    }

    fn on_key_event_overview(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') => {
                let mut mr = MergeRequest {
                    title: self.mr_title.clone(),
                    description: self.mr_description.clone(),
                    reviewers: self
                        .selected_reviewers
                        .iter()
                        .map(|&i| self.cfg.reviewers[i].clone())
                        .collect(),
                    labels: self
                        .cfg
                        .labels
                        .keys()
                        .nth(self.selected_label)
                        .map(|k| vec![k.clone()])
                        .unwrap_or_default(),
                    draft: false, // TODO: Add a way to mark as draft
                    assignee: self.cfg.assignee.clone(),
                    cmd: None, // Placeholder for command, not used in this example
                };
                for dir_index in &self.selected_repos {
                    let dir = self.dirs[*dir_index].clone();
                    std::env::set_current_dir(&self.cfg.working_dir.join(&dir))
                        .expect(format!("Failed to change directory to: {}", dir).as_str());

                    // mr.dummy_create();
                    mr.create();
                    mr.run();
                }

                self.quit();
            }
            KeyCode::Char('n') => {
                self.screen = Screen::ReviewerSelection;
            }
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}

pub struct MergeRequest {
    title: String,
    description: String,
    reviewers: Vec<String>,
    labels: Vec<String>,
    draft: bool,
    assignee: String,
    cmd: Option<std::process::Command>,
}

impl MergeRequest {
    // Placeholder for actual MR creation logic
    fn dummy_create(&mut self) {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(format!(
            "terminal-notifier -sound default -title 'Created MR: {}' -message '{}'",
            self.title, self.description,
        ));

        self.cmd = Some(cmd);
    }

    fn create(&mut self) {
        // Create the merge request using glab CLI
        let mut cmd = std::process::Command::new("glab");
        cmd.arg("mr")
            .arg("create")
            .arg("--assignee")
            .arg(&self.assignee);

        if !self.reviewers.is_empty() {
            for reviewer in &self.reviewers {
                cmd.arg("--reviewer").arg(reviewer);
            }
        }

        if !self.labels.is_empty() {
            for label in &self.labels {
                cmd.arg("--label").arg(label);
            }
        }

        let current_branch = get_current_branch();

        cmd.arg("--title").arg(&self.title);
        cmd.arg("--description").arg(&self.description);

        if DEFAULT_BRANCHES.contains(&current_branch.as_str()) {
            // If the current branch is main or master, create a new branch

            std::process::Command::new("git")
                .arg("switch")
                .arg("-c")
                .arg(self.title.replace(' ', "-"))
                .status()
                .expect("Failed to create new branch");

            std::process::Command::new("git")
                .arg("add")
                .arg(".")
                .status()
                .expect("Failed to add changes");

            std::process::Command::new("git")
                .arg("commit")
                .arg("-am")
                .arg(&self.title)
                .status()
                .or_else(|_e| -> Result<std::process::ExitStatus, std::io::Error> {
                    // Retry once if adding and committing fails, this might happen if the pre-commit hook formats the code
                    // TODO: test this.
                    std::process::Command::new("git")
                        .arg("add")
                        .arg(".")
                        .status()
                        .expect("Failed to add changes Second attempt");

                    let status = std::process::Command::new("git")
                        .arg("commit")
                        .arg("-am")
                        .arg(&self.title)
                        .status()
                        .expect("Failed to commit changes second attempt");

                    Ok(status)
                })
                .expect("Failed to commit changes twice.");

            // TODO: add retry for when pre-commit hook makes some formatting changes

            cmd.arg("--push");
        } else {
            // If not, just use the current branch
            cmd.arg("--yes");
        }

        self.cmd = Some(cmd);
    }

    fn run(&mut self) {
        if let Some(cmd) = &mut self.cmd {
            let status = cmd.status().expect("Failed to execute command");
            if !status.success() {
                eprintln!("Failed to create merge request: {:?}", status);
            } else {
                println!("Merge request created successfully.");
            }
        } else {
            eprintln!("No command to run. Please create the MR first.");
        }
    }

    fn dry_run(&mut self) {
        if let Some(cmd) = &self.cmd {
            println!("Dry run command: {:?}", cmd);
        } else {
            eprintln!("No command to dry run. Please create the MR first.");
        }
    }
}

fn get_current_branch() -> String {
    let current_branch_output = std::process::Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output()
        .expect("Failed to get current branch");

    String::from_utf8_lossy(&current_branch_output.stdout)
        .trim()
        .to_string()
}

fn load_config_from_toml() -> Config {
    let content = std::fs::read_to_string(CONFIG_FILE).unwrap_or_default();

    #[derive(Deserialize)]
    struct ConfigToml {
        reviewers: Option<Vec<String>>,
        labels: Option<HashMap<String, String>>,
        working_dir: Option<String>,
        assignee: Option<String>,
    }

    // if the entire parsing fails return a config with None values
    let parsed: ConfigToml = toml::from_str(&content).unwrap_or(ConfigToml {
        reviewers: None,
        labels: None,
        working_dir: None,
        assignee: None,
    });

    // check if a root is specified in toml, if not use current directory
    let working_dir_str = parsed.working_dir.unwrap_or(".".to_string());

    // there is a root, now create a PathBuf
    let working_dir = if working_dir_str.starts_with('/') || working_dir_str.starts_with('\\') {
        // root // absolute path
        PathBuf::from(&working_dir_str)
            .canonicalize()
            .expect("Failed to resolve absolute path")
    } else {
        // working dir is specified as relative path
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(working_dir_str)
            .canonicalize()
            .expect("Failed to resolve relative path")
    };

    // if individual fields fail, we use default values
    Config {
        working_dir,
        reviewers: parsed.reviewers.unwrap_or_default(),
        labels: parsed
            .labels
            .map(|m| m.into_iter().collect())
            .unwrap_or_default(),
        assignee: parsed.assignee.expect("Assignee is required"),
    }
}

fn ensure_glab_installed() {
    if std::process::Command::new("glab")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!(
            "[Error] Gitlab CLI `glab` is not installed. Please install it to use this application."
        );
        std::process::exit(1);
    }
}

fn ensure_git_repo() {
    if std::process::Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .is_err()
    {
        eprintln!(
            "[Error] This is not a git repository. Please run this application inside a git repository."
        );
        std::process::exit(1);
    }
}
