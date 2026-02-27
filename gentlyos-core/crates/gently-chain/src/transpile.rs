//! CODIE → Move transpiler
//!
//! The CODIE executor IS Move. Every CODIE keyword maps 1:1 to a Move concept:
//!
//! ```text
//! CODIE Keyword    Move Concept
//! ─────────────    ────────────────────────────────
//! pug              module definition
//! bark             object read (Sui query)
//! treat/biz        function return (produce resource)
//! fence            constraint block (assert! statements)
//! bone             resource struct (linear type — can't copy, can't drop)
//! blob             struct with store+drop (flexible, non-linear)
//! spin             async transaction / loop construct
//! pin              entry function (PTB-callable)
//! cali             internal function
//! elf              let binding
//! anchor           event emission (on-chain checkpoint)
//! turk             TODO comment (incomplete marker)
//! ```
//!
//! CODIE doesn't need a general-purpose code generator.
//! CODIE IS Move's human-readable layer.
//! The blockchain IS the evaluator.

use anyhow::{Result, bail};
use std::fmt::Write;

use gently_codie::{CodieAst, CodieType, SourceKind};

// ── Public types ────────────────────────────────────────────────

/// A transpiled Move module ready for deployment
#[derive(Debug, Clone)]
pub struct MoveModule {
    /// Module name (from pug identifier)
    pub name: String,
    /// Generated Move source code
    pub source: String,
    /// Structs defined (bone/blob keywords)
    pub structs: Vec<MoveStruct>,
    /// Functions defined (pin/cali/biz keywords)
    pub functions: Vec<MoveFunction>,
    /// Dependencies (bark @external references)
    pub dependencies: Vec<String>,
}

/// A Move struct definition (from bone or blob)
#[derive(Debug, Clone)]
pub struct MoveStruct {
    pub name: String,
    /// bone = has key (linear resource), blob = has key + store + drop (flexible)
    pub abilities: Vec<MoveAbility>,
    pub fields: Vec<MoveField>,
}

/// Move abilities — the physics of the type system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveAbility {
    /// Stored on-chain, uniquely identified
    Key,
    /// Can be stored inside other structs
    Store,
    /// Can be copied (NEVER for bone, optional for blob)
    Copy,
    /// Can be implicitly destroyed (NEVER for bone)
    Drop,
}

impl std::fmt::Display for MoveAbility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveAbility::Key => write!(f, "key"),
            MoveAbility::Store => write!(f, "store"),
            MoveAbility::Copy => write!(f, "copy"),
            MoveAbility::Drop => write!(f, "drop"),
        }
    }
}

/// A field in a Move struct
#[derive(Debug, Clone)]
pub struct MoveField {
    pub name: String,
    pub type_name: String,
}

/// A Move function (from pin, cali, or biz)
#[derive(Debug, Clone)]
pub struct MoveFunction {
    pub name: String,
    pub visibility: MoveVisibility,
    pub params: Vec<MoveParam>,
    pub return_type: Option<String>,
    pub body: Vec<MoveStatement>,
}

/// Move function visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveVisibility {
    /// pin → public entry (PTB-callable, transaction entry point)
    Public,
    /// biz → public (goal/output endpoint, composable)
    PublicPackage,
    /// cali → internal (module-private)
    Internal,
}

/// A parameter in a Move function
#[derive(Debug, Clone)]
pub struct MoveParam {
    pub name: String,
    pub type_name: String,
    pub is_ref: bool,
    pub is_mut_ref: bool,
}

/// A statement in a Move function body
#[derive(Debug, Clone)]
pub enum MoveStatement {
    /// elf → let binding
    Let { name: String, type_name: Option<String>, value: String },
    /// bark → object read / borrow
    Borrow { name: String, source: String, is_mut: bool },
    /// treat → return expression
    Return(String),
    /// anchor → event::emit
    Emit { event_type: String, fields: Vec<(String, String)> },
    /// fence/bone → assert! (constraint enforcement)
    Assert { condition: String, error_code: u64 },
    /// Raw Move expression (for complex transpilations)
    Raw(String),
    /// spin → transfer_to_sender or share_object
    Transfer { object: String, recipient: String },
    /// turk → TODO comment
    Comment(String),
}

// ── Public API ──────────────────────────────────────────────────

/// Transpile a CODIE AST into Move source code.
///
/// This is the keystone function. CODIE compressed instructions become
/// Move modules that Sui executes natively.
pub fn codie_to_move(ast: &CodieAst) -> Result<MoveModule> {
    let mut transpiler = MoveTranspiler::new();
    transpiler.transpile(ast)
}

/// Transpile raw CODIE source directly to Move
pub fn source_to_move(codie_source: &str) -> Result<MoveModule> {
    let ast = gently_codie::parse(codie_source)
        .map_err(|e| anyhow::anyhow!("CODIE parse error: {}", e))?;
    codie_to_move(&ast)
}

/// Transpile compressed glyph string to Move (hydrate first, then transpile)
pub fn glyph_to_move(glyph: &str) -> Result<MoveModule> {
    let human = gently_codie::rehydrate(glyph);
    source_to_move(&human)
}

/// Transpile by hash lookup (the full pipeline: hash → hydrate → transpile)
pub fn hash_to_move(hash: &str) -> Result<MoveModule> {
    let source = gently_codie::hydrate_hash(hash)
        .ok_or_else(|| anyhow::anyhow!("CODIE hash {} not found in registry", hash))?;
    source_to_move(&source)
}

