use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error.")]
    Diesel(#[from] diesel::result::Error),
    #[error("Nonexistent position for swap.")]
    NonexistentPosition { pos: i32 },
    #[error("Something unexpected.")]
    Wtf(String),
}

pub type Result<T> = std::result::Result<T, Error>;
