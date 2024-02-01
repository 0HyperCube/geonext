#[macro_use]
extern crate log;

mod client_message;
pub mod map_loader;
mod server_message;
pub mod territories;

pub use client_message::ClientMessage;
pub use server_message::ServerMessage;