// ── Internal transpiler ─────────────────────────────────────────

struct MoveTranspiler {
    module_name: String,
    structs: Vec<MoveStruct>,
    functions: Vec<MoveFunction>,
    dependencies: Vec<String>,
    error_code_counter: u64,
    /// Constraints collected from fence blocks, injected into the next function
    pending_constraints: Vec<String>,
}

impl MoveTranspiler {
    fn new() -> Self {
        Self {
            module_name: String::new(),
            structs: Vec::new(),
            functions: Vec::new(),
            dependencies: Vec::new(),
            error_code_counter: 0,
            pending_constraints: Vec::new(),
        }
    }

    fn next_error_code(&mut self) -> u64 {
        self.error_code_counter += 1;
        self.error_code_counter
    }

    fn add_dependency(&mut self, dep: &str) {
        let dep = dep.to_string();
        if !self.dependencies.contains(&dep) {
            self.dependencies.push(dep);
        }
    }

    fn transpile(&mut self, ast: &CodieAst) -> Result<MoveModule> {
        self.walk_top_level(ast)?;

        if self.module_name.is_empty() {
            bail!("No pug (module definition) found in CODIE source");
        }

        let source = self.render_move();

        Ok(MoveModule {
            name: self.module_name.clone(),
            source,
            structs: self.structs.clone(),
            functions: self.functions.clone(),
            dependencies: self.dependencies.clone(),
        })
    }

    /// Walk the top-level AST node (must be a Program)
    fn walk_top_level(&mut self, ast: &CodieAst) -> Result<()> {
        match ast {
            CodieAst::Program { name, body, .. } => {
                self.module_name = to_snake_case(name);
                for node in body {
                    self.walk_module_level(node)?;
                }
            }
            // If not wrapped in a Program, treat the whole thing as one statement
            other => {
                self.module_name = "anonymous".to_string();
                self.walk_module_level(other)?;
            }
        }
        Ok(())
    }

    /// Walk a node at module scope. Here we build structs and functions.
    fn walk_module_level(&mut self, node: &CodieAst) -> Result<()> {
        match node {
            // bone → resource struct (linear type: has key + store, NO copy, NO drop)
            CodieAst::Immutable { rule } => {
                // At module level, bone defines a resource struct.
                // The rule string becomes the struct name.
                // Fields come from sub-patterns or we use defaults.
                let name = extract_struct_name(rule);
                self.structs.push(MoveStruct {
                    name: to_pascal_case(&name),
                    abilities: vec![MoveAbility::Key, MoveAbility::Store],
                    fields: vec![
                        MoveField { name: "id".to_string(), type_name: "UID".to_string() },
                        MoveField { name: "value".to_string(), type_name: "u64".to_string() },
                    ],
                });
            }

            // blob → flexible struct (has key + store + drop — can be destroyed)
            CodieAst::Flexible { name, body } => {
                let struct_name = name.as_deref().unwrap_or("Data");
                let fields = self.extract_fields_from_body(body);
                self.structs.push(MoveStruct {
                    name: to_pascal_case(struct_name),
                    abilities: vec![MoveAbility::Key, MoveAbility::Store, MoveAbility::Drop],
                    fields,
                });
            }

            // pin → public entry function (transaction entry point, PTB-callable)
            CodieAst::Specification { name, fields } => {
                let func_name = name.as_deref().unwrap_or("execute");
                let mut func = self.build_function_from_fields(func_name, fields, MoveVisibility::Public);
                // Inject pending constraints as assert! at function start
                self.inject_constraints(&mut func);
                self.functions.push(func);
            }

            // cali → internal function (module-private)
            CodieAst::Function { name, params, body, returns } => {
                let mut func = self.build_function_from_body(name, params, body, returns, MoveVisibility::Internal);
                self.inject_constraints(&mut func);
                self.functions.push(func);
            }

            // biz → public function (composable endpoint)
            CodieAst::Goal { expression, .. } => {
                let func = self.build_goal_function(expression);
                self.functions.push(func);
            }

            // fence → constraint block (rules become assert! statements)
            // Constraints are collected and injected into the next function.
            CodieAst::Constraint { rules } => {
                for rule in rules {
                    if let CodieAst::Immutable { rule: text } = rule {
                        self.pending_constraints.push(text.clone());
                    }
                }
            }

            // anchor → event struct definition at module level
            CodieAst::Checkpoint { hash } => {
                let event_name = format!("Event_{}", &hash[..hash.len().min(8)]);
                self.structs.push(MoveStruct {
                    name: event_name,
                    abilities: vec![MoveAbility::Copy, MoveAbility::Drop],
                    fields: vec![
                        MoveField { name: "hash".to_string(), type_name: "vector<u8>".to_string() },
                    ],
                });
            }

            // turk → TODO comment (incomplete, skip at module level)
            CodieAst::Incomplete { comment, .. } => {
                let msg = comment.as_deref().unwrap_or("incomplete");
                // Push a stub function with a TODO
                self.functions.push(MoveFunction {
                    name: "todo".to_string(),
                    visibility: MoveVisibility::Internal,
                    params: vec![],
                    return_type: None,
                    body: vec![MoveStatement::Comment(format!("TODO: {}", msg))],
                });
            }

            // Nested program → treated as a sub-module (flatten into current module)
            CodieAst::Program { body, .. } => {
                for child in body {
                    self.walk_module_level(child)?;
                }
            }

            // Variable at module level → const or struct field (treat as comment)
            CodieAst::Variable { name, type_hint, value } => {
                // At module scope, an elf binding hints at a struct field.
                // We collect these as pending for the next struct definition.
                // For now, emit a comment in the first function.
                let type_name = type_hint.as_ref()
                    .map(|t| codie_type_to_move(t))
                    .unwrap_or_else(|| infer_move_type(value));
                // If we have an existing struct being built, add to it;
                // otherwise this is a module-level constant (not native in Move,
                // so we add it as a comment).
                if let Some(last_struct) = self.structs.last_mut() {
                    last_struct.fields.push(MoveField {
                        name: to_snake_case(name),
                        type_name,
                    });
                }
            }

            // Fetch at module level → dependency declaration
            CodieAst::Fetch { source, source_kind, .. } => {
                if matches!(source_kind, SourceKind::Api | SourceKind::Database | SourceKind::Foam) {
                    self.add_dependency(source);
                }
            }

            // Loops at module level → generate an entry function that loops
            CodieAst::Loop { .. } |
            CodieAst::WhileLoop { .. } |
            CodieAst::ForeverLoop { .. } |
            CodieAst::TimesLoop { .. } => {
                // Move doesn't have top-level loops; wrap in a function
                let mut stmts = Vec::new();
                self.walk_function_body_nodes(std::slice::from_ref(node), &mut stmts);
                self.functions.push(MoveFunction {
                    name: "run".to_string(),
                    visibility: MoveVisibility::Public,
                    params: vec![MoveParam {
                        name: "ctx".to_string(),
                        type_name: "TxContext".to_string(),
                        is_ref: true,
                        is_mut_ref: true,
                    }],
                    return_type: None,
                    body: stmts,
                });
            }

            // Anything else at module level → skip silently
            _ => {}
        }

        Ok(())
    }

