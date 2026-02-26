//! CODIE Parser - Parses tokens into AST
//!
//! Steps 2.8, 2.9 from BUILD_STEPS.md

use crate::ast::{CodieAst, CodieLiteral, CodieType, SourceKind};
use crate::token::{CodieKeyword, CodieToken};
use thiserror::Error;

/// Parser errors
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: String },

    #[error("Unexpected end of input")]
    UnexpectedEof,

    #[error("Invalid syntax: {0}")]
    InvalidSyntax(String),

    #[error("Duplicate definition: {0}")]
    DuplicateDefinition(String),

    #[error("Lexer error: {0}")]
    LexerError(String),
}

/// The CODIE parser
pub struct CodieParser {
    tokens: Vec<CodieToken>,
    position: usize,
}

impl CodieParser {
    /// Create a new parser with the given tokens
    pub fn new(tokens: Vec<CodieToken>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// Peek at current token
    fn peek(&self) -> Option<&CodieToken> {
        self.tokens.get(self.position)
    }

    /// Peek at token at offset
    fn peek_ahead(&self, offset: usize) -> Option<&CodieToken> {
        self.tokens.get(self.position + offset)
    }

    /// Check if at end
    fn is_at_end(&self) -> bool {
        matches!(self.peek(), None | Some(CodieToken::Eof))
    }

    /// Advance to next token
    fn advance(&mut self) -> Option<&CodieToken> {
        if !self.is_at_end() {
            self.position += 1;
        }
        self.tokens.get(self.position - 1)
    }

    /// Skip newlines and indentation
    fn skip_whitespace(&mut self) {
        while let Some(token) = self.peek() {
            if matches!(token, CodieToken::Newline | CodieToken::Indent(_)) {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Skip tree structure tokens (|, ├──, └──)
    fn skip_tree_chars(&mut self) {
        while let Some(token) = self.peek() {
            if matches!(
                token,
                CodieToken::Pipe | CodieToken::TreeBranch | CodieToken::TreeLastBranch
            ) {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Parse the entire program
    pub fn parse(&mut self) -> Result<CodieAst, ParseError> {
        self.skip_whitespace();

        // Expect pug at the start
        if let Some(CodieToken::Keyword(CodieKeyword::Pug)) = self.peek() {
            self.parse_program()
        } else {
            // Parse as a list of statements
            let mut body = Vec::new();
            while !self.is_at_end() {
                self.skip_whitespace();
                self.skip_tree_chars();
                if self.is_at_end() {
                    break;
                }
                body.push(self.parse_statement()?);
            }
            Ok(CodieAst::Program {
                name: "anonymous".to_string(),
                hash: None,
                body,
            })
        }
    }

    /// Parse a program: pug NAME [#hash]
    fn parse_program(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume pug

        // Get program name
        let name = match self.peek() {
            Some(CodieToken::Identifier(s)) => {
                let n = s.clone();
                self.advance();
                n
            }
            _ => {
                return Err(ParseError::InvalidSyntax(
                    "Expected program name after pug".to_string(),
                ))
            }
        };

        // Optional hash
        let hash = if let Some(CodieToken::HashRef(h)) = self.peek() {
            let h = h.clone();
            self.advance();
            Some(h)
        } else {
            None
        };

        // Parse body
        let body = self.parse_block()?;

        Ok(CodieAst::Program { name, hash, body })
    }

    /// Parse a block of statements
    fn parse_block(&mut self) -> Result<Vec<CodieAst>, ParseError> {
        let mut statements = Vec::new();

        loop {
            self.skip_whitespace();
            self.skip_tree_chars();

            if self.is_at_end() {
                break;
            }

            // Check for end of block (would need proper indentation tracking)
            if let Some(token) = self.peek() {
                if matches!(
                    token,
                    CodieToken::Keyword(CodieKeyword::Pug) | CodieToken::Eof
                ) {
                    break;
                }
            }

            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    /// Parse a single statement
    fn parse_statement(&mut self) -> Result<CodieAst, ParseError> {
        self.skip_whitespace();
        self.skip_tree_chars();

        match self.peek() {
            Some(CodieToken::Keyword(kw)) => match kw {
                // Core 12 semantic keywords
                CodieKeyword::Pug => self.parse_program(),
                CodieKeyword::Bark => self.parse_bark(),
                CodieKeyword::Spin => self.parse_spin(),
                CodieKeyword::Cali => self.parse_cali(),
                CodieKeyword::Elf => self.parse_elf(),
                CodieKeyword::Turk => self.parse_turk(),
                CodieKeyword::Fence => self.parse_fence(),
                CodieKeyword::Pin => self.parse_pin(),
                CodieKeyword::Bone => self.parse_bone(),
                CodieKeyword::Blob => self.parse_blob(),
                CodieKeyword::Biz => self.parse_biz(),
                CodieKeyword::Anchor => self.parse_anchor(),

                // Control flow keywords
                CodieKeyword::If => self.parse_if(),
                CodieKeyword::For => self.parse_for(),
                CodieKeyword::While => self.parse_while(),
                CodieKeyword::Fork => self.parse_fork(),
                CodieKeyword::Branch => self.parse_branch(),
                CodieKeyword::Start => self.parse_start(),
                CodieKeyword::Break => {
                    self.advance();
                    Ok(CodieAst::Break)
                }
                CodieKeyword::Continue => {
                    self.advance();
                    Ok(CodieAst::Break) // TODO: Add Continue AST node
                }
                CodieKeyword::Return => self.parse_return(),

                // Meta/generation keywords
                CodieKeyword::Breed => self.parse_breed(),
                CodieKeyword::Speak => self.parse_speak(),
                CodieKeyword::Morph => self.parse_morph(),
                CodieKeyword::Cast => self.parse_cast(),

                // Logic gates, booleans, geometric, dimensional - used in expressions
                _ => self.parse_expression(),
            },
            Some(CodieToken::Question) => self.parse_conditional(),
            Some(CodieToken::Arrow) => self.parse_return(),
            Some(CodieToken::Identifier(_)) => self.parse_expression(),
            Some(CodieToken::Eof) => Ok(CodieAst::Empty),
            Some(other) => Err(ParseError::InvalidSyntax(format!(
                "Unexpected token: {}",
                other
            ))),
            None => Ok(CodieAst::Empty),
        }
    }

    /// Parse bark: bark target ← source
    fn parse_bark(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume bark

        // Get target name
        let target = match self.peek() {
            Some(CodieToken::Identifier(s)) => {
                let t = s.clone();
                self.advance();
                t
            }
            _ => {
                return Err(ParseError::InvalidSyntax(
                    "Expected target after bark".to_string(),
                ))
            }
        };

        // Expect ←
        if !matches!(self.peek(), Some(CodieToken::BackArrow)) {
            return Err(ParseError::InvalidSyntax(
                "Expected ← after bark target".to_string(),
            ));
        }
        self.advance();

        // Get source
        let source = match self.peek() {
            Some(CodieToken::Identifier(s)) => {
                let src = s.clone();
                self.advance();
                src
            }
            _ => {
                return Err(ParseError::InvalidSyntax(
                    "Expected source after ←".to_string(),
                ))
            }
        };

        let source_kind = SourceKind::from_path(&source);

        // TODO: Parse error handlers (? conditions)

        Ok(CodieAst::Fetch {
            target,
            source,
            source_kind,
            options: std::collections::HashMap::new(),
            error_handlers: Vec::new(),
        })
    }

    /// Parse spin loop
    fn parse_spin(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume spin

        match self.peek() {
            Some(CodieToken::Forever) => {
                self.advance();
                let body = self.parse_block()?;
                Ok(CodieAst::ForeverLoop { body })
            }
            Some(CodieToken::Keyword(CodieKeyword::While)) => {
                self.advance();
                let condition = Box::new(self.parse_expression()?);
                let body = self.parse_block()?;
                Ok(CodieAst::WhileLoop { condition, body })
            }
            Some(CodieToken::NumberLiteral(n)) => {
                let count = *n as u64;
                self.advance();
                if matches!(self.peek(), Some(CodieToken::Times)) {
                    self.advance();
                }
                let body = self.parse_block()?;
                Ok(CodieAst::TimesLoop { count, body })
            }
            Some(CodieToken::Identifier(iter)) => {
                let iterator = iter.clone();
                self.advance();

                if !matches!(self.peek(), Some(CodieToken::In)) {
                    return Err(ParseError::InvalidSyntax(
                        "Expected IN after iterator".to_string(),
                    ));
                }
                self.advance();

                let collection = Box::new(self.parse_expression()?);
                let body = self.parse_block()?;

                Ok(CodieAst::Loop {
                    iterator,
                    collection,
                    body,
                })
            }
            _ => Err(ParseError::InvalidSyntax(
                "Expected loop type after spin".to_string(),
            )),
        }
    }

    /// Parse cali (function definition)
    fn parse_cali(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume cali

        let name = match self.peek() {
            Some(CodieToken::Identifier(s)) => {
                let n = s.clone();
                self.advance();
                n
            }
            _ => {
                return Err(ParseError::InvalidSyntax(
                    "Expected function name after cali".to_string(),
                ))
            }
        };

        let body = self.parse_block()?;

        Ok(CodieAst::Function {
            name,
            params: Vec::new(), // TODO: parse parameters
            body,
            returns: None,
        })
    }

    /// Parse elf (variable binding)
    fn parse_elf(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume elf

        let name = match self.peek() {
            Some(CodieToken::Identifier(s)) => {
                let n = s.clone();
                self.advance();
                n
            }
            _ => {
                return Err(ParseError::InvalidSyntax(
                    "Expected variable name after elf".to_string(),
                ))
            }
        };

        // Optional type hint
        let type_hint = if matches!(self.peek(), Some(CodieToken::Colon)) {
            self.advance();
            if let Some(CodieToken::Identifier(t)) = self.peek() {
                let ty = CodieType::from_str(t);
                self.advance();
                Some(ty)
            } else {
                None
            }
        } else {
            None
        };

        // Expect ←
        if !matches!(self.peek(), Some(CodieToken::BackArrow)) {
            return Err(ParseError::InvalidSyntax(
                "Expected ← after variable name".to_string(),
            ));
        }
        self.advance();

        let value = Box::new(self.parse_expression()?);

        Ok(CodieAst::Variable {
            name,
            type_hint,
            value,
        })
    }

    /// Parse turk (incomplete marker)
    fn parse_turk(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume turk

        let hash = if let Some(CodieToken::OpenParen) = self.peek() {
            self.advance();
            let h = if let Some(CodieToken::HashRef(h)) = self.peek() {
                let hash = h.clone();
                self.advance();
                Some(hash)
            } else {
                None
            };
            if matches!(self.peek(), Some(CodieToken::CloseParen)) {
                self.advance();
            }
            h
        } else {
            None
        };

        Ok(CodieAst::Incomplete {
            hash,
            comment: None,
        })
    }

    /// Parse fence (constraints)
    fn parse_fence(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume fence
        let rules = self.parse_block()?;
        Ok(CodieAst::Constraint { rules })
    }

    /// Parse pin (specification)
    fn parse_pin(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume pin

        let name = if let Some(CodieToken::Identifier(s)) = self.peek() {
            let n = s.clone();
            self.advance();
            Some(n)
        } else {
            None
        };

        // Parse fields as block for now
        let block = self.parse_block()?;
        let fields: Vec<(String, CodieAst)> = block
            .into_iter()
            .filter_map(|node| {
                if let CodieAst::Variable { name, value, .. } = node {
                    Some((name, *value))
                } else {
                    None
                }
            })
            .collect();

        Ok(CodieAst::Specification { name, fields })
    }

    /// Parse bone (immutable rule)
    fn parse_bone(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume bone

        // Collect the rest of the line as the rule
        let mut rule = String::new();
        while let Some(token) = self.peek() {
            if matches!(token, CodieToken::Newline | CodieToken::Eof) {
                break;
            }
            rule.push_str(&format!("{} ", token));
            self.advance();
        }

        Ok(CodieAst::Immutable {
            rule: rule.trim().to_string(),
        })
    }

    /// Parse blob (flexible section)
    fn parse_blob(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume blob

        let name = if let Some(CodieToken::Identifier(s)) = self.peek() {
            let n = s.clone();
            self.advance();
            Some(n)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(CodieAst::Flexible { name, body })
    }

    /// Parse biz (goal/return)
    fn parse_biz(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume biz

        // Expect →
        if !matches!(self.peek(), Some(CodieToken::Arrow)) {
            return Err(ParseError::InvalidSyntax("Expected → after biz".to_string()));
        }
        self.advance();

        let expression = Box::new(self.parse_expression()?);

        // Optional anchor
        let anchor_hash = if let Some(CodieToken::HashRef(h)) = self.peek() {
            let hash = h.clone();
            self.advance();
            Some(hash)
        } else {
            None
        };

        Ok(CodieAst::Goal {
            expression,
            anchor_hash,
        })
    }

    /// Parse anchor (checkpoint)
    fn parse_anchor(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume anchor

        let hash = match self.peek() {
            Some(CodieToken::HashRef(h)) => {
                let hash = h.clone();
                self.advance();
                hash
            }
            _ => {
                return Err(ParseError::InvalidSyntax(
                    "Expected hash after anchor".to_string(),
                ))
            }
        };

        Ok(CodieAst::Checkpoint { hash })
    }

    /// Parse if statement
    fn parse_if(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume if
        let condition = Box::new(self.parse_expression()?);
        let body = self.parse_block()?;

        // Check for else
        self.skip_whitespace();
        if matches!(self.peek(), Some(CodieToken::Keyword(CodieKeyword::Else))) {
            self.advance();
            let _else_body = self.parse_block()?;
            // TODO: proper If/Else AST with else branch
        }

        Ok(CodieAst::Conditional {
            condition,
            then_branch: Box::new(CodieAst::Flexible {
                name: None,
                body,
            }),
        })
    }

    /// Parse for loop
    fn parse_for(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume for

        let iterator = match self.peek() {
            Some(CodieToken::Identifier(s)) => {
                let i = s.clone();
                self.advance();
                i
            }
            _ => {
                return Err(ParseError::InvalidSyntax(
                    "Expected iterator after for".to_string(),
                ))
            }
        };

        if !matches!(self.peek(), Some(CodieToken::In)) {
            return Err(ParseError::InvalidSyntax(
                "Expected IN after iterator".to_string(),
            ));
        }
        self.advance();

        let collection = Box::new(self.parse_expression()?);
        let body = self.parse_block()?;

        Ok(CodieAst::Loop {
            iterator,
            collection,
            body,
        })
    }

    /// Parse while loop
    fn parse_while(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume while
        let condition = Box::new(self.parse_expression()?);
        let body = self.parse_block()?;
        Ok(CodieAst::WhileLoop { condition, body })
    }

    /// Parse fork (parallel execution)
    fn parse_fork(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume fork
        let body = self.parse_block()?;
        // TODO: Add Fork AST node - for now use Flexible
        Ok(CodieAst::Flexible {
            name: Some("fork".to_string()),
            body,
        })
    }

    /// Parse branch (conditional split)
    fn parse_branch(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume branch
        let condition = Box::new(self.parse_expression()?);
        let body = self.parse_block()?;
        Ok(CodieAst::Conditional {
            condition,
            then_branch: Box::new(CodieAst::Flexible {
                name: None,
                body,
            }),
        })
    }

    /// Parse start (execution block)
    fn parse_start(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume start
        let name = if let Some(CodieToken::Identifier(s)) = self.peek() {
            let n = s.clone();
            self.advance();
            Some(n)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(CodieAst::Program {
            name: name.unwrap_or_else(|| "start".to_string()),
            hash: None,
            body,
        })
    }

    /// Parse breed (language identification/generation)
    fn parse_breed(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume breed

        let lang = match self.peek() {
            Some(CodieToken::Identifier(s)) => {
                let l = s.clone();
                self.advance();
                l
            }
            Some(CodieToken::StringLiteral(s)) => {
                let l = s.clone();
                self.advance();
                l
            }
            _ => "auto".to_string(),
        };

        let _body = self.parse_block()?;

        // breed creates a specification for code generation
        // TODO: Include body in generated specification
        Ok(CodieAst::Specification {
            name: Some(format!("breed:{}", lang)),
            fields: vec![("language".to_string(), CodieAst::Literal(CodieLiteral::String(lang)))],
        })
    }

    /// Parse speak (prompt generation)
    fn parse_speak(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume speak

        // Get the prompt/output expression
        let expression = Box::new(self.parse_expression()?);

        // speak is like biz but specifically for prompt generation
        Ok(CodieAst::Goal {
            expression,
            anchor_hash: None,
        })
    }

    /// Parse morph (transformation)
    fn parse_morph(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume morph

        let source = match self.peek() {
            Some(CodieToken::Identifier(s)) => {
                let src = s.clone();
                self.advance();
                src
            }
            _ => {
                return Err(ParseError::InvalidSyntax(
                    "Expected source after morph".to_string(),
                ))
            }
        };

        // Expect → for target
        if !matches!(self.peek(), Some(CodieToken::Arrow)) {
            return Err(ParseError::InvalidSyntax(
                "Expected → after morph source".to_string(),
            ));
        }
        self.advance();

        let target = Box::new(self.parse_expression()?);

        Ok(CodieAst::Variable {
            name: format!("morph:{}", source),
            type_hint: None,
            value: target,
        })
    }

    /// Parse cast (type/language casting)
    fn parse_cast(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume cast

        let value = Box::new(self.parse_expression()?);

        // Expect → for target type
        if !matches!(self.peek(), Some(CodieToken::Arrow)) {
            return Err(ParseError::InvalidSyntax(
                "Expected → after cast value".to_string(),
            ));
        }
        self.advance();

        let target_type = match self.peek() {
            Some(CodieToken::Identifier(s)) => {
                let t = s.clone();
                self.advance();
                t
            }
            _ => {
                return Err(ParseError::InvalidSyntax(
                    "Expected type after cast →".to_string(),
                ))
            }
        };

        Ok(CodieAst::Call {
            function: format!("cast:{}", target_type),
            args: vec![*value],
        })
    }

    /// Parse conditional: ? condition → action
    fn parse_conditional(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume ?

        let condition = Box::new(self.parse_expression()?);

        if !matches!(self.peek(), Some(CodieToken::Arrow)) {
            return Err(ParseError::InvalidSyntax(
                "Expected → after condition".to_string(),
            ));
        }
        self.advance();

        let then_branch = Box::new(self.parse_expression()?);

        Ok(CodieAst::Conditional {
            condition,
            then_branch,
        })
    }

    /// Parse return: → value
    fn parse_return(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume →
        let value = Box::new(self.parse_expression()?);
        Ok(CodieAst::Return { value })
    }

    /// Parse an expression
    fn parse_expression(&mut self) -> Result<CodieAst, ParseError> {
        match self.peek() {
            Some(CodieToken::StringLiteral(s)) => {
                let lit = CodieLiteral::String(s.clone());
                self.advance();
                Ok(CodieAst::Literal(lit))
            }
            Some(CodieToken::NumberLiteral(n)) => {
                let lit = CodieLiteral::Number(*n);
                self.advance();
                Ok(CodieAst::Literal(lit))
            }
            Some(CodieToken::Identifier(s)) => {
                let name = s.clone();
                self.advance();

                // Check for function call
                if matches!(self.peek(), Some(CodieToken::OpenParen)) {
                    self.advance();
                    let args = self.parse_arg_list()?;
                    if matches!(self.peek(), Some(CodieToken::CloseParen)) {
                        self.advance();
                    }
                    Ok(CodieAst::Call {
                        function: name,
                        args,
                    })
                } else {
                    Ok(CodieAst::Identifier(name))
                }
            }
            Some(CodieToken::OpenBrace) => self.parse_object(),
            Some(CodieToken::OpenBracket) => self.parse_list(),
            _ => Ok(CodieAst::Empty),
        }
    }

    /// Parse argument list
    fn parse_arg_list(&mut self) -> Result<Vec<CodieAst>, ParseError> {
        let mut args = Vec::new();

        loop {
            if matches!(
                self.peek(),
                Some(CodieToken::CloseParen) | Some(CodieToken::CloseBracket) | None
            ) {
                break;
            }

            args.push(self.parse_expression()?);

            if matches!(self.peek(), Some(CodieToken::Comma)) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(args)
    }

    /// Parse object literal
    fn parse_object(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume {

        let mut fields = Vec::new();

        loop {
            self.skip_whitespace();

            if matches!(self.peek(), Some(CodieToken::CloseBrace) | None) {
                break;
            }

            // Get key
            let key = match self.peek() {
                Some(CodieToken::Identifier(s)) => {
                    let k = s.clone();
                    self.advance();
                    k
                }
                _ => break,
            };

            // Expect :
            if matches!(self.peek(), Some(CodieToken::Colon)) {
                self.advance();
            }

            // Get value
            let value = self.parse_expression()?;
            fields.push((key, value));

            if matches!(self.peek(), Some(CodieToken::Comma)) {
                self.advance();
            }
        }

        if matches!(self.peek(), Some(CodieToken::CloseBrace)) {
            self.advance();
        }

        Ok(CodieAst::Object { fields })
    }

    /// Parse list literal
    fn parse_list(&mut self) -> Result<CodieAst, ParseError> {
        self.advance(); // consume [
        let items = self.parse_arg_list()?;
        if matches!(self.peek(), Some(CodieToken::CloseBracket)) {
            self.advance();
        }
        Ok(CodieAst::List { items })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::CodieLexer;

    fn parse(source: &str) -> Result<CodieAst, ParseError> {
        let mut lexer = CodieLexer::new(source);
        let tokens = lexer.tokenize_all().map_err(|e| ParseError::LexerError(e.to_string()))?;
        let mut parser = CodieParser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_parse_simple_program() {
        let ast = parse("pug TEST").unwrap();
        if let CodieAst::Program { name, .. } = ast {
            assert_eq!(name, "TEST");
        } else {
            panic!("Expected Program");
        }
    }

    #[test]
    fn test_parse_bark() {
        let ast = parse("bark user ← @database/users").unwrap();
        if let CodieAst::Program { body, .. } = ast {
            assert!(!body.is_empty());
        }
    }

    #[test]
    fn test_parse_elf() {
        let ast = parse("elf x ← 42").unwrap();
        if let CodieAst::Program { body, .. } = ast {
            if let Some(CodieAst::Variable { name, .. }) = body.first() {
                assert_eq!(name, "x");
            }
        }
    }
}
