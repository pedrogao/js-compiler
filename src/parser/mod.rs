use crate::lexer::{Token, TokenType};

#[derive(Debug, Clone)]
pub enum Expression {
    // Literals
    Number(f64),
    String(String),
    Boolean(bool),
    Null,

    // Variables and Functions
    Identifier(String),
    FunctionCall {
        name: String,
        arguments: Vec<Expression>,
    },

    // Operators
    BinaryOp {
        op: String,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    UnaryOp {
        op: String,
        expr: Box<Expression>,
    },

    // Control Flow
    Conditional {
        condition: Box<Expression>,
        then_expr: Box<Expression>,
        else_expr: Box<Expression>,
    },
}

#[derive(Debug, Clone)]
pub enum Statement {
    // Variable Declaration
    Let {
        name: String,
        initializer: Expression,
    },

    // Control Flow
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },

    // Functions
    FunctionDeclaration {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    Return(Option<Expression>),

    // Other
    Block(Vec<Statement>),
    ExpressionStatement(Expression),
}

#[derive(Debug)]
pub struct AST {
    pub statements: Vec<Statement>,
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.current < self.tokens.len() {
            self.current += 1;
            Some(self.tokens[self.current - 1].clone())
        } else {
            None
        }
    }

    fn parse_function(&mut self) -> Statement {
        self.advance(); // consume 'function'
        let name = match self.advance().unwrap().token_type {
            TokenType::Identifier(name) => name,
            _ => panic!("Expected function name"),
        };

        let mut params = Vec::new();
        self.advance(); // consume '('

        while let Some(token) = self.peek() {
            match &token.token_type {
                TokenType::RParen => {
                    self.advance();
                    break;
                }
                TokenType::Identifier(param) => {
                    params.push(param.clone());
                    self.advance();
                    if let Some(Token {
                        token_type: TokenType::Comma,
                        ..
                    }) = self.peek()
                    {
                        self.advance();
                    }
                }
                _ => panic!("Invalid parameter"),
            }
        }

        let mut body = Vec::new();
        self.advance(); // consume '{'

        while let Some(token) = self.peek() {
            match &token.token_type {
                TokenType::RBrace => {
                    self.advance();
                    break;
                }
                _ => body.push(self.parse_statement()),
            }
        }

        Statement::FunctionDeclaration { name, params, body }
    }

