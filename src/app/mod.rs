//! Holds main application and rendering logic for the Multi MR CLI tool.
use std::fs;
use std::{collections::HashSet, process::Stdio};

use color_eyre::Result;

use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, List, ListItem, Paragraph, Widget},
};

use crate::config::Config;
use crate::merge_request;

mod input;

#[derive(Debug, Default)]
pub(crate) enum Screens {
    #[default]
    RepoSelection,
    CreateMR,
    ReviewerSelection,
    Finalize,
}

impl Screens {
    pub(crate) fn help(&self) -> &'static str {
        match self {
            Screens::RepoSelection => "↑/↓/j/k: Move  Space: Select  Enter: Next  q/Esc: Quit",
            Screens::CreateMR => "Tab: Switch field  ↑/↓/j/k: Select Label  Enter: Next  Esc: Back",
            Screens::ReviewerSelection => "↑/↓/j/k: Move   Space:  Select  Enter: Next  Esc: Back",
            Screens::Finalize => "y/Enter: Confirm  n/Esc: Back",
        }
    }

    pub(crate) fn title(&self) -> &'static str {
        match self {
            Screens::RepoSelection => "Select Repos",
            Screens::CreateMR => "Describe",
            Screens::ReviewerSelection => "Add Reviewers",
            Screens::Finalize => "Finalize",
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
    /// List of current branches in the selected directories.
    pub(crate) branches: Vec<String>,
    /// Indices of selected directories
    pub(crate) selected_repos: HashSet<usize>,
    /// Currently highlighted directory index
    pub(crate) selected_index: usize,
    /// Current screen (stage) of the application
    pub(crate) screen: Screens,
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

            let mut valid_dirs = Vec::new();
            for dir in &app.dirs {
                // Check if the directory is a git repository
                if std::process::Command::new("git")
                    .arg("rev-parse")
                    .arg("--is-inside-work-tree")
                    .current_dir(app.config.working_dir.join(dir))
                    .stderr(Stdio::null())
                    .stdout(Stdio::null())
                    .status()
                    .is_ok()
                {
                    // If it is, add it to the list of valid directories
                    valid_dirs.push(dir.clone());
                }
            }
            app.dirs = valid_dirs;

            for dir in app.dirs.iter() {
                // Check if the directory is a git repository
                if let Ok(current_branch_output) = std::process::Command::new("git")
                    .arg("branch")
                    .arg("--show-current")
                    .current_dir(app.config.working_dir.join(dir))
                    .output()
                {
                    app.branches.push(
                        String::from_utf8_lossy(&current_branch_output.stdout)
                            .trim()
                            .to_string(),
                    )
                }
            }
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
            Screens::RepoSelection => self.render_repo_selection(inner_area, frame.buffer_mut()),
            Screens::CreateMR => self.render_create_mr(inner_area, frame.buffer_mut()),
            Screens::ReviewerSelection => {
                self.render_reviewer_selection(inner_area, frame.buffer_mut())
            }
            Screens::Finalize => self.render_overview(inner_area, frame.buffer_mut()),
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
                    format!(
                        "[x] {} ({})",
                        d,
                        self.branches.get(i).unwrap_or(&"???".to_string())
                    )
                } else {
                    format!(
                        "[ ] {} ({})",
                        d,
                        self.branches.get(i).unwrap_or(&"???".to_string())
                    )
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

    /// Set running to false to quit the application.
    pub(crate) fn quit(&mut self) {
        self.running = false;
    }

    pub(crate) fn quit_completed(&mut self) {
        self.user_input_completed = true;
        self.running = false;
    }
}
