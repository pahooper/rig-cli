use rig::completion::message::{AssistantContent, Message, UserContent};
use rig::completion::CompletionRequest;

/// Formats the chat history into a single string prompting the user for the next action.
pub fn format_chat_history(request: &CompletionRequest) -> String {
    let mut prompt_str = String::new();

    if let Some(preamble) = &request.preamble {
        prompt_str.push_str(&format!("System: {}\n\n", preamble));
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
                prompt_str.push_str(&format!("User: {}\n", text));
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
                prompt_str.push_str(&format!("Assistant: {}\n", text));
            }
        }
    }

    prompt_str
}
