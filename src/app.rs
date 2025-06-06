use std::collections::HashSet;
use std::fs;

use color_eyre::Result;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use crate::config::Config;
use crate::merge_request;

#[derive(Debug, Default)]
pub(crate) enum Screen {
    #[default]
    RepoSelection,
    CreateMR,
    ReviewerSelection,
    Finalize,
}

impl Screen {
    pub(crate) fn help(&self) -> &'static str {
        match self {
            Screen::RepoSelection => "↑/↓/j/k: Move  Space: Select  Enter: Next  q/Esc: Quit",
            Screen::CreateMR => "Tab: Switch field  ↑/↓/j/k: Select Label  Enter: Next  Esc: Back",
            Screen::ReviewerSelection => "↑/↓/j/k: Move   Space:  Select  Enter: Next  Esc: Back",
            Screen::Finalize => "y/Enter: Confirm  n/Esc: Back",
        }
    }

    pub(crate) fn title(&self) -> &'static str {
        match self {
            Screen::RepoSelection => "Select Repos",
            Screen::CreateMR => "Describe",
            Screen::ReviewerSelection => "Add Reviewers",
            Screen::Finalize => "Finalize",
        }
    }
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    /// Configuration loaded from `multimr.toml`
    pub(crate) config: Config,
    /// Is the application running?
    pub(crate) running: bool,
    /// List of directories in the current working directory.
    pub(crate) dirs: Vec<String>,
    /// Indices of selected directories
    pub(crate) selected_repos: HashSet<usize>,
    /// Currently highlighted directory index
    pub(crate) selected_index: usize,
    /// Current screen (stage) of the application
    pub(crate) screen: Screen,
    /// Title of the merge requests to be created
    pub(crate) mr_title: String,
    /// Description of the merge requests to be created
    pub(crate) mr_description: String,
    /// Indices of selected reviewers
    pub(crate) selected_reviewers: HashSet<usize>,
    /// Currently selected label index
    pub(crate) selected_label: usize,

    /// Whether the user has completed the input process and did not quit early
    pub(crate) user_input_completed: bool,

    // TODO: move stuff only relevant to specific screens into a separate struct
    /// Input focus specifically for the CreateMR screen
    pub(crate) input_focus: InputFocus,
    /// Currently highlighted reviewer index
    pub(crate) reviewer_index: usize,

    // TODO: move this out of here
    /// The merge request that is created at the end of the process
    pub(crate) mr: Option<merge_request::MergeRequest>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum InputFocus {
    #[default]
    Title,
    Description,
    Label,
}

impl App {
    pub(crate) fn new(config: Config) -> Self {
        let mut app = Self {
            config,
            selected_label: 0,
            selected_index: 0,
            ..Default::default()
        };

        // Populate dirs with all directories in the current working directory
        if let Ok(entries) = fs::read_dir(&app.config.working_dir) {
            // TODO: ensure we only show git directories
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
    pub(crate) fn run(mut self, mut terminal: DefaultTerminal) -> Result<Self> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(self)
    }

    /// This holds generic rendering, it calls screen specific rendering methods.
    /// Split the screen: main box + help footer at the bottom
    pub(crate) fn render(&mut self, frame: &mut Frame) {
        let [window, footer] = Layout::vertical([
            Constraint::Min(0),    // main area for the box
            Constraint::Length(1), // footer (help)
        ])
        .areas(frame.area());

        let title = Line::from(format!("Multi MR - {}", self.screen.title()))
            .bold()
            .blue()
            .centered();

        // Outer block for the whole screen (except help)
        let outer_block = Block::bordered().title(title);
        let inner_area = outer_block.inner(window);

        match self.screen {
            Screen::RepoSelection => self.render_repo_selection(inner_area, frame.buffer_mut()),
            Screen::CreateMR => self.render_create_mr(inner_area, frame.buffer_mut()),
            Screen::ReviewerSelection => {
                self.render_reviewer_selection(inner_area, frame.buffer_mut())
            }
            Screen::Finalize => self.render_overview(inner_area, frame.buffer_mut()),
        }

        outer_block.render(window, frame.buffer_mut());
        Paragraph::new(self.screen.help())
            .centered()
            .style(Style::default().fg(Color::DarkGray))
            .render(footer, frame.buffer_mut());
    }

    /// The repo selection shows a list of directories in the current working directory and which ones are selected.
    pub(crate) fn render_repo_selection(&mut self, window: Rect, buf: &mut Buffer) {
        let [repo_list_area, dir_info_area] = Layout::vertical([
            Constraint::Min(3),
            Constraint::Length(1), // for directory info
        ])
        .areas(window);

        let repos: Vec<ListItem> = self
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

        List::new(repos).render(repo_list_area, buf);

        Paragraph::new(format!(
            "Current directory: {} (Selected: {})",
            self.config.working_dir.display(),
            self.selected_repos.len()
        ))
        .centered()
        .render(dir_info_area, buf);
    }

    /// This screen allows the user to enter a title, description, and select labels for the merge request.
    pub(crate) fn render_create_mr(&mut self, window: Rect, buf: &mut Buffer) {
        let [
            dir_area,
            title_input_area,
            description_input_area,
            label_input_area,
        ] = Layout::vertical([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(5),
        ])
        .areas(window);

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

        Paragraph::new(format!("Repositories:\n{}", dirs_text)).render(dir_area, buf);

        Paragraph::new(self.mr_title.as_str())
            .style(if self.input_focus == InputFocus::Title {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            })
            .block(Block::bordered().title("Title"))
            .render(title_input_area, buf);

        Paragraph::new(self.mr_description.as_str())
            .style(if self.input_focus == InputFocus::Description {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            })
            .block(Block::bordered().title("Description"))
            .render(description_input_area, buf);

        let label_items: Vec<ListItem> = self
            .config
            .labels
            .iter()
            .enumerate()
            .map(|(i, (k, v))| {
                let marker = if i == self.selected_label {
                    "(x)"
                } else {
                    "( )"
                };
                let mut item = ListItem::new(format!("{} {}: {}", marker, k, v));
                if self.input_focus == InputFocus::Label && i == self.selected_label {
                    item = item.style(Style::default().fg(Color::Yellow).bg(Color::Blue));
                } else if i == self.selected_label {
                    item = item.style(Style::default().fg(Color::Yellow));
                }
                item
            })
            .collect();

        List::new(label_items)
            .block(Block::bordered().title("Gitlab Label"))
            .render(label_input_area, buf);
    }

    /// This screen allows the user to select reviewers for the merge request.
    pub(crate) fn render_reviewer_selection(&mut self, window: Rect, buf: &mut Buffer) {
        let [reviewer_area, assignee_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Min(1)]).areas(window);

        let items: Vec<ListItem> = self
            .config
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

        List::new(items).render(reviewer_area, buf);
        if let Some(assignee) = &self.config.assignee {
            Paragraph::new(format!("Assignee: {}", assignee))
                .style(Style::default().fg(Color::Green))
                .render(assignee_area, buf);
        } else {
            // If no assignee is set, show a placeholder
            Paragraph::new("No assignee set")
                .style(Style::default().fg(Color::Red))
                .render(assignee_area, buf);
        }
    }