    /// Build a function from pin's field list
    fn build_function_from_fields(
        &mut self,
        name: &str,
        fields: &[(String, CodieAst)],
        vis: MoveVisibility,
    ) -> MoveFunction {
        let mut params = Vec::new();
        let mut body = Vec::new();

        // Entry functions always take TxContext
        if vis == MoveVisibility::Public {
            params.push(MoveParam {
                name: "ctx".to_string(),
                type_name: "TxContext".to_string(),
                is_ref: true,
                is_mut_ref: true,
            });
        }

        // Each field in the spec becomes either a param or a body statement
        for (key, value) in fields {
            match value {
                CodieAst::Identifier(type_name) => {
                    params.push(MoveParam {
                        name: to_snake_case(key),
                        type_name: type_name.clone(),
                        is_ref: false,
                        is_mut_ref: false,
                    });
                }
                CodieAst::Literal(lit) => {
                    body.push(MoveStatement::Let {
                        name: to_snake_case(key),
                        type_name: None,
                        value: format_literal(lit),
                    });
                }
                _ => {
                    body.push(MoveStatement::Comment(format!("{}: <complex>", key)));
                }
            }
        }

        MoveFunction {
            name: to_snake_case(name),
            visibility: vis,
            params,
            return_type: None,
            body,
        }
    }

    /// Build a function from cali's body
    fn build_function_from_body(
        &mut self,
        name: &str,
        params: &[(String, Option<CodieType>)],
        body: &[CodieAst],
        returns: &Option<Box<CodieAst>>,
        vis: MoveVisibility,
    ) -> MoveFunction {
        let mut move_params: Vec<MoveParam> = params.iter().map(|(pname, ptype)| {
            let type_name = ptype.as_ref()
                .map(|t| codie_type_to_move(t))
                .unwrap_or_else(|| "u64".to_string());
            MoveParam {
                name: to_snake_case(pname),
                type_name,
                is_ref: false,
                is_mut_ref: false,
            }
        }).collect();

        // Entry functions always take TxContext
        if vis == MoveVisibility::Public {
            move_params.push(MoveParam {
                name: "ctx".to_string(),
                type_name: "TxContext".to_string(),
                is_ref: true,
                is_mut_ref: true,
            });
        }

        let mut stmts = Vec::new();
        self.walk_function_body_nodes(body, &mut stmts);

        let return_type = returns.as_ref().map(|r| infer_move_type(r));

        MoveFunction {
            name: to_snake_case(name),
            visibility: vis,
            params: move_params,
            return_type,
            body: stmts,
        }
    }

    /// Build a public function from a biz/Goal node
    fn build_goal_function(&mut self, expression: &CodieAst) -> MoveFunction {
        let return_expr = ast_to_move_expr(expression);
        let return_type = Some(infer_move_type(expression));

        MoveFunction {
            name: "output".to_string(),
            visibility: MoveVisibility::PublicPackage,
            params: vec![],
            return_type,
            body: vec![MoveStatement::Return(return_expr)],
        }
    }

