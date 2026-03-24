//! TUI application state and main event loop.
//!
//! Implements the core state machine for the terminal UI.  All view-specific
//! rendering lives in the `views` sub-module; this module owns the shared
//! `App` struct and the `run_tui` entry point.

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fixonce_core::{
    api::{dashboard::DashboardData, ApiClient},
    auth::token::TokenManager,
    memory::types::{Memory, SearchMemoryResponse},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;

use super::data::{self, AppMessage, DataState};
use super::views;

// ---------------------------------------------------------------------------
// Minimum terminal size (EC-35)
// ---------------------------------------------------------------------------

/// Minimum columns required to render the TUI without garbling.
pub const MIN_COLS: u16 = 80;
/// Minimum rows required to render the TUI without garbling.
pub const MIN_ROWS: u16 = 24;

// ---------------------------------------------------------------------------
// Input mode
// ---------------------------------------------------------------------------

/// Whether the TUI is in navigation mode or text-input mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Navigation,
    Input,
}

// ---------------------------------------------------------------------------
// Form mode
// ---------------------------------------------------------------------------

/// Whether the create form is in create or edit mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormMode {
    Create,
    Edit { memory_id: String },
}

// ---------------------------------------------------------------------------
// Search type
// ---------------------------------------------------------------------------

/// Search type for the Search view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchType {
    Hybrid,
    Fts,
    Vector,
}

impl SearchType {
    pub fn next(self) -> Self {
        match self {
            Self::Hybrid => Self::Fts,
            Self::Fts => Self::Vector,
            Self::Vector => Self::Hybrid,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hybrid => "hybrid",
            Self::Fts => "fts",
            Self::Vector => "vector",
        }
    }
}

// ---------------------------------------------------------------------------
// List mode
// ---------------------------------------------------------------------------

/// Dashboard memory list mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListMode {
    RecentlyCreated,
    RecentlyViewed,
    MostAccessed,
}

impl ListMode {
    pub fn next(self) -> Self {
        match self {
            Self::RecentlyCreated => Self::RecentlyViewed,
            Self::RecentlyViewed => Self::MostAccessed,
            Self::MostAccessed => Self::RecentlyCreated,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::RecentlyCreated => Self::MostAccessed,
            Self::RecentlyViewed => Self::RecentlyCreated,
            Self::MostAccessed => Self::RecentlyViewed,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::RecentlyCreated => "Recently Created",
            Self::RecentlyViewed => "Recently Viewed",
            Self::MostAccessed => "Most Accessed",
        }
    }
}

// ---------------------------------------------------------------------------
// Heatmap mode
// ---------------------------------------------------------------------------

/// Dashboard activity heatmap mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeatmapMode {
    Created,
    Read,
    Searched,
}

impl HeatmapMode {
    pub fn next(self) -> Self {
        match self {
            Self::Created => Self::Read,
            Self::Read => Self::Searched,
            Self::Searched => Self::Created,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Created => Self::Searched,
            Self::Read => Self::Created,
            Self::Searched => Self::Read,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Created => "Memories Created",
            Self::Read => "Memories Read",
            Self::Searched => "Searches Made",
        }
    }

    pub fn action(self) -> &'static str {
        match self {
            Self::Created => "memory.created",
            Self::Read => "memory.accessed",
            Self::Searched => "memory.searched",
        }
    }
}

// ---------------------------------------------------------------------------
// View enum
// ---------------------------------------------------------------------------

/// Which screen the TUI is currently displaying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Search,
    MemoryDetail(String),
    CreateForm,
    Keys,
}

// ---------------------------------------------------------------------------
// Form field state
// ---------------------------------------------------------------------------

/// Which field in the create-memory form is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Title,
    Content,
    Summary,
    MemoryType,
    Source,
    Language,
}

