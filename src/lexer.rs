use std::fmt;
use std::str::Chars;

pub struct Lexer<'input> {
    src: Chars<'input>,
    line: usize,
    peek: Option<char>,
    next: Option<Kind>,
    last: Option<Kind>,
}

impl<'input> Lexer<'input> {
    pub fn new(src: &'input str) -> Self {
        Self {
            src: src.chars(),
            line: 1,
            peek: None,
            next: None,
            last: None,
        }
    }

    fn emit(&mut self, kind: Kind, line: Option<usize>) -> Option<Token> {
        self.last = Some(kind.clone());
        Some(Token::new(kind, line.unwrap_or(self.line)))
    }

    fn next_char(&mut self) -> Option<char> {
        let next = self.peek.take().or_else(|| self.src.next());
        if next == Some('\n') {
            self.line += 1;
        }
        next
    }

    fn push_char(&mut self, c: char) {
        assert!(self.peek.is_none());
        if c == '\n' {
            self.line -= 1;
        }
        self.peek = Some(c);
    }

    fn consume_line_terminators(&mut self) -> usize {
        let line = self.line;
        while let Some(c) = self.next_char() {
            if !is_line_terminator(c) {
                self.push_char(c);
                break;
            }
        }
        line
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(kind) = self.next.take() {
            return self.emit(kind, None);
        }

        let mut buf = String::new();

        while let Some(c) = self.next_char() {
            if buf.is_empty() {
                if c == '{' {
                    let line = self.consume_line_terminators();
                    return self.emit(Kind::LeftBrace, Some(line));
                }
                if c == '}' {
                    let kind = if self.last != Some(Kind::Semi) {
                        self.next = Some(Kind::RightBrace);
                        Kind::Semi
                    } else {
                        Kind::RightBrace
                    };
                    return self.emit(kind, None);
                }
            }

            if is_line_terminator(c) {
                if buf.is_empty() {
                    let line = self.consume_line_terminators();
                    return if self.last.is_none() {
                        // Don't emit leading delimiters.
                        self.next()
                    } else {
                        self.emit(Kind::Semi, Some(line - 1))
                    };
                } else {
                    self.push_char(c);
                    break;
                }
            }

            if c.is_whitespace() {
                if buf.is_empty() {
                    // Ignore consecutive whitespace.
                    continue;
                } else {
                    // At the end of a token.
                    break;
                }
            }

            buf.push(c);
        }

        if buf.is_empty() {
            // Ensure we always emit a trailing semi to reduce
            // edge cases in the parser.
            if self.last == Some(Kind::Semi) {
                None
            } else {
                self.emit(Kind::Semi, None)
            }
        } else {
            self.emit(Kind::Word(buf), None)
        }
    }
}

fn is_line_terminator(c: char) -> bool {
    c == '\n' || c == ';'
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: Kind,
    pub line: usize,
}

impl Token {
    fn new(kind: Kind, line: usize) -> Self {
        Self { kind, line }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Word(String),
    LeftBrace,
    RightBrace,
    Semi,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match *self {
            Kind::Word(ref word) => word,
            Kind::LeftBrace => "{",
            Kind::RightBrace => "}",
            Kind::Semi => ";",
        };

        write!(f, "'{}'", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command() {
        let tokens: Vec<Kind> = Lexer::new("cat /etc/hosts /etc/passwd")
            .map(|t| t.kind)
            .collect();
        assert_eq!(
            tokens,
            vec![
                Kind::Word("cat".into()),
                Kind::Word("/etc/hosts".into()),
                Kind::Word("/etc/passwd".into()),
                Kind::Semi,
            ],
        );
    }

    #[test]
    fn if_stmt() {
        let tokens: Vec<Kind> = Lexer::new("if true { echo truthy }\n")
            .map(|t| t.kind)
            .collect();
        assert_eq!(
            tokens,
            vec![
                Kind::Word("if".into()),
                Kind::Word("true".into()),
                Kind::LeftBrace,
                Kind::Word("echo".into()),
                Kind::Word("truthy".into()),
                Kind::Semi,
                Kind::RightBrace,
                Kind::Semi,
            ],
        );
    }

    #[test]
    fn empty_body() {
        let tokens: Vec<Kind> = Lexer::new("if false { }\n").map(|t| t.kind).collect();
        assert_eq!(
            tokens,
            vec![
                Kind::Word("if".into()),
                Kind::Word("false".into()),
                Kind::LeftBrace,
                Kind::Semi,
                Kind::RightBrace,
                Kind::Semi,
            ],
        );
    }

    #[test]
    fn multiline_nested_if_else_stmt() {
        let src = r#"if /bin/a {
  echo a
} else if /bin/b {
  echo b
  echo 2
  if true {
    exit
  }
} else {
  echo c
}
"#;
        let tokens: Vec<Token> = Lexer::new(src).collect();
        assert_eq!(
            tokens,
            vec![
                Token::new(Kind::Word("if".into()), 1),
                Token::new(Kind::Word("/bin/a".into()), 1),
                Token::new(Kind::LeftBrace, 1),
                Token::new(Kind::Word("echo".into()), 2),
                Token::new(Kind::Word("a".into()), 2),
                Token::new(Kind::Semi, 2),
                Token::new(Kind::RightBrace, 3),
                Token::new(Kind::Word("else".into()), 3),
                Token::new(Kind::Word("if".into()), 3),
                Token::new(Kind::Word("/bin/b".into()), 3),
                Token::new(Kind::LeftBrace, 3),
                Token::new(Kind::Word("echo".into()), 4),
                Token::new(Kind::Word("b".into()), 4),
                Token::new(Kind::Semi, 4),
                Token::new(Kind::Word("echo".into()), 5),
                Token::new(Kind::Word("2".into()), 5),
                Token::new(Kind::Semi, 5),
                Token::new(Kind::Word("if".into()), 6),
                Token::new(Kind::Word("true".into()), 6),
                Token::new(Kind::LeftBrace, 6),
                Token::new(Kind::Word("exit".into()), 7),
                Token::new(Kind::Semi, 7),
                Token::new(Kind::RightBrace, 8),
                Token::new(Kind::Semi, 8),
                Token::new(Kind::RightBrace, 9),
                Token::new(Kind::Word("else".into()), 9),
                Token::new(Kind::LeftBrace, 9),
                Token::new(Kind::Word("echo".into()), 10),
                Token::new(Kind::Word("c".into()), 10),
                Token::new(Kind::Semi, 10),
                Token::new(Kind::RightBrace, 11),
                Token::new(Kind::Semi, 11),
            ],
        );
    }
}
