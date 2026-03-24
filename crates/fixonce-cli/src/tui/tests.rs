//! State-transition tests for the TUI `App`.
//!
//! These tests exercise key-event handling and state mutations without
//! rendering anything to a terminal, making them safe to run in CI.

#![cfg(test)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fixonce_core::memory::types::{
    EmbeddingStatus, Memory, MemoryType, PipelineStatus, SourceType,
};
use tokio::sync::mpsc;

use super::app::{App, FormField, InputMode, View, MIN_COLS, MIN_ROWS};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn shift(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

/// Create a test App with a no-op channel (no API client).
fn test_app() -> App {
    let (tx, rx) = mpsc::unbounded_channel();
    App::new(String::new(), None, tx, rx)
}

/// Create a test App with a specific API URL.
fn test_app_with_url(url: &str) -> App {
    let (tx, rx) = mpsc::unbounded_channel();
    App::new(url.to_owned(), None, tx, rx)
}

fn make_memory(id: &str, title: &str, decay: f64) -> Memory {
    Memory {
        id: id.to_owned(),
        title: title.to_owned(),
        content: format!("content of {title}"),
        summary: format!("summary of {title}"),
        memory_type: MemoryType::Gotcha,
        source_type: SourceType::Manual,
        language: None,
        compact_pragma: None,
        compact_compiler: None,
        midnight_js: None,
        indexer_version: None,
        node_version: None,
        source_url: None,
        repo_url: None,
        task_summary: None,
        session_id: None,
        decay_score: decay,
        reinforcement_score: 1.0,
        last_accessed_at: None,
        embedding_status: EmbeddingStatus::Complete,
        pipeline_status: PipelineStatus::Complete,
        deleted_at: None,
        created_at: "2024-01-01T00:00:00Z".to_owned(),
        updated_at: "2024-01-02T00:00:00Z".to_owned(),
        created_by: "user-1".to_owned(),
        anti_memory: None,
    }
}

// ---------------------------------------------------------------------------
// App::new
// ---------------------------------------------------------------------------

#[test]
fn new_initialises_default_view() {
    let app = test_app_with_url("https://example.com");
    assert_eq!(app.current_view, View::Dashboard);
    assert!(!app.should_quit);
    assert!(app.search_query.is_empty());
    assert_eq!(app.input_mode, InputMode::Navigation);
}

// ---------------------------------------------------------------------------
// Global Ctrl+C quits from any view
// ---------------------------------------------------------------------------

#[test]
fn ctrl_c_quits_from_dashboard() {
    let mut app = test_app();
    app.handle_key_event(ctrl(KeyCode::Char('c')));
    assert!(app.should_quit);
}

#[test]
fn ctrl_c_quits_from_search() {
    let mut app = test_app();
    app.navigate_to(View::Search);
    app.handle_key_event(ctrl(KeyCode::Char('c')));
    assert!(app.should_quit);
}

// ---------------------------------------------------------------------------
// 'q' quits
// ---------------------------------------------------------------------------

#[test]
fn q_quits_from_dashboard() {
    let mut app = test_app();
    app.handle_key_event(key(KeyCode::Char('q')));
    assert!(app.should_quit);
}

// ---------------------------------------------------------------------------
// Numeric navigation shortcuts
// ---------------------------------------------------------------------------

#[test]
fn pressing_2_navigates_to_search() {
    let mut app = test_app();
    app.handle_key_event(key(KeyCode::Char('2')));
    assert_eq!(app.current_view, View::Search);
}

#[test]
fn pressing_3_navigates_to_create_form() {
    let mut app = test_app();
    app.handle_key_event(key(KeyCode::Char('3')));
    assert_eq!(app.current_view, View::CreateForm);
}

#[test]
fn pressing_4_navigates_to_keys() {
    let mut app = test_app();
    app.handle_key_event(key(KeyCode::Char('4')));
    assert_eq!(app.current_view, View::Keys);
}

#[test]
fn pressing_1_from_search_returns_to_dashboard() {
    let mut app = test_app();
    app.navigate_to(View::Search);
    app.handle_key_event(key(KeyCode::Char('1')));
    assert_eq!(app.current_view, View::Dashboard);
}

// ---------------------------------------------------------------------------
// Dashboard: navigation
// ---------------------------------------------------------------------------

#[test]
fn dashboard_arrow_keys_navigate() {
    let mut app = test_app();
    app.memories = vec![
        make_memory("a", "Alpha", 90.0),
        make_memory("b", "Beta", 70.0),
        make_memory("c", "Gamma", 50.0),
    ];

    assert_eq!(app.selected_index, 0);
    app.handle_key_event(key(KeyCode::Down));
    assert_eq!(app.selected_index, 1);
    app.handle_key_event(key(KeyCode::Down));
    assert_eq!(app.selected_index, 2);
    // Cannot go past last.
    app.handle_key_event(key(KeyCode::Down));
    assert_eq!(app.selected_index, 2);
    app.handle_key_event(key(KeyCode::Up));
    assert_eq!(app.selected_index, 1);
}

#[test]
fn dashboard_enter_navigates_to_detail() {
    let mut app = test_app();
    app.memories = vec![make_memory("mem-001", "Alpha", 90.0)];

    app.handle_key_event(key(KeyCode::Enter));
    assert_eq!(app.current_view, View::MemoryDetail("mem-001".to_owned()));
}

// ---------------------------------------------------------------------------
// Search: input mode
// ---------------------------------------------------------------------------

#[test]
fn search_i_enters_input_mode() {
    let mut app = test_app();
    app.navigate_to(View::Search);
    app.handle_key_event(key(KeyCode::Char('i')));
    assert_eq!(app.input_mode, InputMode::Input);
}

#[test]
fn search_input_typing_updates_query() {
    let mut app = test_app();
    app.navigate_to(View::Search);
    app.input_mode = InputMode::Input;
    app.handle_key_event(key(KeyCode::Char('r')));
    app.handle_key_event(key(KeyCode::Char('u')));
    app.handle_key_event(key(KeyCode::Char('s')));
    assert_eq!(app.search_query, "rus");
}

#[test]
fn search_input_esc_clears_and_exits() {
    let mut app = test_app();
    app.navigate_to(View::Search);
    app.input_mode = InputMode::Input;
    app.search_query = "rust".to_owned();
    app.handle_key_event(key(KeyCode::Esc));
    assert!(app.search_query.is_empty());
    assert_eq!(app.input_mode, InputMode::Navigation);
}

#[test]
fn search_input_enter_exits_input_mode() {
    let mut app = test_app();
    app.navigate_to(View::Search);
    app.input_mode = InputMode::Input;
    app.search_query = "rust".to_owned();
    app.handle_key_event(key(KeyCode::Enter));
    assert_eq!(app.input_mode, InputMode::Navigation);
}

// ---------------------------------------------------------------------------
// Memory detail: scroll
// ---------------------------------------------------------------------------

#[test]
fn memory_detail_scroll_down_and_up() {
    let mut app = test_app();
    app.navigate_to(View::MemoryDetail("x".to_owned()));

    assert_eq!(app.scroll_offset, 0);
    app.handle_key_event(key(KeyCode::Down));
    assert_eq!(app.scroll_offset, 1);
    app.handle_key_event(key(KeyCode::Char('j')));
    assert_eq!(app.scroll_offset, 2);
    app.handle_key_event(key(KeyCode::Up));
    assert_eq!(app.scroll_offset, 1);
    app.handle_key_event(key(KeyCode::Char('k')));
    assert_eq!(app.scroll_offset, 0);
    // Cannot go negative.
    app.handle_key_event(key(KeyCode::Up));
    assert_eq!(app.scroll_offset, 0);
}

#[test]
fn memory_detail_esc_returns_to_previous_view() {
    let mut app = test_app();
    // Navigate Dashboard -> Search -> MemoryDetail
    app.navigate_to(View::Search);
    app.navigate_to(View::MemoryDetail("x".to_owned()));
    app.handle_key_event(key(KeyCode::Esc));
    // Should go back to Search (the previous view).
    assert_eq!(app.current_view, View::Search);
}

#[test]
fn memory_detail_backspace_returns_to_previous_view() {
    let mut app = test_app();
    app.navigate_to(View::MemoryDetail("x".to_owned()));
    app.handle_key_event(key(KeyCode::Backspace));
    // Previous view was Dashboard.
    assert_eq!(app.current_view, View::Dashboard);
}

// ---------------------------------------------------------------------------
// Create form: field navigation
// ---------------------------------------------------------------------------

#[test]
fn create_form_tab_cycles_forward_through_fields() {
    let mut app = test_app();
    app.navigate_to(View::CreateForm);
    assert_eq!(app.form_field, FormField::Title);

    app.handle_key_event(key(KeyCode::Tab));
    assert_eq!(app.form_field, FormField::Content);
    app.handle_key_event(key(KeyCode::Tab));
    assert_eq!(app.form_field, FormField::Summary);
    app.handle_key_event(key(KeyCode::Tab));
    assert_eq!(app.form_field, FormField::MemoryType);
    app.handle_key_event(key(KeyCode::Tab));
    assert_eq!(app.form_field, FormField::Source);
    app.handle_key_event(key(KeyCode::Tab));
    assert_eq!(app.form_field, FormField::Language);
    // Wraps back.
    app.handle_key_event(key(KeyCode::Tab));
    assert_eq!(app.form_field, FormField::Title);
}

#[test]
fn create_form_shift_backtab_cycles_backward() {
    let mut app = test_app();
    app.navigate_to(View::CreateForm);

    // BackTab with SHIFT goes backwards.
    app.handle_key_event(shift(KeyCode::BackTab));
    assert_eq!(app.form_field, FormField::Language);
}

#[test]
fn create_form_typing_updates_active_field() {
    let mut app = test_app();
    app.navigate_to(View::CreateForm);
    assert_eq!(app.form_field, FormField::Title);

    for c in ['H', 'e', 'l', 'l', 'o'] {
        app.handle_key_event(key(KeyCode::Char(c)));
    }
    assert_eq!(app.form_title, "Hello");
    assert!(app.form_content.is_empty());
}

#[test]
fn create_form_backspace_removes_char() {
    let mut app = test_app();
    app.navigate_to(View::CreateForm);
    app.form_title = "Helo".to_owned();
    app.handle_key_event(key(KeyCode::Backspace));
    assert_eq!(app.form_title, "Hel");
}

#[test]
fn create_form_esc_returns_to_dashboard() {
    let mut app = test_app();
    app.navigate_to(View::CreateForm);
    app.handle_key_event(key(KeyCode::Esc));
    assert_eq!(app.current_view, View::Dashboard);
}

#[test]
fn create_form_ctrl_s_sets_status_message() {
    let mut app = test_app();
    app.navigate_to(View::CreateForm);
    app.handle_key_event(ctrl(KeyCode::Char('s')));
    assert!(app.status_message.is_some());
    // Should NOT quit.
    assert!(!app.should_quit);
}

// ---------------------------------------------------------------------------
// filtered_memories
// ---------------------------------------------------------------------------

#[test]
fn filtered_memories_empty_query_returns_all() {
    let mut app = test_app();
    app.memories = vec![
        make_memory("a", "Alpha", 90.0),
        make_memory("b", "Beta", 70.0),
    ];
    assert_eq!(app.filtered_memories().len(), 2);
}

#[test]
fn filtered_memories_with_query_filters_by_title() {
    let mut app = test_app();
    app.memories = vec![
        make_memory("a", "Rust async", 90.0),
        make_memory("b", "Python typing", 70.0),
    ];
    app.search_query = "rust".to_owned();
    let result = app.filtered_memories();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "a");
}

