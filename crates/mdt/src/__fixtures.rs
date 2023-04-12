use super::*;

pub fn consumer_token_group() -> TokenGroup {
  TokenGroup {
    tokens: vec![
      Token::HtmlCommentOpen,
      Token::Whitespace(b' '),
      Token::ConsumerTag,
      Token::Ident("exampleName".to_string()),
      Token::BraceClose,
      Token::Whitespace(b' '),
      Token::HtmlCommentClose,
    ],
    position: Position {
      start: Point {
        line: 1,
        column: 1,
        offset: 0,
      },
      end: Point {
        line: 1,
        column: 24,
        offset: 23,
      },
    },
  }
}

pub fn consumer_token_group_with_arguments() -> TokenGroup {
  TokenGroup {
    tokens: vec![
      Token::HtmlCommentOpen,
      Token::Whitespace(b' '),
      Token::ConsumerTag,
      Token::Ident("exampleName".to_string()),
      Token::Pipe,
      Token::Ident("trim".to_string()),
      Token::Pipe,
      Token::Ident("indent".to_string()),
      Token::ArgumentDelimiter,
      Token::String("/// ".to_string(), b'"'),
      Token::BraceClose,
      Token::Whitespace(b' '),
      Token::HtmlCommentClose,
    ],
    position: Position {
      start: Point {
        line: 1,
        column: 1,
        offset: 0,
      },
      end: Point {
        line: 1,
        column: 45,
        offset: 44,
      },
    },
  }
}

pub fn provider_token_group() -> TokenGroup {
  TokenGroup {
    tokens: vec![
      Token::HtmlCommentOpen,
      Token::Whitespace(b' '),
      Token::ProviderTag,
      Token::Ident("exampleProvider".to_string()),
      Token::BraceClose,
      Token::Whitespace(b' '),
      Token::HtmlCommentClose,
    ],
    position: Position {
      start: Point {
        line: 1,
        column: 1,
        offset: 0,
      },
      end: Point {
        line: 1,
        column: 28,
        offset: 27,
      },
    },
  }
}

pub fn closing_token_group() -> TokenGroup {
  TokenGroup {
    tokens: vec![
      Token::HtmlCommentOpen,
      Token::Whitespace(b' '),
      Token::CloseTag,
      Token::Ident("example".to_string()),
      Token::BraceClose,
      Token::Whitespace(b' '),
      Token::HtmlCommentClose,
    ],
    position: Position {
      start: Point {
        line: 1,
        column: 1,
        offset: 0,
      },
      end: Point {
        line: 1,
        column: 20,
        offset: 19,
      },
    },
  }
}

pub fn closing_token_group_no_whitespace() -> TokenGroup {
  TokenGroup {
    tokens: vec![
      Token::HtmlCommentOpen,
      Token::Newline,
      Token::CloseTag,
      Token::Ident("example".to_string()),
      Token::BraceClose,
      Token::HtmlCommentClose,
    ],
    position: Position {
      start: Point {
        line: 1,
        column: 2,
        offset: 1,
      },
      end: Point {
        line: 2,
        column: 13,
        offset: 19,
      },
    },
  }
}
