use derive_more::Deref;
use derive_more::DerefMut;
use markdown::mdast::Html;
use markdown::mdast::Node;
use markdown::to_mdast;
use markdown::ParseOptions;

use super::MdtError;
use super::MdtResult;
use crate::Position;

pub fn parse(content: impl AsRef<str>) -> MdtResult<Vec<Block>> {
  let content = content.as_ref();
  let html_nodes = get_html_nodes(content)?;
  let blocks = vec![];
  let _block_creators = Vec::<BlockCreator>::new();

  for node in html_nodes {
    let Some(ref position) = node.position else {
      continue;
    };

    println!("{:?}", position);
    println!("VALUE: {:?}", node.value);

    for _ch in content.chars() {}
  }

  Ok(blocks)
}

pub fn get_html_nodes(content: impl AsRef<str>) -> MdtResult<Vec<Html>> {
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

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Blocks(Vec<Block>);

impl From<Vec<Block>> for Blocks {
  fn from(blocks: Vec<Block>) -> Self {
    Self(blocks)
  }
}

struct BlockCreator {
  name: String,
  r#type: BlockType,
  opening: Position,
  closing: Option<Position>,
  transformers: Vec<Transformer>,
}

impl BlockCreator {
  pub fn new(name: String, r#type: BlockType, opening: Position) -> Self {
    Self {
      name,
      r#type,
      opening,
      closing: None,
      transformers: vec![],
    }
  }

  pub fn into_block(self) -> MdtResult<Block> {
    let Some(closing) = self.closing else {
      return Err(MdtError::MissingClosingTag(self.name));
    };

    let block = Block {
      name: self.name,
      r#type: self.r#type,
      opening: self.opening,
      closing,
      transformers: self.transformers,
    };

    Ok(block)
  }
}

#[derive(Debug, Clone)]
pub struct Block {
  /// The name of the block. This is used to
  pub name: String,
  pub r#type: BlockType,
  pub opening: Position,
  pub closing: Position,
  pub transformers: Vec<Transformer>,
}

#[derive(Debug, Clone)]
pub struct Transformer {
  pub r#type: TransformerType,
  pub args: Vec<Argument>,
}

#[derive(Debug, Clone)]
pub enum Argument {
  String(String),
  Number(f64),
  Boolean(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformerType {
  /// Trim all whitespace from the start and end of the content.
  Trim,
  /// Trim all whitespace from the start of the content.
  TrimStart,
  /// Trim all whitespace from the end of the content.
  TrimEnd,
  /// Wrap the content in the given string.
  Wrap,
  /// Indent each line with the given string.
  Indent,
  /// Wrap the content in a codeblock with the provided language string.
  CodeBlock,
  /// Wrap the content with inline code `\`content\``.
  Code,
  /// Replace all instances of the given string with the replacement string.
  Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
  /// These are the blocks that are used to provide a value to any consumers.
  /// Their names can be referenced by consumers to hoist content. They should
  /// only exist within the confines of a `*.t.md` file.
  ///
  /// ```md
  /// <!-- {@exampleProvider} -->
  /// <!-- {/exampleProvider} -->
  /// ```
  Provider,
  /// Consumers are blocks that have their content hoisted from a provider with
  /// the same name. They will be updated to the latest content whenever the
  /// `mdt` command is run.
  ///
  /// ```md
  /// <!-- {=exampleConsumer} -->
  /// <!-- {/exampleConsumer} -->
  /// ```
  Consumer,
}
