#[derive(Clone)]
pub struct StoreError {
    pub message: String,
}

impl StoreError {
    pub fn new(message: &str) -> StoreError {
        StoreError {
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Store error: {}", self.message)
    }
}

impl std::fmt::Debug for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}:{}] Store error: {}", file!(), line!(), self.message)
    }
}
