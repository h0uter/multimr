use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{ListItem, Paragraph};
use ratatui::{
    DefaultTerminal, Frame,
    style::Stylize,
    text::Line,
    widgets::{Block, List},
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> color_eyre::Result<()> {
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
    SelectReviewers,
    Overview,
}

impl Screen {
    fn help(&self) -> &'static str {
        match self {
            Screen::RepoSelection => "↑/↓: Move  Space: Select  Enter: Next  q/Esc/Ctrl+C: Quit",
            Screen::CreateMR => "Tab: Switch field  ↑/↓: Select Label  Enter: Next  Esc: Back",
            Screen::SelectReviewers => "↑/↓: Move  Space: Select  Enter: Finish  Esc: Back",
            Screen::Overview => "y: Confirm  n: Back",
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
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,
    /// List of directories in the current working directory.
    dirs: Vec<String>,
    /// Indices of selected directories
    selected: HashSet<usize>,
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
        let cfg = load_reviewers_and_labels_from_toml();
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
            Screen::RepoSelection => self.render_selection(frame),
            Screen::CreateMR => self.render_create_mr(frame),
            Screen::SelectReviewers => self.render_select_reviewers(frame),
            Screen::Overview => self.render_overview(frame),
        }
    }

    fn render_selection(&mut self, frame: &mut Frame) {
        let title = Line::from("Mutli MR").bold().blue().centered();
        let items: Vec<ListItem> = self
            .dirs
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let line = if self.selected.contains(&i) {
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
            self.selected.len()
        ))
        .centered();
        frame.render_widget(dir_info, chunks[2]);
        let help = Paragraph::new(self.screen.help())
            .centered()
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[3]);
    }

    fn render_create_mr(&mut self, frame: &mut Frame) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Style};
        use ratatui::widgets::{Block, List, ListItem, Paragraph};
        let selected_dirs: Vec<&String> = self
            .selected
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
            Paragraph::new(format!("Title: {}", self.mr_title))
                .style(Style::default().bg(Color::Blue).fg(Color::White))
        } else {
            Paragraph::new(format!("Title: {}", self.mr_title))
        };
        let desc_input = if self.input_focus == InputFocus::Description {
            Paragraph::new(format!("Description: {}", self.mr_description))
                .style(Style::default().bg(Color::Blue).fg(Color::White))
        } else {
            Paragraph::new(format!("Description: {}", self.mr_description))
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

    fn render_select_reviewers(&mut self, frame: &mut Frame) {
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
            .selected
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
            Screen::SelectReviewers => self.on_key_event_select_reviewers(key),
            Screen::Overview => self.on_key_event_overview(key),
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
                if self.selected.contains(&self.selected_index) {
                    self.selected.remove(&self.selected_index);
                } else {
                    self.selected.insert(self.selected_index);
                }
            }
            (_, KeyCode::Enter) => {
                if !self.selected.is_empty() {
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
                if self.input_focus == InputFocus::Label {
                    self.screen = Screen::SelectReviewers;
                } else if self.input_focus == InputFocus::Description
                    || self.input_focus == InputFocus::Title
                {
                    self.screen = Screen::SelectReviewers;
                }
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
                self.screen = Screen::Overview;
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
                // Aggregate data as string
                let selected_dirs: Vec<&String> = self
                    .selected
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
                let _data = format!(
                    "Repositories: {}\nTitle: {}\nDescription: {}\nReviewers: {}",
                    selected_dirs
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    self.mr_title,
                    self.mr_description,
                    selected_reviewers
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                // Run placeholder shell command and exit
                std::process::Command::new("sh")
                    .arg("-c")
                    // .arg(format!("echo '{}'", data.replace("'", "''")))
                    .arg("terminal-notifier -sound default -message 'Merge Request Created'")
                    .status()
                    .ok();
                self.quit();
            }
            KeyCode::Char('n') => {
                self.screen = Screen::SelectReviewers;
            }
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}

/// Loads reviewers from a TOML file.
///
/// This function simulates loading reviewers from a file. In a real application, you would
/// read from an actual TOML file and parse the contents.
fn load_reviewers_and_labels_from_toml() -> Config {
    let path = "multimr.toml";

    let content = std::fs::read_to_string(path).unwrap_or_default();

    #[derive(Deserialize)]
    struct ConfigToml {
        reviewers: Option<Vec<String>>,
        labels: Option<HashMap<String, String>>,
        working_dir: Option<String>,
    }

    // if the entire parsing fails return a config with None values
    let parsed: ConfigToml = toml::from_str(&content).unwrap_or(ConfigToml {
        reviewers: None,
        labels: None,
        working_dir: None,
    });

    // check if a root is specified in toml, if not use current directory
    let working_dir_str = parsed.working_dir.unwrap_or_else(|| {
        ".".to_string() // default to current directory if not specified
    });

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
    }
}