impl FormField {
    pub fn next(self) -> Self {
        match self {
            Self::Title => Self::Content,
            Self::Content => Self::Summary,
            Self::Summary => Self::MemoryType,
            Self::MemoryType => Self::Source,
            Self::Source => Self::Language,
            Self::Language => Self::Title,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Title => Self::Language,
            Self::Content => Self::Title,
            Self::Summary => Self::Content,
            Self::MemoryType => Self::Summary,
            Self::Source => Self::MemoryType,
            Self::Language => Self::Source,
        }
    }
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

/// Full application state shared across all views.
pub struct App {
    /// Which view is currently shown.
    pub current_view: View,
    /// Set to `true` to break the event loop and exit.
    pub should_quit: bool,
    /// Text typed into the search / filter bar.
    pub search_query: String,
    /// Memories fetched from the API (used by list and detail views).
    pub memories: Vec<Memory>,
    /// Row index currently highlighted in list views.
    pub selected_index: usize,
    /// Base URL for API calls (used when the event loop fetches data from the backend).
    #[allow(dead_code)]
    pub api_url: String,
    /// Transient status message shown in the status bar.
    pub status_message: Option<String>,
    /// Activity log entries (for the Activity view).
    pub activity_entries: Vec<String>,
    /// Scroll offset for the detail / activity views.
    pub scroll_offset: usize,
    /// Whether the terminal is large enough to render the UI (EC-35).
    pub terminal_too_small: bool,
    // --- Create-form fields ---
    pub form_field: FormField,
    pub form_title: String,
    pub form_content: String,
    pub form_summary: String,
    pub form_memory_type: String,
    pub form_source: String,
    pub form_language: String,
    // --- New fields ---
    /// Whether the TUI is in navigation mode or text-input mode.
    pub input_mode: InputMode,
    /// Previous view for back-navigation.
    pub previous_view: Option<View>,
    /// Whether the create form is in create or edit mode.
    pub form_mode: FormMode,
    /// Search type for the Search view.
    pub search_type: SearchType,
    /// Dashboard memory list mode.
    pub list_mode: ListMode,
    /// Dashboard activity heatmap mode.
    pub heatmap_mode: HeatmapMode,
    /// Async dashboard data.
    pub dashboard_data: DataState<DashboardData>,
    /// Async search results.
    pub search_results: DataState<SearchMemoryResponse>,
    /// API client for spawning async tasks.
    pub api_client: Option<ApiClient>,
    /// Sender for async messages.
    pub tx: mpsc::UnboundedSender<AppMessage>,
    /// Receiver for async messages.
    pub rx: mpsc::UnboundedReceiver<AppMessage>,
}

impl App {
    /// Construct a new [`App`] with sensible defaults.
    #[must_use]
    pub fn new(
        api_url: String,
        client: Option<ApiClient>,
        tx: mpsc::UnboundedSender<AppMessage>,
        rx: mpsc::UnboundedReceiver<AppMessage>,
    ) -> Self {
        Self {
            current_view: View::Dashboard,
            should_quit: false,
            search_query: String::new(),
            memories: Vec::new(),
            selected_index: 0,
            api_url,
            status_message: None,
            activity_entries: Vec::new(),
            scroll_offset: 0,
            terminal_too_small: false,
            form_field: FormField::Title,
            form_title: String::new(),
            form_content: String::new(),
            form_summary: String::new(),
            form_memory_type: String::from("gotcha"),
            form_source: String::from("manual"),
            form_language: String::new(),
            input_mode: InputMode::Navigation,
            previous_view: None,
            form_mode: FormMode::Create,
            search_type: SearchType::Fts,
            list_mode: ListMode::RecentlyCreated,
            heatmap_mode: HeatmapMode::Created,
            dashboard_data: DataState::default(),
            search_results: DataState::default(),
            api_client: client,
            tx,
            rx,
        }
    }

    /// Navigate to a different view, resetting per-view state as needed.
    pub fn navigate_to(&mut self, view: View) {
        // Reset scroll when changing views.
        if self.current_view != view {
            self.scroll_offset = 0;
            self.previous_view = Some(self.current_view.clone());
        }
        self.current_view = view;
        self.status_message = None;
    }

