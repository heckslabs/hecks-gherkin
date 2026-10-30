//! Projects a plain-sentence domain into Ruby-form bluebook text.
//!
//! The output is the text hecks loads as a `.bluebook`. This crate never builds IR: the
//! contract (`docs/sentence-contract.md` in the hecks repository) is sentences in, bluebook text
//! out.

mod project;

pub use project::{project, Diagnostic};
