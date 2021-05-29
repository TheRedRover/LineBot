use std::{result, str::Utf8Error};
use thiserror::Error;

use crate::da;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Database error")]
    Diesel(#[from] da::Error),
    #[error("Teloxide request error")]
    TeloxideRequest(#[from] teloxide::RequestError),
    #[error("Teloxide download error")]
    TeloxideDonload(#[from] teloxide::DownloadError),
    #[error("Invalid UTF")]
    UTF(#[from] Utf8Error),
    #[error("No queue reply.")]
    NoQueueReply,
}

pub type Result<T> = result::Result<T, Error>;
