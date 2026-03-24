//! State-transition tests for the TUI `App`.
//!
//! These tests exercise key-event handling and state mutations without
//! rendering anything to a terminal, making them safe to run in CI.

#![cfg(test)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fixonce_core::memory::types::{
    EmbeddingStatus, Memory, MemoryType, PipelineStatus, SearchHit, SearchMemoryResponse,
    SourceType,
};
use tokio::sync::mpsc;

use super::app::{
    App, FormField, FormMode, HeatmapMode, InputMode, ListMode, SearchType, View, MIN_COLS,
    MIN_ROWS,
};
use super::data::DataState;

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

// ---------------------------------------------------------------------------
// Input mode: 'q' does not quit
// ---------------------------------------------------------------------------

#[test]
fn q_does_not_quit_in_input_mode() {
    let mut app = test_app();
    app.navigate_to(View::Search);
    app.input_mode = InputMode::Input;
    app.handle_key_event(key(KeyCode::Char('q')));
    // In input mode, 'q' is just a character appended to the search query.
    assert!(!app.should_quit);
    assert_eq!(app.search_query, "q");
}

// ---------------------------------------------------------------------------
// Input mode: Ctrl+C always quits
// ---------------------------------------------------------------------------

#[test]
fn ctrl_c_quits_in_input_mode() {
    let mut app = test_app();
    app.navigate_to(View::Search);
    app.input_mode = InputMode::Input;
    app.handle_key_event(ctrl(KeyCode::Char('c')));
    assert!(app.should_quit);
}

// ---------------------------------------------------------------------------
// Input mode: number keys do not navigate views
// ---------------------------------------------------------------------------

#[test]
fn number_keys_dont_navigate_in_input_mode() {
    let mut app = test_app();
    app.navigate_to(View::Search);
    app.input_mode = InputMode::Input;
    app.handle_key_event(key(KeyCode::Char('1')));
    app.handle_key_event(key(KeyCode::Char('2')));
    app.handle_key_event(key(KeyCode::Char('3')));
    // Still on Search — numbers were appended to the query, not treated as nav.
    assert_eq!(app.current_view, View::Search);
    assert_eq!(app.search_query, "123");
}

// ---------------------------------------------------------------------------
// Dashboard: bracket keys cycle heatmap mode
// ---------------------------------------------------------------------------

#[test]
fn bracket_keys_cycle_heatmap_mode() {
    let mut app = test_app();
    // Default is Created.
    assert_eq!(app.heatmap_mode, HeatmapMode::Created);

    // ']' advances forward: Created -> Read -> Searched -> Created
    app.handle_key_event(key(KeyCode::Char(']')));
    assert_eq!(app.heatmap_mode, HeatmapMode::Read);
    app.handle_key_event(key(KeyCode::Char(']')));
    assert_eq!(app.heatmap_mode, HeatmapMode::Searched);
    app.handle_key_event(key(KeyCode::Char(']')));
    assert_eq!(app.heatmap_mode, HeatmapMode::Created);

    // '[' moves backward.
    app.handle_key_event(key(KeyCode::Char('[')));
    assert_eq!(app.heatmap_mode, HeatmapMode::Searched);
}

// ---------------------------------------------------------------------------
// Dashboard: apostrophe cycles list mode forward
// ---------------------------------------------------------------------------

#[test]
fn semicolon_cycles_list_mode() {
    let mut app = test_app();
    // Default is RecentlyCreated.
    assert_eq!(app.list_mode, ListMode::RecentlyCreated);

    // '\'' advances: RecentlyCreated -> RecentlyViewed -> MostAccessed -> RecentlyCreated
    app.handle_key_event(key(KeyCode::Char('\'')));
    assert_eq!(app.list_mode, ListMode::RecentlyViewed);
    app.handle_key_event(key(KeyCode::Char('\'')));
    assert_eq!(app.list_mode, ListMode::MostAccessed);
    app.handle_key_event(key(KeyCode::Char('\'')));
    assert_eq!(app.list_mode, ListMode::RecentlyCreated);
}

// ---------------------------------------------------------------------------
// Dashboard: changing list mode resets selected_index
// ---------------------------------------------------------------------------

#[test]
fn semicolon_resets_selected_index() {
    let mut app = test_app();
    app.memories = vec![
        make_memory("a", "Alpha", 90.0),
        make_memory("b", "Beta", 70.0),
        make_memory("c", "Gamma", 50.0),
    ];
    app.selected_index = 2;

    // Apostrophe cycles list mode and resets selection.
    app.handle_key_event(key(KeyCode::Char('\'')));
    assert_eq!(app.selected_index, 0);

    // Semicolon (reverse cycle) also resets selection.
    app.selected_index = 1;
    app.handle_key_event(key(KeyCode::Char(';')));
    assert_eq!(app.selected_index, 0);
}

// ---------------------------------------------------------------------------
// Dashboard: Enter with empty list does not navigate
// ---------------------------------------------------------------------------

