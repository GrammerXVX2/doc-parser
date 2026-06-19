pub mod charts;
pub mod comments;
pub mod docx;
pub mod formulas;
pub mod media;
pub mod notes;
pub mod numbering;
pub mod ooxml;
pub mod pptx;
pub mod relationships;
pub mod shapes;
pub mod slides;
pub mod styles;
pub mod xlsx;

pub use ooxml::OoxmlPackage;
pub use relationships::{Relationship, parse_relationships};
