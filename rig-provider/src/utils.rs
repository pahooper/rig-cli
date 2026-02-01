use std::fmt::Write;

use rig::completion::message::{AssistantContent, Message, UserContent};
use rig::completion::CompletionRequest;

/// Formats the chat history into a single string prompting the user for the next action.
#[must_use]
pub fn format_chat_history(request: &CompletionRequest) -> String {
    let mut prompt_str = String::new();

    if let Some(preamble) = &request.preamble {
        let _ = write!(prompt_str, "System: {preamble}\n\n");
    }

    for msg in request.chat_history.iter() {
        match msg {
            Message::User { content } => {
                let text = content
                    .iter()
                    .map(|c| match c {
                        UserContent::Text(t) => t.text.clone(),
                        _ => String::new(),
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let _ = writeln!(prompt_str, "User: {text}");
            }
            Message::Assistant { content, .. } => {
                let text = content
                    .iter()
                    .map(|c| match c {
                        AssistantContent::Text(t) => t.text.clone(),
                        _ => String::new(),
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let _ = writeln!(prompt_str, "Assistant: {text}");
            }
        }
    }

    prompt_str
}
