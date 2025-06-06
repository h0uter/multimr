use color_eyre::Result;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;

use crate::merge_request;

use super::App;
use super::InputFocus;
use super::Screen;

impl App {
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
}
