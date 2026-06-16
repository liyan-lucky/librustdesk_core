#[derive(Debug)]
pub enum CallbackResult {
    Next,
    Stop,
    StopWithError(String),
}