#[test]
fn filtered_memories_case_insensitive() {
    let mut app = test_app();
    app.memories = vec![make_memory("a", "RUST TIPS", 90.0)];
    app.search_query = "rust".to_owned();
    assert_eq!(app.filtered_memories().len(), 1);
}

#[test]
fn filtered_memories_matches_content() {
    let mut app = test_app();
    app.memories = vec![make_memory("a", "Title", 90.0)];
    app.search_query = "content of".to_owned(); // from make_memory helper
    assert_eq!(app.filtered_memories().len(), 1);
}

// ---------------------------------------------------------------------------
// Navigate resets scroll offset
// ---------------------------------------------------------------------------

#[test]
fn navigate_to_resets_scroll_offset() {
    let mut app = test_app();
    app.navigate_to(View::MemoryDetail("x".to_owned()));
    app.scroll_offset = 10;
    app.navigate_to(View::Search);
    assert_eq!(app.scroll_offset, 0);
}

// ---------------------------------------------------------------------------
// Navigate sets previous_view
// ---------------------------------------------------------------------------

#[test]
fn navigate_to_sets_previous_view() {
    let mut app = test_app();
    assert!(app.previous_view.is_none());
    app.navigate_to(View::Search);
    assert_eq!(app.previous_view, Some(View::Dashboard));
    app.navigate_to(View::Keys);
    assert_eq!(app.previous_view, Some(View::Search));
}

// ---------------------------------------------------------------------------
// Terminal-too-small: only q/Ctrl+C allowed
// ---------------------------------------------------------------------------

#[test]
fn too_small_blocks_navigation() {
    let mut app = test_app();
    app.terminal_too_small = true;
    // Number keys should be ignored.
    app.handle_key_event(key(KeyCode::Char('2')));
    assert_eq!(app.current_view, View::Dashboard);
}

#[test]
fn too_small_q_still_quits() {
    let mut app = test_app();
    app.terminal_too_small = true;
    app.handle_key_event(key(KeyCode::Char('q')));
    assert!(app.should_quit);
}

// ---------------------------------------------------------------------------
// MIN_COLS / MIN_ROWS sanity
// ---------------------------------------------------------------------------

#[test]
fn min_size_constants_are_sensible() {
    assert!(MIN_COLS >= 60, "MIN_COLS should be at least 60 columns");
    assert!(MIN_ROWS >= 20, "MIN_ROWS should be at least 20 rows");
}