    /// Walk AST nodes that appear inside a function body, producing MoveStatements
    fn walk_function_body_nodes(&mut self, nodes: &[CodieAst], stmts: &mut Vec<MoveStatement>) {
        for node in nodes {
            match node {
                // elf → let binding
                CodieAst::Variable { name, type_hint, value } => {
                    let type_name = type_hint.as_ref().map(|t| codie_type_to_move(t));
                    stmts.push(MoveStatement::Let {
                        name: to_snake_case(name),
                        type_name,
                        value: ast_to_move_expr(value),
                    });
                }

                // bark → borrow / read
                CodieAst::Fetch { target, source, source_kind, .. } => {
                    if source.starts_with('@') || source.starts_with('$') {
                        let dep = source.trim_start_matches('@').trim_start_matches('$');
                        self.add_dependency(dep.to_string().split('/').next().unwrap_or(dep));
                    }
                    let is_mut = matches!(source_kind, SourceKind::Database | SourceKind::Storage);
                    stmts.push(MoveStatement::Borrow {
                        name: to_snake_case(target),
                        source: source.clone(),
                        is_mut,
                    });
                }

                // fence → assert! block
                CodieAst::Constraint { rules } => {
                    for rule in rules {
                        match rule {
                            CodieAst::Immutable { rule: text } => {
                                let code = self.next_error_code();
                                stmts.push(MoveStatement::Assert {
                                    condition: constraint_to_condition(text),
                                    error_code: code,
                                });
                            }
                            other => {
                                self.walk_function_body_nodes(std::slice::from_ref(other), stmts);
                            }
                        }
                    }
                }

                // bone inside function → assert! constraint
                CodieAst::Immutable { rule } => {
                    let code = self.next_error_code();
                    stmts.push(MoveStatement::Assert {
                        condition: constraint_to_condition(rule),
                        error_code: code,
                    });
                }

                // anchor → event::emit
                CodieAst::Checkpoint { hash } => {
                    let event_type = format!("Event_{}", &hash[..hash.len().min(8)]);
                    stmts.push(MoveStatement::Emit {
                        event_type,
                        fields: vec![("hash".to_string(), format!("b\"{}\"", hash))],
                    });
                }

                // biz/treat → return
                CodieAst::Goal { expression, .. } => {
                    stmts.push(MoveStatement::Return(ast_to_move_expr(expression)));
                }
                CodieAst::Return { value } => {
                    stmts.push(MoveStatement::Return(ast_to_move_expr(value)));
                }

                // spin → loop/while (Move has loop {} and while (cond) {})
                CodieAst::Loop { iterator, collection, body } => {
                    stmts.push(MoveStatement::Raw(format!(
                        "// spin {} IN {}",
                        iterator, ast_to_move_expr(collection)
                    )));
                    stmts.push(MoveStatement::Raw(format!(
                        "let mut i = 0;\nwhile (i < vector::length(&{})) {{",
                        ast_to_move_expr(collection)
                    )));
                    let mut inner = Vec::new();
                    self.walk_function_body_nodes(body, &mut inner);
                    for s in &inner {
                        stmts.push(s.clone());
                    }
                    stmts.push(MoveStatement::Raw("    i = i + 1;\n}".to_string()));
                }
                CodieAst::WhileLoop { condition, body } => {
                    stmts.push(MoveStatement::Raw(format!(
                        "while ({}) {{", ast_to_move_expr(condition)
                    )));
                    let mut inner = Vec::new();
                    self.walk_function_body_nodes(body, &mut inner);
                    for s in &inner {
                        stmts.push(s.clone());
                    }
                    stmts.push(MoveStatement::Raw("}".to_string()));
                }
                CodieAst::TimesLoop { count, body } => {
                    stmts.push(MoveStatement::Raw(format!(
                        "let mut i = 0;\nwhile (i < {}) {{", count
                    )));
                    let mut inner = Vec::new();
                    self.walk_function_body_nodes(body, &mut inner);
                    for s in &inner {
                        stmts.push(s.clone());
                    }
                    stmts.push(MoveStatement::Raw("    i = i + 1;\n}".to_string()));
                }
                CodieAst::ForeverLoop { body } => {
                    stmts.push(MoveStatement::Raw("loop {".to_string()));
                    let mut inner = Vec::new();
                    self.walk_function_body_nodes(body, &mut inner);
                    for s in &inner {
                        stmts.push(s.clone());
                    }
                    stmts.push(MoveStatement::Raw("}".to_string()));
                }

                // Conditional → if/assert
                CodieAst::Conditional { condition, then_branch } => {
                    stmts.push(MoveStatement::Raw(format!(
                        "if ({}) {{", ast_to_move_expr(condition)
                    )));
                    self.walk_function_body_nodes(std::slice::from_ref(then_branch.as_ref()), stmts);
                    stmts.push(MoveStatement::Raw("}".to_string()));
                }

                // Function call
                CodieAst::Call { function, args } => {
                    let arg_strs: Vec<String> = args.iter().map(ast_to_move_expr).collect();
                    stmts.push(MoveStatement::Raw(format!(
                        "{}({});", function, arg_strs.join(", ")
                    )));
                }

                // Source reference (@ or $)
                CodieAst::Source { kind, path, .. } => {
                    if kind.requires_ptc() {
                        stmts.push(MoveStatement::Comment(
                            format!("PTC REQUIRED: vault access ${}", path)
                        ));
                    } else {
                        stmts.push(MoveStatement::Raw(format!(
                            "// external: @{}", path
                        )));
                    }
                }

                // turk → TODO comment
                CodieAst::Incomplete { comment, hash } => {
                    let msg = comment.as_deref()
                        .or(hash.as_deref())
                        .unwrap_or("incomplete");
                    stmts.push(MoveStatement::Comment(format!("TODO: {}", msg)));
                }

                // blob inside function → struct instantiation
                CodieAst::Flexible { name, body } => {
                    let struct_name = name.as_deref().unwrap_or("data");
                    stmts.push(MoveStatement::Raw(format!(
                        "let {} = {} {{", to_snake_case(struct_name), to_pascal_case(struct_name)
                    )));
                    for child in body {
                        if let CodieAst::Variable { name: field_name, value, .. } = child {
                            stmts.push(MoveStatement::Raw(format!(
                                "    {}: {},", to_snake_case(field_name), ast_to_move_expr(value)
                            )));
                        }
                    }
                    stmts.push(MoveStatement::Raw("};".to_string()));
                }

                // pin inside function → entry point call (delegate)
                CodieAst::Specification { name, fields } => {
                    let fn_name = name.as_deref().unwrap_or("spec");
                    let args: Vec<String> = fields.iter()
                        .map(|(_, v)| ast_to_move_expr(v))
                        .collect();
                    stmts.push(MoveStatement::Raw(format!(
                        "{}({});", to_snake_case(fn_name), args.join(", ")
                    )));
                }

                // Nested program → inline its body
                CodieAst::Program { body, .. } => {
                    self.walk_function_body_nodes(body, stmts);
                }

                // Break
                CodieAst::Break => {
                    stmts.push(MoveStatement::Raw("break".to_string()));
                }

                // Literals, identifiers, etc → raw expressions
                CodieAst::Literal(_) | CodieAst::Identifier(_) => {
                    stmts.push(MoveStatement::Raw(ast_to_move_expr(node)));
                }

                // Object → struct instantiation
                CodieAst::Object { fields } => {
                    stmts.push(MoveStatement::Raw("{".to_string()));
                    for (k, v) in fields {
                        stmts.push(MoveStatement::Raw(format!(
                            "    {}: {},", k, ast_to_move_expr(v)
                        )));
                    }
                    stmts.push(MoveStatement::Raw("}".to_string()));
                }

                // Binary op → expression
                CodieAst::BinaryOp { left, op, right } => {
                    stmts.push(MoveStatement::Raw(format!(
                        "{} {} {}",
                        ast_to_move_expr(left), op, ast_to_move_expr(right)
                    )));
                }

                // Property access
                CodieAst::Property { object, property } => {
                    stmts.push(MoveStatement::Raw(format!(
                        "{}.{}", ast_to_move_expr(object), property
                    )));
                }

                // cali inside function body → nested function call or inline
                CodieAst::Function { name, body: fn_body, .. } => {
                    stmts.push(MoveStatement::Comment(format!("inline: {}", name)));
                    self.walk_function_body_nodes(fn_body, stmts);
                }

                // Everything else → skip
                CodieAst::Empty | CodieAst::Comment(_) | CodieAst::List { .. } => {}
            }
        }
    }

