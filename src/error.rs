use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Database error")]
    Diesel(#[from] diesel::result::Error),
}
