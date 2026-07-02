//! Active-window evidence store (TASK-046). Storage-only foundation: three SQLite tables,
//! typed write/read/prune APIs, title-redaction gate, and retention lifecycle.
//! No capture, no IPC, no renderer change. App behavior stays identical; tables start empty.

pub mod capture;
pub mod config;
pub mod model;
pub mod settings_api;
pub mod store;

#[cfg(test)]
mod tests;