    /// Inject pending constraints as assert! statements at the start of a function
    fn inject_constraints(&mut self, func: &mut MoveFunction) {
        if self.pending_constraints.is_empty() {
            return;
        }
        let constraints: Vec<String> = self.pending_constraints.drain(..).collect();
        let mut asserts: Vec<MoveStatement> = constraints.into_iter()
            .map(|rule| {
                self.error_code_counter += 1;
                MoveStatement::Assert {
                    condition: constraint_to_condition(&rule),
                    error_code: self.error_code_counter,
                }
            })
            .collect();
        // Prepend asserts before existing body
        asserts.append(&mut func.body);
        func.body = asserts;
    }

    /// Extract struct fields from a blob's body
    fn extract_fields_from_body(&self, body: &[CodieAst]) -> Vec<MoveField> {
        let mut fields = vec![
            MoveField { name: "id".to_string(), type_name: "UID".to_string() },
        ];

        for node in body {
            match node {
                CodieAst::Variable { name, type_hint, value } => {
                    let type_name = type_hint.as_ref()
                        .map(|t| codie_type_to_move(t))
                        .unwrap_or_else(|| infer_move_type(value));
                    fields.push(MoveField {
                        name: to_snake_case(name),
                        type_name,
                    });
                }
                CodieAst::Identifier(name) => {
                    fields.push(MoveField {
                        name: to_snake_case(name),
                        type_name: "u64".to_string(),
                    });
                }
                _ => {}
            }
        }

        // If no explicit fields beyond id, add a default value field
        if fields.len() == 1 {
            fields.push(MoveField {
                name: "value".to_string(),
                type_name: "u64".to_string(),
            });
        }

        fields
    }

