#![deny(clippy::all)]
#![forbid(clippy::indexing_slicing)]

doc_comment::doctest!("../readme.md");

pub use error::*;
use markdown::mdast::Node;
use markdown::to_mdast;
use markdown::ParseOptions;

pub fn get_node_from_content(content: impl AsRef<str>) -> Result<Node> {
  let options = ParseOptions::gfm();
  let mdast = to_mdast(content.as_ref(), &options).map_err(Error::MarkdownError)?;

  Ok(mdast)
}

mod error;

#[cfg(test)]
mod __tests;
