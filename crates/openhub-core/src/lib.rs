//! Sans-IO core for the Red Enlace (ATC) OpenHub payment APIs.
//!
//! This crate never performs network I/O. Each endpoint is an [`Operation`]
//! that builds an [`HttpRequest`] and parses an [`HttpResponse`]; the host
//! language executes the HTTP call with its own client. Bindings reach the
//! operations through a [`Registry`] and the JSON boundary in [`ffi`].

pub mod amount;
mod config;
pub mod envelope;
mod error;
pub mod ffi;
mod http;
mod operation;
pub mod status;
mod token;
pub mod validate;
pub mod webhook;

pub use config::{Config, Environment};
pub use error::{ApiFieldError, Error, Result};
pub use http::{HttpRequest, HttpResponse, Method};
pub use operation::{Chain, Ctx, Handler, OpInfo, Operation, Registry};
pub use status::PaymentStatus;
pub use token::{AccessToken, EXPIRY_SKEW_SECS, Token, TokenInput};
pub use webhook::{WebhookCall, WebhookTarget};

// Operations shared by every product package. Product crates declare their
// own registry and bindings combine them with [`Chain`].
registry!(pub CoreOps {
    operations: [Token],
    handlers: [],
});
