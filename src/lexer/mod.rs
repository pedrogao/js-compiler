#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    Number(f64),
    StringLiteral(String),
    Identifier(String),
    True,
    False,
    Null,

    // Keywords
    Function,
    Let,
    Return,
    If,
    Else,
    While,

    // Operators
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Equal,
    EqualEqual,
    NotEqual,
    LessThan,    // Changed from Less
    GreaterThan, // Changed from Greater
    LessEqual,
    GreaterEqual,
    Not,
    And,
    Or,

    // Delimiters
    LParen, // (
    RParen, // )
    LBrace, // {
    RBrace, // }
    Semicolon,
    Comma,
    QuestionMark,
    Colon,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub line: usize,
    pub column: usize,
}

impl Token {
    fn new(token_type: TokenType, line: usize, column: usize) -> Self {
        Token {
            token_type,
            line,
            column,
        }
    }
}

pub fn tokenize(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    let mut line = 1;
    let mut column = 1;

    while let Some(&c) = chars.peek() {
        match c {
            // Skip whitespace
            ' ' | '\t' | '\r' => {
                column += 1;
                chars.next();
            }

            '\n' => {
                line += 1;
                column = 1;
                chars.next();
            }

            // Numbers
            '0'..='9' => {
                let mut number = String::new();
                let start_column = column;

                while let Some(&c) = chars.peek() {
                    if c.is_digit(10) || c == '.' {
                        number.push(chars.next().unwrap());
                        column += 1;
                    } else {
                        break;
                    }
                }

                tokens.push(Token::new(
                    TokenType::Number(number.parse().unwrap()),
                    line,
                    start_column,
                ));
            }

            // Identifiers and Keywords
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut ident = String::new();
                let start_column = column;

                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(chars.next().unwrap());
                        column += 1;
                    } else {
                        break;
                    }
                }

                let token_type = match ident.as_str() {
                    "function" => TokenType::Function,
                    "let" => TokenType::Let,
                    "return" => TokenType::Return,
                    "if" => TokenType::If,
                    "else" => TokenType::Else,
                    "while" => TokenType::While,
                    "true" => TokenType::True,
                    "false" => TokenType::False,
                    "null" => TokenType::Null,
                    _ => TokenType::Identifier(ident),
                };

                tokens.push(Token::new(token_type, line, start_column));
            }

            // String Literals
            '"' | '\'' => {
                chars.next(); // consume quote
                column += 1;
                let quote = c;
                let mut string = String::new();
                let start_column = column;

                while let Some(&c) = chars.peek() {
                    chars.next();
                    column += 1;

                    if c == quote {
                        break;
                    } else if c == '\\' {
                        if let Some(&escaped) = chars.peek() {
                            chars.next();
                            column += 1;
                            match escaped {
                                'n' => string.push('\n'),
                                't' => string.push('\t'),
                                'r' => string.push('\r'),
                                '\\' => string.push('\\'),
                                '"' => string.push('"'),
                                '\'' => string.push('\''),
                                _ => panic!("Invalid escape sequence: \\{}", escaped),
                            }
                        }
                    } else {
                        string.push(c);
                    }
                }

                tokens.push(Token::new(
                    TokenType::StringLiteral(string),
                    line,
                    start_column,
                ));
            }

            // Comments
            '/' => {
                chars.next();
                column += 1;
                match chars.peek() {
                    Some(&'/') => {
                        // Single-line comment
                        while let Some(&c) = chars.peek() {
                            if c == '\n' {
                                break;
                            }
                            chars.next();
                            column += 1;
                        }
                    }
                    Some(&'*') => {
                        // Multi-line comment
                        chars.next();
                        column += 1;
                        let mut nesting = 1;
                        while nesting > 0 {
                            match chars.next() {
                                Some('*') => {
                                    if let Some(&'/') = chars.peek() {
                                        chars.next();
                                        nesting -= 1;
                                    }
                                    column += 1;
                                }
                                Some('/') => {
                                    if let Some(&'*') = chars.peek() {
                                        chars.next();
                                        nesting += 1;
                                    }
                                    column += 1;
                                }
                                Some('\n') => {
                                    line += 1;
                                    column = 1;
                                }
                                Some(_) => column += 1,
                                None => panic!("Unterminated multi-line comment"),
                            }
                        }
                    }
                    _ => tokens.push(Token::new(TokenType::Divide, line, column - 1)),
                }
            }

            // Operators and punctuation
            '+' => {
                chars.next();
                tokens.push(Token::new(TokenType::Plus, line, column));
                column += 1;
            }
            '-' => {
                chars.next();
                tokens.push(Token::new(TokenType::Minus, line, column));
                column += 1;
            }
            '*' => {
                chars.next();
                tokens.push(Token::new(TokenType::Multiply, line, column));
                column += 1;
            }
            '%' => {
                chars.next();
                tokens.push(Token::new(TokenType::Modulo, line, column));
                column += 1;
            }
            '(' => {
                chars.next();
                tokens.push(Token::new(TokenType::LParen, line, column));
                column += 1;
            }
            ')' => {
                chars.next();
                tokens.push(Token::new(TokenType::RParen, line, column));
                column += 1;
            }
            '{' => {
                chars.next();
                tokens.push(Token::new(TokenType::LBrace, line, column));
                column += 1;
            }
            '}' => {
                chars.next();
                tokens.push(Token::new(TokenType::RBrace, line, column));
                column += 1;
            }
            ';' => {
                chars.next();
                tokens.push(Token::new(TokenType::Semicolon, line, column));
                column += 1;
            }
            ',' => {
                chars.next();
                tokens.push(Token::new(TokenType::Comma, line, column));
                column += 1;
            }
            '?' => {
                chars.next();
                tokens.push(Token::new(TokenType::QuestionMark, line, column));
                column += 1;
            }
            ':' => {
                chars.next();
                tokens.push(Token::new(TokenType::Colon, line, column));
                column += 1;
            }

            // Two-character operators
            '=' => {
                chars.next();
                column += 1;
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    column += 1;
                    tokens.push(Token::new(TokenType::EqualEqual, line, column - 2));
                } else {
                    tokens.push(Token::new(TokenType::Equal, line, column - 1));
                }
            }
            '!' => {
                chars.next();
                column += 1;
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    column += 1;
                    tokens.push(Token::new(TokenType::NotEqual, line, column - 2));
                } else {
                    tokens.push(Token::new(TokenType::Not, line, column - 1));
                }
            }
            '<' => {
                chars.next();
                column += 1;
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    column += 1;
                    tokens.push(Token::new(TokenType::LessEqual, line, column - 2));
                } else {
                    tokens.push(Token::new(TokenType::LessThan, line, column - 1));
                }
            }
            '>' => {
                chars.next();
                column += 1;
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    column += 1;
                    tokens.push(Token::new(TokenType::GreaterEqual, line, column - 2));
                } else {
                    tokens.push(Token::new(TokenType::GreaterThan, line, column - 1));
                }
            }
            '&' => {
                chars.next();
                column += 1;
                if let Some(&'&') = chars.peek() {
                    chars.next();
                    column += 1;
                    tokens.push(Token::new(TokenType::And, line, column - 2));
                } else {
                    panic!("Expected '&&', got single '&'");
                }
            }
            '|' => {
                chars.next();
                column += 1;
                if let Some(&'|') = chars.peek() {
                    chars.next();
                    column += 1;
                    tokens.push(Token::new(TokenType::Or, line, column - 2));
                } else {
                    panic!("Expected '||', got single '|'");
                }
            }

            _ => panic!("Unexpected character: {}", c),
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let input = "let x = 5;";
        let tokens = tokenize(input);

        assert_eq!(tokens[0].token_type, TokenType::Let);
        assert_eq!(tokens[1].token_type, TokenType::Identifier("x".to_string()));
        assert_eq!(tokens[2].token_type, TokenType::Equal);
        assert_eq!(tokens[3].token_type, TokenType::Number(5.0));
        assert_eq!(tokens[4].token_type, TokenType::Semicolon);
    }

    #[test]
    fn test_operators() {
        let input = "+ - * / = == != < > <= >=";
        let tokens = tokenize(input);

        let expected = vec![
            TokenType::Plus,
            TokenType::Minus,
            TokenType::Multiply,
            TokenType::Divide,
            TokenType::Equal,
            TokenType::EqualEqual,
            TokenType::NotEqual,
            TokenType::LessThan,
            TokenType::GreaterThan,
            TokenType::LessEqual,
            TokenType::GreaterEqual,
        ];

        for (i, expected_type) in expected.into_iter().enumerate() {
            assert_eq!(tokens[i].token_type, expected_type);
        }
    }

    #[test]
    fn test_keywords() {
        let input = "function let return if else while true false null";
        let tokens = tokenize(input);

        let expected = vec![
            TokenType::Function,
            TokenType::Let,
            TokenType::Return,
            TokenType::If,
            TokenType::Else,
            TokenType::While,
            TokenType::True,
            TokenType::False,
            TokenType::Null,
        ];

        for (i, expected_type) in expected.into_iter().enumerate() {
            assert_eq!(tokens[i].token_type, expected_type);
        }
    }
}