    /// Render the complete Move module source
    fn render_move(&self) -> String {
        let mut out = String::new();

        // Module header
        writeln!(out, "module {}::{}  {{", self.module_name, self.module_name).unwrap();
        writeln!(out, "    use sui::object::{{Self, UID}};").unwrap();
        writeln!(out, "    use sui::tx_context::TxContext;").unwrap();
        writeln!(out, "    use sui::transfer;").unwrap();

        // Event emission if any anchors used
        if self.structs.iter().any(|s| s.abilities.contains(&MoveAbility::Copy) && s.abilities.contains(&MoveAbility::Drop)) {
            writeln!(out, "    use sui::event;").unwrap();
        }

        // Dependencies from bark @external
        for dep in &self.dependencies {
            writeln!(out, "    use {};", dep).unwrap();
        }

        writeln!(out).unwrap();

        // Structs (bone = linear resource, blob = flexible)
        for s in &self.structs {
            let abilities: Vec<String> = s.abilities.iter().map(|a| a.to_string()).collect();
            writeln!(out, "    struct {} has {} {{", s.name, abilities.join(", ")).unwrap();
            for field in &s.fields {
                writeln!(out, "        {}: {},", field.name, field.type_name).unwrap();
            }
            writeln!(out, "    }}").unwrap();
            writeln!(out).unwrap();
        }

        // Functions
        for f in &self.functions {
            let vis = match f.visibility {
                MoveVisibility::Public => "public entry ",
                MoveVisibility::PublicPackage => "public ",
                MoveVisibility::Internal => "",
            };

            let params: Vec<String> = f.params.iter().map(|p| {
                if p.is_mut_ref {
                    format!("{}: &mut {}", p.name, p.type_name)
                } else if p.is_ref {
                    format!("{}: &{}", p.name, p.type_name)
                } else {
                    format!("{}: {}", p.name, p.type_name)
                }
            }).collect();

            let ret = f.return_type.as_ref()
                .map(|r| format!(": {}", r))
                .unwrap_or_default();

            writeln!(out, "    {}fun {}({}){} {{", vis, f.name, params.join(", "), ret).unwrap();

            for stmt in &f.body {
                match stmt {
                    MoveStatement::Let { name, type_name, value } => {
                        if let Some(t) = type_name {
                            writeln!(out, "        let {}: {} = {};", name, t, value).unwrap();
                        } else {
                            writeln!(out, "        let {} = {};", name, value).unwrap();
                        }
                    }
                    MoveStatement::Borrow { name, source, is_mut } => {
                        let borrow = if *is_mut { "&mut " } else { "&" };
                        writeln!(out, "        let {} = {}{};", name, borrow, source).unwrap();
                    }
                    MoveStatement::Return(val) => {
                        writeln!(out, "        {}", val).unwrap();
                    }
                    MoveStatement::Emit { event_type, fields } => {
                        if fields.is_empty() {
                            writeln!(out, "        event::emit({} {{}});", event_type).unwrap();
                        } else {
                            writeln!(out, "        event::emit({} {{", event_type).unwrap();
                            for (k, v) in fields {
                                writeln!(out, "            {}: {},", k, v).unwrap();
                            }
                            writeln!(out, "        }});").unwrap();
                        }
                    }
                    MoveStatement::Assert { condition, error_code } => {
                        writeln!(out, "        assert!({}, {});", condition, error_code).unwrap();
                    }
                    MoveStatement::Raw(code) => {
                        writeln!(out, "        {}", code).unwrap();
                    }
                    MoveStatement::Transfer { object, recipient } => {
                        writeln!(out, "        transfer::transfer({}, {});", object, recipient).unwrap();
                    }
                    MoveStatement::Comment(msg) => {
                        writeln!(out, "        // {}", msg).unwrap();
                    }
                }
            }

            writeln!(out, "    }}").unwrap();
            writeln!(out).unwrap();
        }

        writeln!(out, "}}").unwrap();
        out
    }
}

// ── Helpers ─────────────────────────────────────────────────────

fn to_pascal_case(s: &str) -> String {
    // First convert to snake_case to normalize, then capitalize each word
    let snake = to_snake_case(s);
    snake.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect()
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() && i > 0 {
            let prev_upper = chars[i - 1].is_uppercase();
            let next_lower = chars.get(i + 1).map(|n| n.is_lowercase()).unwrap_or(false);
            // Insert underscore before: camelCase boundary OR end of ALLCAPS before lowercase
            if !prev_upper || next_lower {
                result.push('_');
            }
        }
        result.push(c.to_ascii_lowercase());
    }
    result.replace(' ', "_").replace('-', "_")
}

/// Map CodieType to Move type string
fn codie_type_to_move(codie_type: &CodieType) -> String {
    match codie_type {
        CodieType::Text => "vector<u8>".to_string(),
        CodieType::Number => "u64".to_string(),
        CodieType::Bool => "bool".to_string(),
        CodieType::Uuid | CodieType::Hash => "vector<u8>".to_string(),
        CodieType::List(inner) => format!("vector<{}>", codie_type_to_move(inner)),
        CodieType::Map(_, _) => "vector<u8>".to_string(), // Move doesn't have native maps
        CodieType::Custom(name) => {
            match name.to_lowercase().as_str() {
                "address" | "addr" | "who" => "address".to_string(),
                "id" | "uid" | "object" => "ID".to_string(),
                "coin" | "token" | "money" => "Coin<SUI>".to_string(),
                "bytes" | "data" | "raw" => "vector<u8>".to_string(),
                _ => name.clone(),
            }
        }
        CodieType::Any => "vector<u8>".to_string(),
    }
}

/// Infer a Move type from an AST expression
fn infer_move_type(ast: &CodieAst) -> String {
    match ast {
        CodieAst::Literal(gently_codie::ast::CodieLiteral::Number(_)) => "u64".to_string(),
        CodieAst::Literal(gently_codie::ast::CodieLiteral::Bool(_)) => "bool".to_string(),
        CodieAst::Literal(gently_codie::ast::CodieLiteral::String(_)) => "vector<u8>".to_string(),
        CodieAst::Literal(gently_codie::ast::CodieLiteral::Null) => "vector<u8>".to_string(),
        CodieAst::Object { .. } => "vector<u8>".to_string(), // serialized
        CodieAst::List { .. } => "vector<u8>".to_string(),
        CodieAst::Identifier(name) => {
            // Try to guess from name conventions
            if name.contains("addr") || name.contains("sender") || name.contains("recipient") {
                "address".to_string()
            } else if name.contains("id") {
                "ID".to_string()
            } else if name.contains("amount") || name.contains("count") || name.contains("value") {
                "u64".to_string()
            } else if name.contains("flag") || name.contains("is_") || name.contains("has_") {
                "bool".to_string()
            } else {
                "u64".to_string()
            }
        }
        _ => "u64".to_string(),
    }
}