#[test]
fn dashboard_enter_with_empty_list() {
    let mut app = test_app();
    // No memories loaded.
    assert!(app.memories.is_empty());

    app.handle_key_event(key(KeyCode::Enter));
    // Should still be on Dashboard — nothing to open.
    assert_eq!(app.current_view, View::Dashboard);
}

// ---------------------------------------------------------------------------
// Search: Tab in input mode cycles SearchType
// ---------------------------------------------------------------------------

#[test]
fn search_tab_cycles_search_type() {
    let mut app = test_app();
    app.navigate_to(View::Search);
    app.input_mode = InputMode::Input;
    // Default is Fts.
    assert_eq!(app.search_type, SearchType::Fts);

    // Tab cycles: Fts -> Vector -> Hybrid -> Fts
    app.handle_key_event(key(KeyCode::Tab));
    assert_eq!(app.search_type, SearchType::Vector);
    app.handle_key_event(key(KeyCode::Tab));
    assert_eq!(app.search_type, SearchType::Hybrid);
    app.handle_key_event(key(KeyCode::Tab));
    assert_eq!(app.search_type, SearchType::Fts);
}

// ---------------------------------------------------------------------------
// Search: Enter in input mode switches to Navigation
// ---------------------------------------------------------------------------

#[test]
fn search_enter_switches_to_navigation() {
    let mut app = test_app();
    app.navigate_to(View::Search);
    app.input_mode = InputMode::Input;
    app.search_query = "rust".to_owned();
    app.handle_key_event(key(KeyCode::Enter));
    assert_eq!(app.input_mode, InputMode::Navigation);
}

// ---------------------------------------------------------------------------
// Navigation history: detail opened from search returns to search on Esc
// ---------------------------------------------------------------------------

#[test]
fn memory_detail_from_search_returns_to_search() {
    let memory = make_memory("mem-42", "Rust tricks", 80.0);
    let mut app = test_app();

    // Navigate to Search and load a fake result.
    app.navigate_to(View::Search);
    app.search_results = DataState::Loaded(SearchMemoryResponse {
        hits: vec![SearchHit {
            memory: memory.clone(),
            similarity: 0.9,
        }],
        total: 1,
    });

    // Open detail from search results.
    app.navigate_to(View::MemoryDetail("mem-42".to_owned()));
    assert_eq!(app.previous_view, Some(View::Search));

    // Esc should return to Search, not Dashboard.
    app.handle_key_event(key(KeyCode::Esc));
    assert_eq!(app.current_view, View::Search);
}

// ---------------------------------------------------------------------------
// Navigation history: edit from detail → Esc form → Dashboard, Esc detail → previous
// ---------------------------------------------------------------------------

#[test]
fn edit_from_detail_preserves_previous() {
    let memory = make_memory("mem-99", "Important tip", 75.0);
    let mut app = test_app();

    // Start from Dashboard, go to detail.
    app.memories = vec![memory.clone()];
    app.navigate_to(View::MemoryDetail("mem-99".to_owned()));
    // previous_view is now Dashboard.
    assert_eq!(app.previous_view, Some(View::Dashboard));

    // Press 'e' to open the edit form.
    // handle_memory_detail_key 'e' calls navigate_to(CreateForm), so
    // previous_view becomes MemoryDetail.
    app.handle_key_event(key(KeyCode::Char('e')));
    assert_eq!(app.current_view, View::CreateForm);
    assert_eq!(
        app.previous_view,
        Some(View::MemoryDetail("mem-99".to_owned()))
    );

    // Esc from the form (navigation mode) goes back to Dashboard.
    app.handle_key_event(key(KeyCode::Esc));
    assert_eq!(app.current_view, View::Dashboard);
}

// ---------------------------------------------------------------------------
// Form: pressing 'e' on a memory detail populates form fields
// ---------------------------------------------------------------------------

#[test]
fn edit_mode_populates_form_fields() {
    let memory = make_memory("mem-7", "My memory", 60.0);
    let mut app = test_app();
    app.memories = vec![memory.clone()];
    app.navigate_to(View::MemoryDetail("mem-7".to_owned()));

    // Press 'e' to open edit form.
    app.handle_key_event(key(KeyCode::Char('e')));

    // Form fields should be populated from the memory.
    assert_eq!(app.form_title, memory.title);
    assert_eq!(app.form_content, memory.content);
    assert_eq!(app.form_summary, memory.summary);
    assert_eq!(app.form_memory_type, memory.memory_type.to_string());
    assert_eq!(app.form_source, memory.source_type.to_string());
    // form_mode should be Edit with the correct memory_id.
    assert_eq!(
        app.form_mode,
        FormMode::Edit {
            memory_id: "mem-7".to_owned()
        }
    );
    // Should be in Input mode ready to type.
    assert_eq!(app.input_mode, InputMode::Input);
}

// ---------------------------------------------------------------------------
// Form: pressing '3' to open create form sets form_mode to Create
// ---------------------------------------------------------------------------

#[test]
fn form_mode_is_create_from_tab3() {
    let mut app = test_app();
    app.handle_key_event(key(KeyCode::Char('3')));
    assert_eq!(app.current_view, View::CreateForm);
    assert_eq!(app.form_mode, FormMode::Create);
}
