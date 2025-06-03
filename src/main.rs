use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    style::Stylize,
    text::Line,
    widgets::{Block, List},
};
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::fs;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}

#[derive(Debug)]
enum Screen {
    RepoSelection,
    CreateMR,
    SelectReviewers,
    Overview,
}

impl Default for Screen {
    fn default() -> Self {
        Screen::RepoSelection
    }
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
    /// List of reviewers
    reviewers: Vec<String>,
    /// Indices of selected reviewers
    selected_reviewers: HashSet<usize>,
    /// Currently highlighted reviewer index
    reviewer_index: usize,
    labels: Vec<(String, String)>, // (key, value)
    selected_label: Option<usize>,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum InputFocus {
    #[default]
    Title,
    Description,
    Label,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        let mut app = Self {
            screen: Screen::RepoSelection,
            ..Default::default()
        };
        // Populate dirs with all directories in the current working directory
        if let Ok(cwd) = env::current_dir() {
            if let Ok(entries) = fs::read_dir(cwd) {
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
        }
        app.selected_index = 0;
        app.selected = HashSet::new();
        // Load reviewers from reviewers.toml
        let (reviewers, labels) = load_reviewers_and_labels_from_toml();
        app.reviewers = reviewers;
        app.labels = labels;
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
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    fn render(&mut self, frame: &mut Frame) {
        match self.screen {
            Screen::RepoSelection => self.render_selection(frame),
            Screen::CreateMR => self.render_create_mr(frame),
            Screen::SelectReviewers => self.render_select_reviewers(frame),
            Screen::Overview => self.render_overview(frame),
        }
    }

    fn render_selection(&mut self, frame: &mut Frame) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Style};
        use ratatui::widgets::{ListItem, Paragraph};
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
                Constraint::Min(1),
                Constraint::Length(1), // for description
                Constraint::Length(1), // for key help bar
            ])
            .split(frame.area());
        frame.render_widget(list, chunks[0]);
        let desc = Paragraph::new("Select repositories to create MR for").centered();
        frame.render_widget(desc, chunks[1]);
        let help = Paragraph::new("↑/↓: Move  Space: Select  Enter: Next  q/Esc/Ctrl+C: Quit")
            .centered()
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[2]);
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
            .labels
            .iter()
            .enumerate()
            .map(|(i, (k, v))| {
                let marker = if Some(i) == self.selected_label {
                    "(x)"
                } else {
                    "( )"
                };
                let line = format!("{} {}: {}", marker, k, v);
                let mut item = ListItem::new(line);
                if self.input_focus == InputFocus::Label && Some(i) == self.selected_label {
                    item = item.style(Style::default().fg(Color::Yellow).bg(Color::Blue));
                } else if Some(i) == self.selected_label {
                    item = item.style(Style::default().fg(Color::Yellow));
                } else if self.input_focus == InputFocus::Label
                    && self.selected_label.is_none()
                    && i == 0
                {
                    item = item.style(Style::default().bg(Color::Blue));
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
        let help = Paragraph::new("Tab: Switch field  ↑/↓: Label  Space/Enter: Select Label  Type: Input  Backspace: Delete  Esc: Back").centered().style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, layout[5]);
    }

    fn render_select_reviewers(&mut self, frame: &mut Frame) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Color, Style};
        use ratatui::widgets::{ListItem, Paragraph};
        let title = Line::from("Select Reviewers").bold().blue().centered();
        let items: Vec<ListItem> = self
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
        let help = Paragraph::new("↑/↓: Move  Space: Select  Enter: Finish  Esc: Back")
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
            .filter_map(|i| self.reviewers.get(i))
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
        let help = Paragraph::new("y: Confirm  n: Back")
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
            (_, KeyCode::Down) => {
                if !self.dirs.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.dirs.len();
                }
            }
            (_, KeyCode::Up) => {
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
                InputFocus::Label => {}
            },
            KeyCode::Down => {
                if self.input_focus == InputFocus::Label && !self.labels.is_empty() {
                    let idx = self.selected_label.unwrap_or(0);
                    self.selected_label = Some((idx + 1) % self.labels.len());
                }
            }
            KeyCode::Up => {
                if self.input_focus == InputFocus::Label && !self.labels.is_empty() {
                    let idx = self.selected_label.unwrap_or(0);
                    self.selected_label = Some(if idx == 0 {
                        self.labels.len() - 1
                    } else {
                        idx - 1
                    });
                }
            }
            KeyCode::Enter => {
                if self.input_focus == InputFocus::Label {
                    if self.selected_label.is_none() && !self.labels.is_empty() {
                        self.selected_label = Some(0);
                    }
                    if self.selected_label.is_some() {
                        self.screen = Screen::SelectReviewers;
                    }
                } else if self.input_focus == InputFocus::Description
                    || self.input_focus == InputFocus::Title
                {
                    if self.selected_label.is_some() {
                        self.screen = Screen::SelectReviewers;
                    }
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
            KeyCode::Down => {
                if !self.reviewers.is_empty() {
                    self.reviewer_index = (self.reviewer_index + 1) % self.reviewers.len();
                }
            }
            KeyCode::Up => {
                if !self.reviewers.is_empty() {
                    if self.reviewer_index == 0 {
                        self.reviewer_index = self.reviewers.len() - 1;
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
                    .filter_map(|i| self.reviewers.get(i))
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
fn load_reviewers_and_labels_from_toml() -> (Vec<String>, Vec<(String, String)>) {
    let path = "multimr.toml";
    let content = std::fs::read_to_string(path).unwrap_or_default();
    #[derive(Deserialize)]
    struct ConfigToml {
        reviewers: Option<Vec<String>>,
        labels: Option<std::collections::BTreeMap<String, String>>,
    }
    let parsed: ConfigToml = toml::from_str(&content).unwrap_or(ConfigToml {
        reviewers: None,
        labels: None,
    });
    let reviewers = parsed.reviewers.unwrap_or_default();
    let labels = parsed
        .labels
        .map(|m| m.into_iter().collect())
        .unwrap_or_default();
    (reviewers, labels)
}
