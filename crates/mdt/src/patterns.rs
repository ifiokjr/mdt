use crate::MdtError;
use crate::MdtResult;
use crate::Token;
use crate::TokenGroup;

pub type PatternMatcher = Box<dyn Fn(&TokenGroup, usize) -> MdtResult<usize> + 'static>;

pub fn closing_pattern() -> Vec<PatternMatcher> {
  vec![
    one(vec![Token::HtmlCommentOpen]),
    optional_many(vec![Token::whitespace(), Token::Newline]),
    one(vec![Token::CloseTag]),
    optional_many(vec![Token::whitespace()]),
    one(vec![Token::any()]),
    optional_many(vec![Token::whitespace()]),
    one(vec![Token::BraceClose]),
    optional_many(vec![Token::whitespace(), Token::Newline]),
    one(vec![Token::HtmlCommentClose]),
  ]
}

pub fn consumer_pattern() -> Vec<PatternMatcher> {
  vec![
    one(vec![Token::HtmlCommentOpen]),
    optional_many(vec![Token::whitespace(), Token::Newline]),
    one(vec![Token::ConsumerTag]),
    optional_many(vec![Token::whitespace()]),
    one(vec![Token::any()]),
    optional_many(vec![Token::whitespace()]),
    optional_many_group(vec![
      one(vec![Token::Pipe]),
      optional_many(vec![Token::whitespace()]),
      one(vec![Token::any()]),
      optional_many(vec![Token::whitespace()]),
      optional_many_group(vec![
        one(vec![Token::ArgumentDelimiter]),
        optional_many(vec![Token::whitespace()]),
        one(vec![
          Token::string(),
          Token::r#true(),
          Token::r#false(),
          Token::int(),
          Token::float(),
        ]),
        optional_many(vec![Token::whitespace()]),
      ]),
    ]),
    one(vec![Token::BraceClose]),
    optional_many(vec![Token::whitespace(), Token::Newline]),
    one(vec![Token::HtmlCommentClose]),
  ]
}

pub fn provider_pattern() -> Vec<PatternMatcher> {
  vec![
    one(vec![Token::HtmlCommentOpen]),
    optional_many(vec![Token::whitespace(), Token::Newline]),
    one(vec![Token::ProviderTag]),
    optional_many(vec![Token::whitespace()]),
    one(vec![Token::any()]),
    optional_many(vec![Token::whitespace()]),
    optional_many_group(vec![
      one(vec![Token::Pipe]),
      optional_many(vec![Token::whitespace()]),
      one(vec![Token::any()]),
      optional_many(vec![Token::whitespace()]),
      optional_many_group(vec![
        one(vec![Token::ArgumentDelimiter]),
        optional_many(vec![Token::whitespace()]),
        one(vec![
          Token::string(),
          Token::r#true(),
          Token::r#false(),
          Token::int(),
          Token::float(),
        ]),
        optional_many(vec![Token::whitespace()]),
      ]),
    ]),
    one(vec![Token::BraceClose]),
    optional_many(vec![Token::whitespace(), Token::Newline]),
    one(vec![Token::HtmlCommentClose]),
  ]
}

pub fn optional_group(matchers: Vec<PatternMatcher>) -> PatternMatcher {
  let method = group(matchers);
  Box::new(move |token_group: &TokenGroup, index: usize| {
    match method(token_group, index) {
      Ok(index) => Ok(index),
      Err(_) => Ok(index),
    }
  })
}

pub fn group(matchers: Vec<PatternMatcher>) -> PatternMatcher {
  Box::new(move |token_group: &TokenGroup, index: usize| {
    let mut next_index = index;

    for matcher in matchers.iter() {
      next_index = matcher(token_group, next_index)?;
    }

    Ok(next_index)
  })
}

pub fn optional_many_group(matchers: Vec<PatternMatcher>) -> PatternMatcher {
  let method = many_group(matchers);
  Box::new(move |token_group: &TokenGroup, index: usize| {
    match method(token_group, index) {
      Ok(index) => Ok(index),
      Err(_) => Ok(index),
    }
  })
}

pub fn many_group(matchers: Vec<PatternMatcher>) -> PatternMatcher {
  let method = group(matchers);
  Box::new(move |token_group: &TokenGroup, index: usize| {
    let mut next_index = method(token_group, index)?;

    while let Ok(index) = method(token_group, next_index) {
      next_index = index
    }

    Ok(next_index)
  })
}

pub fn optional(tokens: Vec<Token>) -> PatternMatcher {
  let method = one(tokens);
  Box::new(move |token_group: &TokenGroup, index: usize| {
    match method(token_group, index) {
      Ok(index) => Ok(index),
      Err(_) => Ok(index),
    }
  })
}

pub fn one(tokens: Vec<Token>) -> PatternMatcher {
  Box::new(move |token_group: &TokenGroup, index: usize| {
    let Some(slice) = token_group.tokens.get(index) else {
        return Err(MdtError::InvalidTokenSequence(index));
      };

    if tokens.iter().any(|token| token.same_type(slice)) {
      return Ok(index + 1);
    }

    Err(MdtError::InvalidTokenSequence(index))
  })
}

pub fn optional_many(tokens: Vec<Token>) -> PatternMatcher {
  let method = many(tokens);
  Box::new(move |token_group: &TokenGroup, index: usize| {
    match method(token_group, index) {
      Ok(index) => Ok(index),
      Err(_) => Ok(index),
    }
  })
}

pub fn many(tokens: Vec<Token>) -> PatternMatcher {
  Box::new(move |token_group: &TokenGroup, index: usize| {
    let Some(slice) = token_group.tokens.get(index..) else {
        return Err(MdtError::InvalidTokenSequence(index));
      };

    let mut next_index = index;

    for item in slice {
      if tokens.iter().any(|token| token.same_type(item)) {
        next_index += 1;
      } else {
        break;
      }
    }

    if next_index > index {
      return Ok(next_index);
    }

    Err(MdtError::InvalidTokenSequence(index))
  })
}

impl TokenGroup {
  /// Checks if the token group matches the given pattern. Returns a result
  /// wrapped in a boolean if the pattern matches otherwise it returns an
  /// error.
  pub fn matches_pattern(&self, pattern: Vec<PatternMatcher>) -> MdtResult<bool> {
    let mut index = 0;

    for matcher in pattern.iter() {
      index = matcher(self, index)?;
    }

    Ok(index == self.tokens.len())
  }

  pub fn is_valid(&self) -> bool {
    let patterns = vec![closing_pattern(), provider_pattern(), consumer_pattern()];

    for pattern in patterns {
      let Some(result) = self.matches_pattern(pattern).ok() else {
        continue;
      };

      if result {
        return true;
      }
    }

    false
  }
}

impl Token {
  fn whitespace() -> Self {
    Self::Whitespace(b'*')
  }

  fn r#true() -> Self {
    Self::Ident("true".into())
  }

  fn r#false() -> Self {
    Self::Ident("false".into())
  }

  fn any() -> Self {
    Self::Ident("*".into())
  }

  fn string() -> Self {
    Self::String("*".into(), b'"')
  }

  fn int() -> Self {
    Self::Int(0)
  }

  fn float() -> Self {
    Self::Float(0.0)
  }
}
