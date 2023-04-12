#![deny(clippy::all)]
#![forbid(clippy::indexing_slicing)]

//! <!-- {=mdtPackageDocumentation|prefix:"\n"|indent:"//! "} -->
//! <!-- {/mdtPackageDocumentation} -->

pub use error::*;
pub use lexer::*;
pub use parser::*;
pub use patterns::PatternMatcher;
pub use position::*;
pub use tokens::*;

mod error;
mod lexer;
mod parser;
pub mod patterns;
mod position;
mod tokens;

#[cfg(test)]
mod __tests;

#[cfg(test)]
mod __fixtures;
