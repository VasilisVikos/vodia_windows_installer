#[derive(Debug, Clone)]
pub struct InitialCredentials {
    pub username: String,
    pub password: String,
}

pub fn generate_initial_credentials() -> InitialCredentials {
    InitialCredentials {
        username: "admin".to_string(),
        password: "blank".to_string(),
    }
}