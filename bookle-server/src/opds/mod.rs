//! OPDS Catalog Support
//!
//! This module provides OPDS 1.2 (Atom/XML) and OPDS 2.0 (JSON) catalog support
//! for e-readers and apps to browse and download books from Bookle.

pub mod opensearch;
pub mod types;
pub mod xml;

pub use types::*;