/// Convert a CodieAst node to a Move expression string
fn ast_to_move_expr(ast: &CodieAst) -> String {
    match ast {
        CodieAst::Literal(gently_codie::ast::CodieLiteral::Number(n)) => {
            format!("{}", *n as u64)
        }
        CodieAst::Literal(gently_codie::ast::CodieLiteral::Bool(b)) => {
            format!("{}", b)
        }
        CodieAst::Literal(gently_codie::ast::CodieLiteral::String(s)) => {
            format!("b\"{}\"", s)
        }
        CodieAst::Literal(gently_codie::ast::CodieLiteral::Null) => {
            "vector::empty<u8>()".to_string()
        }
        CodieAst::Identifier(name) => to_snake_case(name),
        CodieAst::Call { function, args } => {
            let arg_strs: Vec<String> = args.iter().map(ast_to_move_expr).collect();
            format!("{}({})", function, arg_strs.join(", "))
        }
        CodieAst::BinaryOp { left, op, right } => {
            format!("({} {} {})", ast_to_move_expr(left), op, ast_to_move_expr(right))
        }
        CodieAst::Property { object, property } => {
            format!("{}.{}", ast_to_move_expr(object), property)
        }
        CodieAst::Object { fields } => {
            if fields.is_empty() {
                "{}".to_string()
            } else {
                let field_strs: Vec<String> = fields.iter()
                    .map(|(k, v)| format!("{}: {}", k, ast_to_move_expr(v)))
                    .collect();
                format!("{{ {} }}", field_strs.join(", "))
            }
        }
        CodieAst::List { items } => {
            let item_strs: Vec<String> = items.iter().map(ast_to_move_expr).collect();
            format!("vector[{}]", item_strs.join(", "))
        }
        _ => "/* <expr> */".to_string(),
    }
}

/// Format a CodieLiteral for Move
fn format_literal(lit: &gently_codie::ast::CodieLiteral) -> String {
    match lit {
        gently_codie::ast::CodieLiteral::Number(n) => format!("{}", *n as u64),
        gently_codie::ast::CodieLiteral::Bool(b) => format!("{}", b),
        gently_codie::ast::CodieLiteral::String(s) => format!("b\"{}\"", s),
        gently_codie::ast::CodieLiteral::Null => "vector::empty<u8>()".to_string(),
    }
}

/// Extract a struct name from a bone rule string.
/// "NOT: store passwords plain" → "StorePasswordsPlain" (from the constraint text)
/// "AuthToken" → "AuthToken"
fn extract_struct_name(rule: &str) -> String {
    let cleaned = rule
        .strip_prefix("NOT:")
        .or_else(|| rule.strip_prefix("not:"))
        .or_else(|| rule.strip_prefix("NOT "))
        .unwrap_or(rule)
        .trim();

    // If it looks like a name (single word, starts with uppercase), use it
    if cleaned.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        && !cleaned.contains(' ')
    {
        cleaned.to_string()
    } else {
        // Otherwise, generate a name from the first meaningful words
        let words: Vec<&str> = cleaned.split_whitespace().take(3).collect();
        if words.is_empty() {
            "Resource".to_string()
        } else {
            words.join("_")
        }
    }
}

