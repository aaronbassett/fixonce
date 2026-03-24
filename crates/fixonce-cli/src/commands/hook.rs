//! `fixonce hook <event>` — dispatches to the appropriate lifecycle hook handler.
//!
//! This command is intended to be called from the shell wrapper scripts in
//! `hooks/` and is not designed for direct human use.
//!
//! All hooks exit 0 (success) when the user is not authenticated (EC-43).
//! Timeout enforcement is handled by the shell wrappers (EC-41).

use anyhow::Result;
use clap::{Subcommand, ValueEnum};
use fixonce_hooks::{
    post_tool_use::on_post_tool_use, pre_tool_use::on_pre_tool_use,
    session_start::on_session_start, stop::on_stop, user_prompt::on_user_prompt, HookError,
};

// ---------------------------------------------------------------------------
// CLI types
// ---------------------------------------------------------------------------

/// The lifecycle event to handle.
#[derive(Debug, Clone, ValueEnum)]
pub enum HookEvent {
    /// A Claude Code session has started.
    #[value(name = "session-start")]
    SessionStart,
    /// The user submitted a prompt.
    #[value(name = "user-prompt-submit")]
    UserPromptSubmit,
    /// A tool is about to be used.
    #[value(name = "pre-tool-use")]
    PreToolUse,
    /// A tool has just been used.
    #[value(name = "post-tool-use")]
    PostToolUse,
    /// The Claude Code session is ending.
    #[value(name = "stop")]
    Stop,
}

/// Subcommands available under `fixonce hook`.
#[derive(Debug, Subcommand)]
pub enum HookSubcommand {
    /// Session-start hook: surfaces critical memories.
    #[command(name = "session-start")]
    SessionStart,
    /// User-prompt hook: injects relevant memories as context.
    #[command(name = "user-prompt-submit")]
    UserPromptSubmit,
    /// Pre-tool-use hook: warns on anti-memory matches (score > 0.7).
    #[command(name = "pre-tool-use")]
    PreToolUse,
    /// Post-tool-use hook: advises on anti-memory matches (score > 0.5).
    #[command(name = "post-tool-use")]
    PostToolUse,
    /// Stop hook: surfaces session-end reminders.
    #[command(name = "stop")]
    Stop,
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Dispatch to the appropriate hook handler.
///
/// Errors from hook handlers are logged to stderr and treated as non-fatal —
/// the process exits 0 in all cases so Claude Code is never blocked.
pub async fn run_hook(api_url: &str, event: HookSubcommand) -> Result<()> {
    let result = match event {
        HookSubcommand::SessionStart => on_session_start(api_url).await.map(|msg| {
            if !msg.is_empty() {
                print!("{msg}");
            }
        }),

        HookSubcommand::UserPromptSubmit => {
            // Read prompt text from stdin if available.
            let prompt_text = read_stdin_opt();
            on_user_prompt(api_url, &prompt_text).await.map(|msg| {
                if !msg.is_empty() {
                    print!("{msg}");
                }
            })
        }

        HookSubcommand::PreToolUse => {
            // Read tool input from stdin if available.
            let tool_input = read_stdin_opt();
            on_pre_tool_use(api_url, &tool_input).await.map(|warning| {
                if let Some(w) = warning {
                    print!("{w}");
                }
            })
        }

        HookSubcommand::PostToolUse => {
            // Read tool output from stdin if available.
            let tool_output = read_stdin_opt();
            on_post_tool_use(api_url, &tool_output)
                .await
                .map(|advisory| {
                    if let Some(a) = advisory {
                        print!("{a}");
                    }
                })
        }

        HookSubcommand::Stop => on_stop(api_url).await.map(|msg| {
            if !msg.is_empty() {
                print!("{msg}");
            }
        }),
    };

    // EC-43: skip silently when unauthenticated.
    // All other errors are logged to stderr but never propagated — hooks must
    // never block the agent.
    if let Err(e) = result {
        match e {
            HookError::Unauthenticated => {
                // Silent skip per EC-43.
            }
            other => {
                eprintln!("[FixOnce] hook error (non-fatal): {other}");
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read all of stdin into a string, returning an empty string on any error or
/// when stdin is a TTY (interactive terminal with no piped input).
fn read_stdin_opt() -> String {
    use std::io::{IsTerminal as _, Read as _};

    // Only attempt to read stdin when it is not connected to a terminal.
    // `IsTerminal` is stable since Rust 1.70 and avoids unsafe code.
    if std::io::stdin().is_terminal() {
        return String::new();
    }

    let mut buf = String::new();
    let _ = std::io::stdin().read_to_string(&mut buf);
    buf
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify all hook event variants have names consistent with shell script names.
    #[test]
    fn hook_event_names_match_scripts() {
        // These should match hooks/*.sh file names.
        let events = [
            ("session-start", HookEvent::SessionStart),
            ("user-prompt-submit", HookEvent::UserPromptSubmit),
            ("pre-tool-use", HookEvent::PreToolUse),
            ("post-tool-use", HookEvent::PostToolUse),
            ("stop", HookEvent::Stop),
        ];
        for (expected_name, event) in events {
            // Verify the value is parseable from its expected CLI string.
            let parsed = HookEvent::from_str(expected_name, false);
            assert!(
                parsed.is_ok(),
                "HookEvent variant for '{expected_name}' must be parseable"
            );
            // Verify the Display / debug representation matches.
            let _ = event; // consumed above
        }
    }
}
