use std::{result, str::Utf8Error};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Database error")]
    Diesel(#[from] diesel::result::Error),
    #[error("Teloxide request error")]
    TeloxideRequest(#[from] teloxide::RequestError),
    #[error("Teloxide download error")]
    TeloxideDonload(#[from] teloxide::DownloadError),
    #[error("Invalid UTF")]
    UTF(#[from] Utf8Error),
}

pub type Result<T> = result::Result<T, Error>;