    /// Move the list selection up by one.
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move the list selection down by one, clamped to the list length.
    pub fn select_next(&mut self, list_len: usize) {
        if list_len > 0 && self.selected_index + 1 < list_len {
            self.selected_index += 1;
        }
    }

    /// Scroll up in scrollable views.
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Scroll down in scrollable views.
    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }

    /// Apply a key event globally and dispatch view-specific handling.
    pub fn handle_key_event(&mut self, key: KeyEvent) {
        // Global: Ctrl+C quit from any view.
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        // If the terminal is too small, only accept 'q' / Ctrl-C to quit.
        if self.terminal_too_small {
            if key.code == KeyCode::Char('q') {
                self.should_quit = true;
            }
            return;
        }

        match self.input_mode {
            InputMode::Navigation => self.handle_navigation_key(key),
            InputMode::Input => self.handle_input_key(key),
        }
    }

    // -----------------------------------------------------------------------
    // Navigation-mode key handling
    // -----------------------------------------------------------------------

    fn handle_navigation_key(&mut self, key: KeyEvent) {
        // View-specific dispatch (may consume the key).
        match self.current_view.clone() {
            View::Dashboard => self.handle_dashboard_key(key),
            View::Search => self.handle_search_key(key),
            View::MemoryDetail(_) => self.handle_memory_detail_key(key),
            View::CreateForm => self.handle_create_form_key(key),
            View::Keys => self.handle_keys_key(key),
        }
    }

    // -----------------------------------------------------------------------
    // Input-mode key handling
    // -----------------------------------------------------------------------

    fn handle_input_key(&mut self, key: KeyEvent) {
        match self.current_view {
            View::Search => self.handle_search_input_key(key),
            View::CreateForm => self.handle_create_form_input_key(key),
            _ => {
                // Unexpected input mode — escape back to navigation.
                self.input_mode = InputMode::Navigation;
            }
        }
    }

