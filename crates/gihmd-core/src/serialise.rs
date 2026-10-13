use std::fmt::Display;

use serde::Serializer;

/// Serialises a `#[source]` error field by its `Display` text so that domain errors can cross the
/// IPC boundary unchanged while keeping the real cause in the Rust error chain.
pub fn display<T: Display, S: Serializer>(value: &T, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}