/// Convert a bone constraint rule to a Move assert condition.
/// "NOT: store passwords plain" → "true /* constraint: store passwords plain */"
/// "amount > 0" → "amount > 0"
fn constraint_to_condition(rule: &str) -> String {
    let cleaned = rule
        .strip_prefix("NOT:")
        .or_else(|| rule.strip_prefix("not:"))
        .or_else(|| rule.strip_prefix("NOT "))
        .map(|s| s.trim());

    if let Some(negated) = cleaned {
        // It's a NOT constraint — the assertion is that this doesn't happen
        format!("true /* NOT: {} */", negated)
    } else if rule.contains('>') || rule.contains('<') || rule.contains("==") || rule.contains("!=") {
        // It's already an expression-like constraint
        rule.to_string()
    } else {
        // Generic constraint — wrap as a comment in an assert
        format!("true /* constraint: {} */", rule)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("auth_token"), "AuthToken");
        assert_eq!(to_pascal_case("user"), "User");
        assert_eq!(to_pascal_case("reasoning step"), "ReasoningStep");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("AuthToken"), "auth_token");
        assert_eq!(to_snake_case("create_step"), "create_step");
        assert_eq!(to_snake_case("simple"), "simple");
    }

    #[test]
    fn test_codie_type_mapping() {
        assert_eq!(codie_type_to_move(&CodieType::Text), "vector<u8>");
        assert_eq!(codie_type_to_move(&CodieType::Number), "u64");
        assert_eq!(codie_type_to_move(&CodieType::Bool), "bool");
        assert_eq!(codie_type_to_move(&CodieType::Custom("address".to_string())), "address");
    }

    #[test]
    fn test_move_ability_display() {
        assert_eq!(MoveAbility::Key.to_string(), "key");
        assert_eq!(MoveAbility::Store.to_string(), "store");
        assert_eq!(MoveAbility::Copy.to_string(), "copy");
        assert_eq!(MoveAbility::Drop.to_string(), "drop");
    }

    #[test]
    fn test_bone_is_linear() {
        // bone = resource = no copy, no drop
        let bone_struct = MoveStruct {
            name: "AuthToken".to_string(),
            abilities: vec![MoveAbility::Key, MoveAbility::Store],
            fields: vec![
                MoveField { name: "id".to_string(), type_name: "UID".to_string() },
                MoveField { name: "owner".to_string(), type_name: "address".to_string() },
            ],
        };
        assert!(!bone_struct.abilities.contains(&MoveAbility::Copy));
        assert!(!bone_struct.abilities.contains(&MoveAbility::Drop));
    }

    #[test]
    fn test_blob_is_flexible() {
        // blob = flexible = has drop (can be destroyed)
        let blob_struct = MoveStruct {
            name: "TempData".to_string(),
            abilities: vec![MoveAbility::Key, MoveAbility::Store, MoveAbility::Drop],
            fields: vec![
                MoveField { name: "id".to_string(), type_name: "UID".to_string() },
                MoveField { name: "data".to_string(), type_name: "vector<u8>".to_string() },
            ],
        };
        assert!(blob_struct.abilities.contains(&MoveAbility::Drop));
        assert!(!blob_struct.abilities.contains(&MoveAbility::Copy));
    }

    #[test]
    fn test_extract_struct_name() {
        assert_eq!(extract_struct_name("AuthToken"), "AuthToken");
        assert_eq!(extract_struct_name("NOT: store passwords plain"), "store_passwords_plain");
        assert_eq!(extract_struct_name(""), "Resource");
    }

    #[test]
    fn test_constraint_to_condition() {
        assert_eq!(
            constraint_to_condition("NOT: store passwords plain"),
            "true /* NOT: store passwords plain */"
        );
        assert_eq!(
            constraint_to_condition("amount > 0"),
            "amount > 0"
        );
    }

    #[test]
    fn test_transpile_simple_program() {
        let ast = CodieAst::Program {
            name: "LOGIN".to_string(),
            hash: None,
            body: vec![
                CodieAst::Immutable { rule: "AuthToken".to_string() },
                CodieAst::Function {
                    name: "validate".to_string(),
                    params: vec![("user".to_string(), Some(CodieType::Text))],
                    body: vec![
                        CodieAst::Return {
                            value: Box::new(CodieAst::Literal(
                                gently_codie::ast::CodieLiteral::Bool(true)
                            )),
                        },
                    ],
                    returns: Some(Box::new(CodieAst::Literal(
                        gently_codie::ast::CodieLiteral::Bool(true)
                    ))),
                },
            ],
        };

        let module = codie_to_move(&ast).unwrap();
        assert_eq!(module.name, "login");
        assert_eq!(module.structs.len(), 1);
        assert_eq!(module.structs[0].name, "AuthToken");
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "validate");
        assert!(module.source.contains("module login::login"));
        assert!(module.source.contains("struct AuthToken has key, store"));
        assert!(module.source.contains("fun validate"));
    }

    #[test]
    fn test_transpile_with_constraints() {
        let ast = CodieAst::Program {
            name: "SECURE".to_string(),
            hash: None,
            body: vec![
                CodieAst::Constraint {
                    rules: vec![
                        CodieAst::Immutable { rule: "NOT: unlimited attempts".to_string() },
                        CodieAst::Immutable { rule: "amount > 0".to_string() },
                    ],
                },
                CodieAst::Specification {
                    name: Some("mint".to_string()),
                    fields: vec![
                        ("amount".to_string(), CodieAst::Identifier("u64".to_string())),
                    ],
                },
            ],
        };

        let module = codie_to_move(&ast).unwrap();
        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];
        assert_eq!(func.name, "mint");
        // Should have asserts injected from fence
        let has_assert = func.body.iter().any(|s| matches!(s, MoveStatement::Assert { .. }));
        assert!(has_assert, "fence constraints should inject assert! statements");
    }

    #[test]
    fn test_transpile_blob_struct() {
        let ast = CodieAst::Program {
            name: "DATA".to_string(),
            hash: None,
            body: vec![
                CodieAst::Flexible {
                    name: Some("TempCache".to_string()),
                    body: vec![
                        CodieAst::Variable {
                            name: "ttl".to_string(),
                            type_hint: Some(CodieType::Number),
                            value: Box::new(CodieAst::Literal(
                                gently_codie::ast::CodieLiteral::Number(60.0)
                            )),
                        },
                    ],
                },
            ],
        };

        let module = codie_to_move(&ast).unwrap();
        assert_eq!(module.structs.len(), 1);
        let s = &module.structs[0];
        assert_eq!(s.name, "TempCache");
        assert!(s.abilities.contains(&MoveAbility::Drop));
        assert!(s.fields.iter().any(|f| f.name == "ttl" && f.type_name == "u64"));
    }

    #[test]
    fn test_source_to_move_simple() {
        // This tests the full pipeline: CODIE source string → parse → transpile
        let result = source_to_move("pug TEST");
        assert!(result.is_ok());
        let module = result.unwrap();
        assert_eq!(module.name, "test");
    }

    #[test]
    fn test_render_produces_valid_structure() {
        let ast = CodieAst::Program {
            name: "EXAMPLE".to_string(),
            hash: None,
            body: vec![
                CodieAst::Immutable { rule: "Token".to_string() },
                CodieAst::Specification {
                    name: Some("create".to_string()),
                    fields: vec![],
                },
            ],
        };

        let module = codie_to_move(&ast).unwrap();
        assert!(module.source.contains("module example::example"));
        assert!(module.source.contains("use sui::object::{Self, UID}"));
        assert!(module.source.contains("use sui::tx_context::TxContext"));
        assert!(module.source.contains("struct Token has key, store"));
        assert!(module.source.contains("public entry fun create"));
    }
}