    /// Handle key events while in input mode on the Search view.
    fn handle_search_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                self.search_query.push(c);
            }
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            KeyCode::Enter => {
                // Fire search and switch back to navigation.
                self.input_mode = InputMode::Navigation;
                if let Some(client) = &self.api_client {
                    data::search_memories_async(
                        client.clone(),
                        self.search_query.clone(),
                        self.search_type.label().to_owned(),
                        self.tx.clone(),
                    );
                }
            }
            KeyCode::Tab => {
                self.search_type = self.search_type.next();
            }
            KeyCode::Esc => {
                self.search_query.clear();
                self.input_mode = InputMode::Navigation;
            }
            _ => {}
        }
    }

    /// Handle key events while in input mode on the Create form.
    fn handle_create_form_input_key(&mut self, key: KeyEvent) {
        // Ctrl+S submits.
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('s') {
            self.status_message = Some(
                "Ctrl+S detected — submit via `fixonce create` CLI for full pipeline.".to_owned(),
            );
            return;
        }
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Navigation;
                self.navigate_to(View::Dashboard);
            }
            KeyCode::Tab => {
                self.form_field = self.form_field.next();
            }
            KeyCode::BackTab => {
                self.form_field = self.form_field.prev();
            }
            KeyCode::Char(c) => {
                let field = self.get_active_form_field_mut();
                field.push(c);
            }
            KeyCode::Enter => {
                // Insert newline in multiline fields (content, summary).
                if matches!(self.form_field, FormField::Content | FormField::Summary) {
                    let field = self.get_active_form_field_mut();
                    field.push('\n');
                }
            }
            KeyCode::Backspace => {
                let field = self.get_active_form_field_mut();
                field.pop();
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Global navigation helpers
    // -----------------------------------------------------------------------

    /// Handle numeric / tab shortcuts that are shared across most views.
    ///
    /// Returns `true` if a global shortcut consumed the key.
    fn handle_global_nav(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                true
            }
            KeyCode::Char('1') => {
                self.navigate_to(View::Dashboard);
                true
            }
            KeyCode::Char('2') => {
                self.navigate_to(View::Search);
                true
            }
            KeyCode::Char('3') => {
                self.navigate_to(View::CreateForm);
                true
            }
            KeyCode::Char('4') => {
                self.navigate_to(View::Keys);
                true
            }
            _ => false,
        }
    }

    // -----------------------------------------------------------------------
    // Per-view key handlers (navigation mode)
    // -----------------------------------------------------------------------

    fn handle_dashboard_key(&mut self, key: KeyEvent) {
        if self.handle_global_nav(key) {
            return;
        }
        match key.code {
            KeyCode::Down => {
                let len = self.filtered_memories().len();
                self.select_next(len);
            }
            KeyCode::Up => self.select_prev(),
            KeyCode::Enter => {
                if let Some(mem) = self.filtered_memories().get(self.selected_index).copied() {
                    let id = mem.id.clone();
                    self.navigate_to(View::MemoryDetail(id));
                }
            }
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        if self.handle_global_nav(key) {
            return;
        }
        match key.code {
            KeyCode::Char('i') | KeyCode::Char('/') => {
                self.input_mode = InputMode::Input;
            }
            KeyCode::Down => {
                self.select_next(usize::MAX);
            }
            KeyCode::Up => self.select_prev(),
            KeyCode::Esc => {
                self.navigate_to(View::Dashboard);
            }
            _ => {}
        }
    }

    fn handle_memory_detail_key(&mut self, key: KeyEvent) {
        if self.handle_global_nav(key) {
            return;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.scroll_up(),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_down(),
            KeyCode::Esc | KeyCode::Backspace => {
                // Navigate back to previous view or Dashboard.
                let back = self
                    .previous_view
                    .take()
                    .unwrap_or(View::Dashboard);
                self.navigate_to(back);
            }
            _ => {}
        }
    }

    fn handle_create_form_key(&mut self, key: KeyEvent) {
        // Ctrl+S submits (handled in the event loop, not here).
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('s') {
            self.status_message = Some(
                "Ctrl+S detected — submit via `fixonce create` CLI for full pipeline.".to_owned(),
            );
            return;
        }
        if key.code == KeyCode::Esc {
            self.navigate_to(View::Dashboard);
            return;
        }
        if key.code == KeyCode::Tab {
            self.form_field = self.form_field.next();
            return;
        }
        if key.modifiers == KeyModifiers::SHIFT && key.code == KeyCode::BackTab {
            self.form_field = self.form_field.prev();
            return;
        }
        // Enter input mode for typing into form fields.
        if key.code == KeyCode::Char('i') || key.code == KeyCode::Enter {
            self.input_mode = InputMode::Input;
            return;
        }
        // Global nav (numbers) still work.
        if self.handle_global_nav(key) {
            return;
        }
        // Type into the active field (for single-character keys in navigation mode).
        match key.code {
            KeyCode::Char(c) => {
                let field = self.get_active_form_field_mut();
                field.push(c);
            }
            KeyCode::Backspace => {
                let field = self.get_active_form_field_mut();
                field.pop();
            }
            _ => {}
        }
    }

    fn handle_keys_key(&mut self, key: KeyEvent) {
        if self.handle_global_nav(key) {
            return;
        }
        match key.code {
            KeyCode::Up => self.select_prev(),
            KeyCode::Down => {
                // Keys list length is not known here; use a generous upper bound.
                self.select_next(usize::MAX);
            }
            KeyCode::Esc => self.navigate_to(View::Dashboard),
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Message handling
    // -----------------------------------------------------------------------

    /// Handle an async message from a background task.
    pub fn handle_message(&mut self, msg: AppMessage) {
        match msg {
            AppMessage::DashboardLoaded(Ok(data)) => {
                self.dashboard_data = DataState::Loaded(data);
            }
            AppMessage::DashboardLoaded(Err(e)) => {
                self.dashboard_data = DataState::Error(e);
            }
            AppMessage::MemoriesLoaded(Ok(memories)) => {
                self.memories = memories;
            }
            AppMessage::MemoriesLoaded(Err(e)) => {
                self.status_message = Some(format!("Failed to load memories: {e}"));
            }
            AppMessage::SearchResults(Ok(results)) => {
                self.search_results = DataState::Loaded(results);
            }
            AppMessage::SearchResults(Err(e)) => {
                self.search_results = DataState::Error(e);
            }
            AppMessage::SubmitResult(Ok(msg)) => {
                self.status_message = Some(msg);
            }
            AppMessage::SubmitResult(Err(e)) => {
                self.status_message = Some(format!("Submit failed: {e}"));
            }
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Return the memories that match the current search query (case-insensitive).
    #[must_use]
    pub fn filtered_memories(&self) -> Vec<&Memory> {
        let q = self.search_query.to_lowercase();
        if q.is_empty() {
            self.memories.iter().collect()
        } else {
            self.memories
                .iter()
                .filter(|m| {
                    m.title.to_lowercase().contains(&q)
                        || m.summary.to_lowercase().contains(&q)
                        || m.content.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    /// Get a mutable reference to the currently active create-form field.
    fn get_active_form_field_mut(&mut self) -> &mut String {
        match self.form_field {
            FormField::Title => &mut self.form_title,
            FormField::Content => &mut self.form_content,
            FormField::Summary => &mut self.form_summary,
            FormField::MemoryType => &mut self.form_memory_type,
            FormField::Source => &mut self.form_source,
            FormField::Language => &mut self.form_language,
        }
    }
}

// ---------------------------------------------------------------------------
// TUI entry point
// ---------------------------------------------------------------------------

/// Launch the interactive TUI.
///
/// # Errors
///
/// Returns an error if the terminal cannot be set up or if an I/O error occurs
/// during the event loop.
///
/// # Panics
///
/// Panics only if the system is unable to create a `CrosstermBackend`, which
/// is an unrecoverable state.
pub async fn run_tui(api_url: &str) -> Result<()> {
    // EC-36: Refuse to launch in non-TTY environments.
    // crossterm's `enable_raw_mode` will fail or behave incorrectly on a
    // non-TTY, but we want a clear error message before we attempt it.
    if !crossterm::tty::IsTty::is_tty(&io::stdout()) {
        anyhow::bail!(
            "fixonce tui requires an interactive terminal (TTY). \
             Refusing to launch in a non-TTY environment."
        );
    }

    // Check auth before entering raw mode.
    let mgr = TokenManager::new();
    let token = match mgr.load_token() {
        Ok(Some(t)) if !mgr.is_expired(&t) => t,
        _ => {
            // Task 6 will implement show_unauthenticated_splash.
            anyhow::bail!("Not authenticated — run `fixonce login` first");
        }
    };

    let client = ApiClient::new(api_url)
        .context("Failed to create API client")?
        .with_token(&token);

    let (tx, rx) = mpsc::unbounded_channel();

    // Kick off initial data fetches.
    data::fetch_dashboard_async(client.clone(), tx.clone());
    data::fetch_memories_async(client.clone(), tx.clone());

    // Setup terminal.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(api_url.to_owned(), Some(client), tx, rx);

    // Main event loop.
    let result = run_event_loop(&mut terminal, &mut app);

    // Always restore the terminal, even on error.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

/// Inner event loop — separated so terminal cleanup always runs.
fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Drain async messages.
        while let Ok(msg) = app.rx.try_recv() {
            app.handle_message(msg);
        }

        // Check terminal size (EC-35).
        let size = terminal.size()?;
        app.terminal_too_small = size.width < MIN_COLS || size.height < MIN_ROWS;

        // Draw frame.
        terminal.draw(|f| {
            if app.terminal_too_small {
                views::too_small::render(f, f.area());
            } else {
                match &app.current_view {
                    View::Dashboard => views::dashboard::render(f, app),
                    View::Search => views::dashboard::render(f, app), // temporary until Task 10
                    View::MemoryDetail(_) => views::memory_detail::render(f, app),
                    View::CreateForm => views::create_form::render(f, app),
                    View::Keys => views::keys::render(f, app),
                }
            }
        })?;

        // Poll for events with a 250 ms tick for auto-refresh.
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key_event(key);
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
