//! Core types for the Bookle Intermediate Representation (IR)

mod block;
mod book;
mod chapter;
mod metadata;
mod resource;
mod toc;

pub use block::{Block, Inline, TableCell, TableData};
pub use book::Book;
pub use chapter::Chapter;
pub use metadata::{Metadata, ReadingDirection, SeriesInfo};
pub use resource::{Resource, ResourceData, ResourceStore};
pub use toc::TocEntry;
