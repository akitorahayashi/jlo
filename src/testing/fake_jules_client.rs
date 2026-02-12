use crate::domain::AppError;
use crate::domain::layers::strategy::JulesClientFactory;
use crate::ports::{JulesClient, SessionRequest, SessionResponse};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct FakeJulesClient {
    pub created_sessions: Arc<Mutex<Vec<SessionRequest>>>,
    pub response_session_id: String,
}

impl FakeJulesClient {
    pub fn new(response_session_id: impl Into<String>) -> Self {
        Self {
            created_sessions: Arc::new(Mutex::new(vec![])),
            response_session_id: response_session_id.into(),
        }
    }

    pub fn get_created_sessions(&self) -> Vec<SessionRequest> {
        self.created_sessions.lock().unwrap().clone()
    }
}

impl JulesClient for FakeJulesClient {
    fn create_session(&self, request: SessionRequest) -> Result<SessionResponse, AppError> {
        self.created_sessions.lock().unwrap().push(request);
        Ok(SessionResponse {
            session_id: self.response_session_id.clone(),
            status: "created".to_string(),
        })
    }
}

pub struct FakeJulesClientFactory {
    pub client: FakeJulesClient,
}

impl FakeJulesClientFactory {
    pub fn new(client: FakeJulesClient) -> Self {
        Self { client }
    }
}

impl JulesClientFactory for FakeJulesClientFactory {
    fn create(&self) -> Result<Box<dyn JulesClient>, AppError> {
        Ok(Box::new(self.client.clone()))
    }
}
