use crate::error::AppError;
use tower_sessions::Session;

#[derive(Clone, Debug)]
pub struct PageContext {
    pub user_pid: Option<String>,
}

impl PageContext {
    pub async fn from_session(session: &Session) -> Result<Self, AppError> {
        let user_pid = session
            .get::<String>("user_pid")
            .await
            .map_err(anyhow::Error::from)?;

        Ok(Self { user_pid })
    }

    pub fn is_authenticated(&self) -> bool {
        self.user_pid.is_some()
    }
}
