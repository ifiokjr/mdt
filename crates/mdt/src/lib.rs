#![deny(clippy::all)]
#![forbid(clippy::indexing_slicing)]

doc_comment::doctest!("../readme.md");

pub use error::*;
pub use lexer::*;
use markdown::mdast::Html;
use markdown::mdast::Node;
use markdown::to_mdast;
use markdown::ParseOptions;
pub use position::*;
pub use tokens::*;

pub fn get_html_nodes(content: impl AsRef<str>) -> Result<Vec<Html>> {
  let options = ParseOptions::gfm();
  let mdast = to_mdast(content.as_ref(), &options).map_err(MdtError::Markdown)?;
  let mut html_nodes = vec![];
  collect_html(&mdast, &mut html_nodes);

  Ok(html_nodes)
}

fn collect_html(node: &Node, nodes: &mut Vec<Html>) {
  match node {
    Node::Html(html) => nodes.push(html.clone()),
    _ => {
      if let Some(node) = node.children() {
        for child in node {
          collect_html(child, nodes);
        }
      }
    }
  }
}

mod error;
mod lexer;
pub mod patterns;
mod position;
mod tokens;

#[cfg(test)]
mod __tests;
