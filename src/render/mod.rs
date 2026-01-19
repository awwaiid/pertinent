// Render module - abstraction layer for multiple rendering backends

pub mod types;
pub mod resolved;
pub mod resolver;
pub mod bevy;
pub mod pdf;

// Re-export commonly used types
pub use resolver::{resolve_deck, ResolveConfig};
