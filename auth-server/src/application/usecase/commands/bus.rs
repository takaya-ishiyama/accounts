use std::{fmt, future::Future, pin::Pin};

use crate::application::usecase::commands::dto::{CommandDto, CommandResponse};

#[derive(Debug)]
pub enum CommandBusError {
    DomainError(String),
    Backend(String),
}

impl fmt::Display for CommandBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DomainError(msg) => write!(f, "domain error: {msg}"),
            Self::Backend(msg) => write!(f, "command bus backend error: {msg}"),
        }
    }
}

pub type CommandBusFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommandBusError>> + 'a>>;

pub trait CommandBus {
    fn dispatch<'a>(
        &'a self,
        account_id: &'a str,
        cmd: &'a CommandDto,
    ) -> CommandBusFuture<'a, CommandResponse>;
}