    /// This screen shows an overview of selected configuration and prompts the user one final time.
    pub(crate) fn render_overview(&mut self, window: Rect, buf: &mut Buffer) {
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
            .filter_map(|i| self.config.reviewers.get(i))
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

        let [overview_area] = Layout::vertical([Constraint::Min(1)]).areas(window);

        Paragraph::new(format!(
            "Overview\n\nRepositories: {}\nTitle: {}\nDescription: {}\nReviewers: {}\n\nPress 'y' to confirm, 'n' to go back.",
            dirs_text, self.mr_title, self.mr_description, reviewers_text
        )).render(overview_area, buf);
    }

    /// Reads the crossterm events and updates the state of [`App`].
    pub(crate) fn handle_crossterm_events(&mut self) -> Result<()> {
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
    pub(crate) fn on_key_event(&mut self, key: KeyEvent) {
        // Handle global key events first
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                if key.modifiers == KeyModifiers::CONTROL {
                    self.quit();
                }
            }
            _ => {}
        }

        match self.screen {
            Screen::RepoSelection => self.on_key_event_selection(key),
            Screen::CreateMR => self.on_key_event_create_mr(key),
            Screen::ReviewerSelection => self.on_key_event_select_reviewers(key),
            Screen::Finalize => self.on_key_event_overview(key),
        }
    }

    pub(crate) fn on_key_event_selection(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.quit();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.dirs.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.dirs.len();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.dirs.is_empty() {
                    if self.selected_index == 0 {
                        self.selected_index = self.dirs.len() - 1;
                    } else {
                        self.selected_index -= 1;
                    }
                }
            }
            KeyCode::Char(' ') => {
                if self.selected_repos.contains(&self.selected_index) {
                    self.selected_repos.remove(&self.selected_index);
                } else {
                    self.selected_repos.insert(self.selected_index);
                }
            }
            KeyCode::Enter => {
                if !self.selected_repos.is_empty() {
                    self.screen = Screen::CreateMR;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn on_key_event_create_mr(&mut self, key: KeyEvent) {
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
                        if !self.config.labels.is_empty() {
                            let idx = self.selected_label;
                            self.selected_label = (idx + 1) % self.config.labels.len();
                        }
                    }
                    'k' => {
                        if !self.config.labels.is_empty() {
                            let idx = self.selected_label;
                            self.selected_label = if idx == 0 {
                                self.config.labels.len() - 1
                            } else {
                                idx - 1
                            };
                        }
                    }
                    _ => {}
                },
            },
            KeyCode::Down => {
                if self.input_focus == InputFocus::Label && !self.config.labels.is_empty() {
                    let idx = self.selected_label;
                    self.selected_label = (idx + 1) % self.config.labels.len();
                }
            }
            KeyCode::Up => {
                if self.input_focus == InputFocus::Label && !self.config.labels.is_empty() {
                    let idx = self.selected_label;
                    self.selected_label = if idx == 0 {
                        self.config.labels.len() - 1
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

    pub(crate) fn on_key_event_select_reviewers(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.config.reviewers.is_empty() {
                    self.reviewer_index = (self.reviewer_index + 1) % self.config.reviewers.len();
                }
            }
            KeyCode::Up | KeyCode::Char('h') => {
                if !self.config.reviewers.is_empty() {
                    if self.reviewer_index == 0 {
                        self.reviewer_index = self.config.reviewers.len() - 1;
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

    pub(crate) fn on_key_event_overview(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                self.mr = Some(merge_request::MergeRequest {
                    title: self.mr_title.clone(),
                    description: self.mr_description.clone(),
                    reviewers: self
                        .selected_reviewers
                        .iter()
                        .map(|&i| self.config.reviewers[i].clone())
                        .collect(),
                    labels: self
                        .config
                        .labels
                        .keys()
                        .nth(self.selected_label)
                        .map(|k| vec![k.clone()])
                        .unwrap_or_default(),
                    assignee: self.config.assignee.clone(),
                });

                self.quit_completed();
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.screen = Screen::ReviewerSelection;
            }
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    pub(crate) fn quit(&mut self) {
        self.running = false;
    }

    pub(crate) fn quit_completed(&mut self) {
        self.user_input_completed = true;
        self.running = false;
    }
}