    fn parse_statement(&mut self) -> Statement {
        match self.peek().unwrap().token_type {
            TokenType::Function => self.parse_function(),
            TokenType::Let => self.parse_let_statement(),
            TokenType::Return => self.parse_return_statement(),
            TokenType::If => self.parse_if_statement(),
            TokenType::While => self.parse_while_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_let_statement(&mut self) -> Statement {
        self.advance(); // consume 'let'

        let name = match self.advance().unwrap().token_type {
            TokenType::Identifier(name) => name,
            _ => panic!("Expected identifier after 'let'"),
        };

        match self.advance().unwrap().token_type {
            TokenType::Equal => {}
            _ => panic!("Expected '=' after identifier in let statement"),
        }

        let initializer = self.parse_expression();

        match self.advance().unwrap().token_type {
            TokenType::Semicolon => {}
            _ => panic!("Expected ';' after let statement"),
        }

        Statement::Let { name, initializer }
    }

    fn parse_return_statement(&mut self) -> Statement {
        self.advance(); // consume 'return'

        let expr = if let Some(token) = self.peek() {
            if matches!(token.token_type, TokenType::Semicolon) {
                None
            } else {
                Some(self.parse_expression())
            }
        } else {
            None
        };

        match self.advance().unwrap().token_type {
            TokenType::Semicolon => {}
            _ => panic!("Expected ';' after return statement"),
        }

        Statement::Return(expr)
    }

    fn parse_expression_statement(&mut self) -> Statement {
        let expr = self.parse_expression();

        match self.advance().unwrap().token_type {
            TokenType::Semicolon => {}
            _ => panic!("Expected ';' after expression statement"),
        }

        Statement::ExpressionStatement(expr)
    }

    fn parse_expression(&mut self) -> Expression {
        self.parse_conditional()
    }

    fn parse_conditional(&mut self) -> Expression {
        let mut expr = self.parse_logical_or();

        if let Some(token) = self.peek() {
            if matches!(token.token_type, TokenType::QuestionMark) {
                self.advance(); // consume ?
                let then_expr = self.parse_expression();
                self.expect_token(TokenType::Colon);
                let else_expr = self.parse_conditional();
                expr = Expression::Conditional {
                    condition: Box::new(expr),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                };
            }
        }
        expr
    }

    fn parse_logical_or(&mut self) -> Expression {
        let mut expr = self.parse_logical_and();

        while let Some(token) = self.peek() {
            if matches!(token.token_type, TokenType::Or) {
                self.advance();
                let right = self.parse_logical_and();
                expr = Expression::BinaryOp {
                    op: "||".to_string(),
                    left: Box::new(expr),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        expr
    }

    fn parse_logical_and(&mut self) -> Expression {
        let mut expr = self.parse_equality();

        while let Some(token) = self.peek() {
            if matches!(token.token_type, TokenType::And) {
                self.advance();
                let right = self.parse_equality();
                expr = Expression::BinaryOp {
                    op: "&&".to_string(),
                    left: Box::new(expr),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        expr
    }

    fn parse_equality(&mut self) -> Expression {
        let mut expr = self.parse_comparison();

        while let Some(token) = self.peek() {
            let op = match &token.token_type {
                TokenType::EqualEqual => "==",
                TokenType::NotEqual => "!=",
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison();
            expr = Expression::BinaryOp {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    fn parse_comparison(&mut self) -> Expression {
        let mut expr = self.parse_term();

        while let Some(token) = self.peek() {
            let op = match &token.token_type {
                TokenType::LessThan => "<",
                TokenType::GreaterThan => ">",
                TokenType::LessEqual => "<=",
                TokenType::GreaterEqual => ">=",
                _ => break,
            };
            self.advance();
            let right = self.parse_term();
            expr = Expression::BinaryOp {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    fn parse_term(&mut self) -> Expression {
        let mut expr = self.parse_factor();

        while let Some(token) = self.peek() {
            let op = match &token.token_type {
                TokenType::Plus => "+",
                TokenType::Minus => "-",
                _ => break,
            };
            self.advance();
            let right = self.parse_factor();
            expr = Expression::BinaryOp {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    fn parse_factor(&mut self) -> Expression {
        let mut expr = self.parse_unary();

        while let Some(token) = self.peek() {
            let op = match &token.token_type {
                TokenType::Multiply => "*",
                TokenType::Divide => "/",
                TokenType::Modulo => "%",
                _ => break,
            };
            self.advance();
            let right = self.parse_unary();
            expr = Expression::BinaryOp {
                op: op.to_string(),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    fn parse_unary(&mut self) -> Expression {
        if let Some(token) = self.peek() {
            match &token.token_type {
                TokenType::Not | TokenType::Minus => {
                    let token_type = token.token_type.clone();
                    self.advance();
                    let op = match token_type {
                        TokenType::Not => "!",
                        TokenType::Minus => "-",
                        _ => unreachable!(),
                    };
                    let expr = self.parse_unary();
                    return Expression::UnaryOp {
                        op: op.to_string(),
                        expr: Box::new(expr),
                    };
                }
                _ => {}
            }
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Expression {
        let token = self.advance().expect("Expected expression");
        match token.token_type {
            TokenType::Number(n) => Expression::Number(n),
            TokenType::StringLiteral(s) => Expression::String(s),
            TokenType::True => Expression::Boolean(true),
            TokenType::False => Expression::Boolean(false),
            TokenType::Null => Expression::Null,
            TokenType::Identifier(name) => {
                if let Some(token) = self.peek() {
                    if matches!(token.token_type, TokenType::LParen) {
                        return self.parse_function_call(name);
                    }
                }
                Expression::Identifier(name)
            }
            TokenType::LParen => {
                let expr = self.parse_expression();
                self.expect_token(TokenType::RParen);
                expr
            }
            _ => panic!("Unexpected token in expression: {:?}", token),
        }
    }

    fn parse_function_call(&mut self, name: String) -> Expression {
        self.advance(); // consume '('

        let mut arguments = Vec::new();

        loop {
            match self.peek().unwrap().token_type {
                TokenType::RParen => {
                    self.advance();
                    break;
                }
                _ => {
                    arguments.push(self.parse_expression());
                    match self.peek().unwrap().token_type {
                        TokenType::Comma => {
                            self.advance();
                        }
                        TokenType::RParen => {}
                        _ => panic!("Expected ',' or ')' in function call"),
                    }
                }
            }
        }

        Expression::FunctionCall { name, arguments }
    }

    fn expect_token(&mut self, expected: TokenType) -> Token {
        let token = self.advance().unwrap();
        if token.token_type != expected {
            panic!("Expected {:?}, got {:?}", expected, token.token_type);
        }
        token
    }

    fn parse_if_statement(&mut self) -> Statement {
        self.advance(); // consume 'if'
        self.expect_token(TokenType::LParen);
        let condition = self.parse_expression();
        self.expect_token(TokenType::RParen);

        let then_branch = self.parse_block();

        let else_branch = if let Some(token) = self.peek() {
            if matches!(token.token_type, TokenType::Else) {
                self.advance(); // consume 'else'
                Some(self.parse_block())
            } else {
                None
            }
        } else {
            None
        };

        Statement::If {
            condition,
            then_branch,
            else_branch,
        }
    }

    fn parse_while_statement(&mut self) -> Statement {
        self.advance(); // consume 'while'
        self.expect_token(TokenType::LParen);
        let condition = self.parse_expression();
        self.expect_token(TokenType::RParen);

        let body = self.parse_block();

        Statement::While { condition, body }
    }

    fn parse_block(&mut self) -> Vec<Statement> {
        self.expect_token(TokenType::LBrace);

        let mut statements = Vec::new();
        while let Some(token) = self.peek() {
            if matches!(token.token_type, TokenType::RBrace) {
                break;
            }
            statements.push(self.parse_statement());
        }

        self.expect_token(TokenType::RBrace);
        statements
    }
}

pub fn parse(tokens: Vec<Token>) -> AST {
    let mut parser = Parser::new(tokens);
    let mut statements = Vec::new();

    while parser.peek().is_some() {
        statements.push(parser.parse_statement());
    }

    AST { statements }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn test_let_statement() {
        let input = "let x = 5;";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);

        let statements = vec![parser.parse_statement()];

        match &statements[0] {
            Statement::Let { name, initializer } => {
                assert_eq!(name, "x");
                match initializer {
                    Expression::Number(val) => assert_eq!(*val, 5.0),
                    _ => panic!("Expected number expression"),
                }
            }
            _ => panic!("Expected let statement"),
        }
    }

    #[test]
    fn test_return_statement() {
        let input = "return 10;";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);

        let statements = vec![parser.parse_statement()];

        match &statements[0] {
            Statement::Return(Some(expr)) => match expr {
                Expression::Number(val) => assert_eq!(*val, 10.0),
                _ => panic!("Expected number expression"),
            },
            _ => panic!("Expected return statement"),
        }
    }

    #[test]
    fn test_if_statement() {
        let input = "if (x > 5) { return true; }";
        let tokens = tokenize(input);
        let mut parser = Parser::new(tokens);

        let statements = vec![parser.parse_statement()];

        match &statements[0] {
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                assert!(else_branch.is_none());
                match condition {
                    Expression::BinaryOp { op, left, right } => {
                        assert_eq!(op, ">");
                        match &**left {
                            Expression::Identifier(name) => assert_eq!(name, "x"),
                            _ => panic!("Expected identifier"),
                        }
                        match &**right {
                            Expression::Number(val) => assert_eq!(*val, 5.0),
                            _ => panic!("Expected number"),
                        }
                    }
                    _ => panic!("Expected binary operation"),
                }
            }
            _ => panic!("Expected if statement"),
        }
    }
}
