//! CODIE Lexer - Tokenizes CODIE source code
//!
//! Extended to handle:
//! - 40 keywords (core + logic + control + bool + geometric + dimensional)
//! - All operators (arithmetic, comparison, logical, bitwise)
//! - Tree structure symbols (├── └── │)
//! - Geometric arrows (→ ← ↔)
//! - Dimensional indexing (dim[n], axis.x)

use crate::parser::ParseError;
use crate::token::{CodieKeyword, CodieToken};
use thiserror::Error;

/// Lexer errors
#[derive(Debug, Error)]
pub enum LexerError {
    #[error("Unexpected character '{0}' at line {1}, column {2}")]
    UnexpectedChar(char, usize, usize),

    #[error("Unterminated string at line {0}")]
    UnterminatedString(usize),

    #[error("Invalid hash reference at line {0}")]
    InvalidHashRef(usize),

    #[error("Invalid number at line {0}")]
    InvalidNumber(usize),
}

impl From<LexerError> for ParseError {
    fn from(e: LexerError) -> Self {
        ParseError::LexerError(e.to_string())
    }
}

/// The CODIE lexer
pub struct CodieLexer<'a> {
    input: &'a str,
    chars: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl<'a> CodieLexer<'a> {
    /// Create a new lexer for the given input
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    /// Get input string (for error reporting)
    #[allow(dead_code)]
    pub fn input(&self) -> &str {
        self.input
    }

    /// Peek at the current character without consuming
    pub fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    /// Peek at character at offset from current position
    fn peek_ahead(&self, offset: usize) -> Option<char> {
        self.chars.get(self.position + offset).copied()
    }

    /// Advance to the next character
    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.position += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    /// Check if at end of input
    fn is_at_end(&self) -> bool {
        self.position >= self.chars.len()
    }

    /// Skip whitespace (but not newlines)
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Count leading spaces for indentation
    #[allow(dead_code)]
    fn count_indent(&mut self) -> usize {
        let mut count = 0;
        while let Some(c) = self.peek() {
            match c {
                ' ' => {
                    count += 1;
                    self.advance();
                }
                '\t' => {
                    count += 4; // Tab = 4 spaces
                    self.advance();
                }
                _ => break,
            }
        }
        count
    }

    /// Read an identifier or keyword
    fn read_identifier(&mut self) -> String {
        let mut result = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                result.push(c);
                self.advance();
            } else {
                break;
            }
        }
        result
    }

    /// Read a string literal
    fn read_string(&mut self) -> Result<String, LexerError> {
        let start_line = self.line;
        self.advance(); // consume opening quote

        let mut result = String::new();
        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance(); // consume closing quote
                return Ok(result);
            } else if c == '\\' {
                self.advance();
                if let Some(escaped) = self.peek() {
                    self.advance();
                    match escaped {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '"' => result.push('"'),
                        '\\' => result.push('\\'),
                        _ => result.push(escaped),
                    }
                }
            } else if c == '\n' {
                return Err(LexerError::UnterminatedString(start_line));
            } else {
                result.push(c);
                self.advance();
            }
        }
        Err(LexerError::UnterminatedString(start_line))
    }

    /// Read a number literal
    fn read_number(&mut self) -> Result<f64, LexerError> {
        let start_line = self.line;
        let mut num_str = String::new();

        // Handle negative
        if self.peek() == Some('-') {
            num_str.push('-');
            self.advance();
        }

        // Read integer part
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Read decimal part
        if self.peek() == Some('.') && self.peek_ahead(1).map_or(false, |c| c.is_ascii_digit()) {
            num_str.push('.');
            self.advance();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    num_str.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        num_str
            .parse()
            .map_err(|_| LexerError::InvalidNumber(start_line))
    }

    /// Read a hash reference (#abc123)
    fn read_hash_ref(&mut self) -> Result<String, LexerError> {
        let start_line = self.line;
        self.advance(); // consume #

        let mut result = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_hexdigit() || c == ':' || c.is_alphanumeric() {
                result.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if result.is_empty() {
            Err(LexerError::InvalidHashRef(start_line))
        } else {
            Ok(result)
        }
    }

    /// Read a source reference (@database/path)
    fn read_source_ref(&mut self) -> String {
        self.advance(); // consume @
        let mut result = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric()
                || c == '/'
                || c == '_'
                || c == '-'
                || c == '.'
                || c == '?'
                || c == '='
                || c == '{'
                || c == '}'
            {
                result.push(c);
                self.advance();
            } else {
                break;
            }
        }
        result
    }

    /// Read a vault reference ($vault/key)
    fn read_vault_ref(&mut self) -> String {
        self.advance(); // consume $
        let mut result = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '/' || c == '_' || c == '-' {
                result.push(c);
                self.advance();
            } else {
                break;
            }
        }
        result
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Result<CodieToken, LexerError> {
        // Check for end of input
        if self.is_at_end() {
            return Ok(CodieToken::Eof);
        }

        // Handle newlines specially
        if self.peek() == Some('\n') {
            self.advance();
            return Ok(CodieToken::Newline);
        }

        // Skip whitespace
        self.skip_whitespace();

        if self.is_at_end() {
            return Ok(CodieToken::Eof);
        }

        let c = self.peek().unwrap();

        // Check for tree structure characters (Unicode box drawing)
        if c == '│' {
            self.advance();
            return Ok(CodieToken::Pipe);
        }
        if c == '├' {
            self.advance();
            // Consume ── if present
            if self.peek() == Some('─') {
                self.advance();
                if self.peek() == Some('─') {
                    self.advance();
                }
            }
            return Ok(CodieToken::TreeBranch);
        }
        if c == '└' {
            self.advance();
            if self.peek() == Some('─') {
                self.advance();
                if self.peek() == Some('─') {
                    self.advance();
                }
            }
            return Ok(CodieToken::TreeLastBranch);
        }

        // Unicode arrows
        if c == '→' {
            self.advance();
            return Ok(CodieToken::Arrow);
        }
        if c == '←' {
            self.advance();
            return Ok(CodieToken::BackArrow);
        }
        if c == '↔' {
            self.advance();
            return Ok(CodieToken::BiArrow);
        }

        // Multi-char and single-char tokens
        match c {
            // Dots and ellipsis
            '.' => {
                self.advance();
                if self.peek() == Some('.') {
                    self.advance();
                    if self.peek() == Some('.') {
                        self.advance();
                        Ok(CodieToken::Ellipsis)
                    } else {
                        Ok(CodieToken::DotDot)
                    }
                } else {
                    Ok(CodieToken::Dot)
                }
            }

            // Colons
            ':' => {
                self.advance();
                if self.peek() == Some(':') {
                    self.advance();
                    Ok(CodieToken::DoubleColon)
                } else {
                    Ok(CodieToken::Colon)
                }
            }

            // Comparison and assignment
            '=' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(CodieToken::DoubleEquals)
                } else {
                    Ok(CodieToken::Equals)
                }
            }
            '!' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(CodieToken::NotEquals)
                } else {
                    Ok(CodieToken::Bang)
                }
            }
            '<' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(CodieToken::LessEquals)
                } else {
                    Ok(CodieToken::OpenAngle)
                }
            }
            '>' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(CodieToken::GreaterEquals)
                } else {
                    Ok(CodieToken::CloseAngle)
                }
            }

            // Logical operators (symbol form)
            '&' => {
                self.advance();
                if self.peek() == Some('&') {
                    self.advance();
                    Ok(CodieToken::DoubleAnd)
                } else {
                    Ok(CodieToken::BitAnd)
                }
            }
            '|' => {
                self.advance();
                if self.peek() == Some('|') {
                    self.advance();
                    Ok(CodieToken::DoubleOr)
                } else {
                    Ok(CodieToken::BitOr)
                }
            }

            // Arithmetic operators
            '+' => {
                self.advance();
                Ok(CodieToken::Plus)
            }
            '-' => {
                // Could be minus or negative number
                if self.peek_ahead(1).map_or(false, |n| n.is_ascii_digit()) {
                    let n = self.read_number()?;
                    Ok(CodieToken::NumberLiteral(n))
                } else {
                    self.advance();
                    Ok(CodieToken::Minus)
                }
            }
            '*' => {
                self.advance();
                Ok(CodieToken::Star)
            }
            '/' => {
                self.advance();
                Ok(CodieToken::Slash)
            }
            '%' => {
                self.advance();
                Ok(CodieToken::Percent)
            }
            '^' => {
                self.advance();
                Ok(CodieToken::Caret)
            }
            '~' => {
                self.advance();
                Ok(CodieToken::Tilde)
            }

            // Delimiters
            '?' => {
                self.advance();
                Ok(CodieToken::Question)
            }
            ',' => {
                self.advance();
                Ok(CodieToken::Comma)
            }
            '(' => {
                self.advance();
                Ok(CodieToken::OpenParen)
            }
            ')' => {
                self.advance();
                Ok(CodieToken::CloseParen)
            }
            '{' => {
                self.advance();
                Ok(CodieToken::OpenBrace)
            }
            '}' => {
                self.advance();
                Ok(CodieToken::CloseBrace)
            }
            '[' => {
                self.advance();
                Ok(CodieToken::OpenBracket)
            }
            ']' => {
                self.advance();
                Ok(CodieToken::CloseBracket)
            }

            // Special prefixes
            '#' => {
                let hash = self.read_hash_ref()?;
                Ok(CodieToken::HashRef(hash))
            }
            '@' => {
                let source = self.read_source_ref();
                Ok(CodieToken::Identifier(format!("@{}", source)))
            }
            '$' => {
                let vault = self.read_vault_ref();
                Ok(CodieToken::Identifier(format!("${}", vault)))
            }

            // String literal
            '"' => {
                let s = self.read_string()?;
                Ok(CodieToken::StringLiteral(s))
            }

            // Numbers
            _ if c.is_ascii_digit() => {
                let n = self.read_number()?;
                Ok(CodieToken::NumberLiteral(n))
            }

            // Identifiers and keywords
            _ if c.is_alphabetic() || c == '_' => {
                let ident = self.read_identifier();

                // Check for keywords (all 40 keywords)
                if let Some(kw) = CodieKeyword::from_str(&ident) {
                    return Ok(CodieToken::Keyword(kw));
                }

                // Check for special loop modifiers (case insensitive)
                match ident.to_uppercase().as_str() {
                    "IN" => Ok(CodieToken::In),
                    "FOREVER" => Ok(CodieToken::Forever),
                    "TIMES" => Ok(CodieToken::Times),
                    _ => Ok(CodieToken::Identifier(ident)),
                }
            }

            // Unknown character
            _ => {
                let line = self.line;
                let col = self.column;
                self.advance(); // Skip unknown char
                Err(LexerError::UnexpectedChar(c, line, col))
            }
        }
    }

    /// Tokenize all input
    pub fn tokenize_all(&mut self) -> Result<Vec<CodieToken>, LexerError> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = matches!(token, CodieToken::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lexer = CodieLexer::new("pug LOGIN");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(
            tokens[0],
            CodieToken::Keyword(CodieKeyword::Pug)
        ));
        assert!(matches!(tokens[1], CodieToken::Identifier(ref s) if s == "LOGIN"));
    }

    #[test]
    fn test_tree_structure() {
        let mut lexer = CodieLexer::new("├── bark\n└── biz");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(tokens[0], CodieToken::TreeBranch));
        assert!(matches!(
            tokens[1],
            CodieToken::Keyword(CodieKeyword::Bark)
        ));
        assert!(matches!(tokens[2], CodieToken::Newline));
        assert!(matches!(tokens[3], CodieToken::TreeLastBranch));
    }

    #[test]
    fn test_arrows() {
        let mut lexer = CodieLexer::new("bark user ← @database");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(
            tokens[0],
            CodieToken::Keyword(CodieKeyword::Bark)
        ));
        assert!(matches!(tokens[1], CodieToken::Identifier(ref s) if s == "user"));
        assert!(matches!(tokens[2], CodieToken::BackArrow));
    }

    #[test]
    fn test_string_literal() {
        let mut lexer = CodieLexer::new("\"Hello, World!\"");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(tokens[0], CodieToken::StringLiteral(ref s) if s == "Hello, World!"));
    }

    #[test]
    fn test_numbers() {
        let mut lexer = CodieLexer::new("42 3.14 -5");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(tokens[0], CodieToken::NumberLiteral(n) if (n - 42.0).abs() < 0.001));
        assert!(matches!(tokens[1], CodieToken::NumberLiteral(n) if (n - 3.14).abs() < 0.001));
        assert!(matches!(tokens[2], CodieToken::NumberLiteral(n) if (n - -5.0).abs() < 0.001));
    }

    #[test]
    fn test_logic_gates() {
        let mut lexer = CodieLexer::new("and or not xor");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(tokens[0], CodieToken::Keyword(CodieKeyword::And)));
        assert!(matches!(tokens[1], CodieToken::Keyword(CodieKeyword::Or)));
        assert!(matches!(tokens[2], CodieToken::Keyword(CodieKeyword::Not)));
        assert!(matches!(tokens[3], CodieToken::Keyword(CodieKeyword::Xor)));
    }

    #[test]
    fn test_control_flow() {
        let mut lexer = CodieLexer::new("if else for while fork branch");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(tokens[0], CodieToken::Keyword(CodieKeyword::If)));
        assert!(matches!(tokens[1], CodieToken::Keyword(CodieKeyword::Else)));
        assert!(matches!(tokens[2], CodieToken::Keyword(CodieKeyword::For)));
        assert!(matches!(tokens[3], CodieToken::Keyword(CodieKeyword::While)));
        assert!(matches!(tokens[4], CodieToken::Keyword(CodieKeyword::Fork)));
        assert!(matches!(tokens[5], CodieToken::Keyword(CodieKeyword::Branch)));
    }

    #[test]
    fn test_booleans() {
        let mut lexer = CodieLexer::new("true false");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(tokens[0], CodieToken::Keyword(CodieKeyword::True)));
        assert!(matches!(tokens[1], CodieToken::Keyword(CodieKeyword::False)));
    }

    #[test]
    fn test_geometric() {
        let mut lexer = CodieLexer::new("mirror fold rotate");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(tokens[0], CodieToken::Keyword(CodieKeyword::Mirror)));
        assert!(matches!(tokens[1], CodieToken::Keyword(CodieKeyword::Fold)));
        assert!(matches!(tokens[2], CodieToken::Keyword(CodieKeyword::Rotate)));
    }

    #[test]
    fn test_dimensional() {
        let mut lexer = CodieLexer::new("dim axis plane space hyper");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(tokens[0], CodieToken::Keyword(CodieKeyword::Dim)));
        assert!(matches!(tokens[1], CodieToken::Keyword(CodieKeyword::Axis)));
        assert!(matches!(tokens[2], CodieToken::Keyword(CodieKeyword::Plane)));
        assert!(matches!(tokens[3], CodieToken::Keyword(CodieKeyword::Space)));
        assert!(matches!(tokens[4], CodieToken::Keyword(CodieKeyword::Hyper)));
    }

    #[test]
    fn test_comparison_operators() {
        let mut lexer = CodieLexer::new("== != <= >= < >");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(tokens[0], CodieToken::DoubleEquals));
        assert!(matches!(tokens[1], CodieToken::NotEquals));
        assert!(matches!(tokens[2], CodieToken::LessEquals));
        assert!(matches!(tokens[3], CodieToken::GreaterEquals));
        assert!(matches!(tokens[4], CodieToken::OpenAngle));
        assert!(matches!(tokens[5], CodieToken::CloseAngle));
    }

    #[test]
    fn test_logical_operators() {
        let mut lexer = CodieLexer::new("&& || ! ~");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(tokens[0], CodieToken::DoubleAnd));
        assert!(matches!(tokens[1], CodieToken::DoubleOr));
        assert!(matches!(tokens[2], CodieToken::Bang));
        assert!(matches!(tokens[3], CodieToken::Tilde));
    }

    #[test]
    fn test_dots() {
        let mut lexer = CodieLexer::new(". .. ...");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(tokens[0], CodieToken::Dot));
        assert!(matches!(tokens[1], CodieToken::DotDot));
        assert!(matches!(tokens[2], CodieToken::Ellipsis));
    }

    #[test]
    fn test_complex_expression() {
        let mut lexer = CodieLexer::new("if x > 0 and y < 10");
        let tokens = lexer.tokenize_all().unwrap();

        assert!(matches!(tokens[0], CodieToken::Keyword(CodieKeyword::If)));
        assert!(matches!(tokens[1], CodieToken::Identifier(ref s) if s == "x"));
        assert!(matches!(tokens[2], CodieToken::CloseAngle)); // >
        assert!(matches!(tokens[3], CodieToken::NumberLiteral(n) if n == 0.0));
        assert!(matches!(tokens[4], CodieToken::Keyword(CodieKeyword::And)));
        assert!(matches!(tokens[5], CodieToken::Identifier(ref s) if s == "y"));
        assert!(matches!(tokens[6], CodieToken::OpenAngle)); // <
        assert!(matches!(tokens[7], CodieToken::NumberLiteral(n) if n == 10.0));
    }
}
