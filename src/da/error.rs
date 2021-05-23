use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error")]
    Diesel(#[from] diesel::result::Error),
    #[error("Nonexistent position for swap.")]
    NonexistentPosition { pos: i32 },
}
