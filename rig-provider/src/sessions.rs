use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;

/// Manages persistent temporary directories for agent sessions.
#[derive(Clone, Default)]
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, Arc<TempDir>>>>,
}

impl SessionManager {
    /// Returns a new, empty `SessionManager`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets or creates a temporary directory for the given session ID.
    ///
    /// # Errors
    /// Returns an error if the temporary directory cannot be created.
    pub async fn get_session_dir(&self, session_id: &str) -> anyhow::Result<PathBuf> {
        let mut sessions = self.sessions.lock().await;

        if let Some(dir) = sessions.get(session_id) {
            Ok(dir.path().to_path_buf())
        } else {
            let dir = Arc::new(tempfile::tempdir()?);
            let path = dir.path().to_path_buf();
            sessions.insert(session_id.to_string(), dir);
            drop(sessions);
            Ok(path)
        }
    }
}
