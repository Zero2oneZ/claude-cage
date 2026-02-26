//! CODIE Token definitions
//!
//! CODIE: 12 core semantic keywords + logic gates + control flow + geometric operators
//!
//! Categories:
//! - Core 12: pug, bark, spin, cali, elf, turk, fence, pin, bone, blob, biz, anchor
//! - Logic Gates: and, or, not, xor, nand, nor
//! - Control Flow: if, else, start, for, fork, branch, while, break, continue, return
//! - Boolean: true, false
//! - Geometric: mirror, fold, rotate, translate, scale
//! - Dimensional: dim, axis, plane, space, hyper

use serde::{Deserialize, Serialize};

/// The 12 core CODIE semantic keywords
pub const CODIE_CORE_KEYWORDS: [&str; 12] = [
    "pug",    // Entry point
    "bark",   // Fetch/get/pull
    "spin",   // Loop
    "cali",   // Function definition
    "elf",    // Variable binding
    "turk",   // Incomplete marker
    "fence",  // Constraints
    "pin",    // Exact specification
    "bone",   // Immutable
    "blob",   // Flexible
    "biz",    // Goal/output
    "anchor", // Save state/checkpoint
];

/// Logic gate keywords
pub const CODIE_LOGIC_GATES: [&str; 6] = [
    "and",  // Logical AND
    "or",   // Logical OR
    "not",  // Logical NOT / negation
    "xor",  // Exclusive OR
    "nand", // NOT AND
    "nor",  // NOT OR
];

/// Control flow keywords
pub const CODIE_CONTROL_FLOW: [&str; 10] = [
    "if",       // Conditional
    "else",     // Alternative branch
    "start",    // Begin execution
    "for",      // Counted iteration
    "fork",     // Parallel execution
    "branch",   // Conditional branch
    "while",    // Conditional loop
    "break",    // Exit loop
    "continue", // Skip to next iteration
    "return",   // Return value
];

/// Boolean literals
pub const CODIE_BOOLEANS: [&str; 2] = ["true", "false"];

/// Geometric transformation keywords
pub const CODIE_GEOMETRIC: [&str; 5] = [
    "mirror",    // Reflection / opposite / negative
    "fold",      // Dimensional folding
    "rotate",    // Rotation transform
    "translate", // Position shift
    "scale",     // Size transform
];

/// Dimensional reference keywords
pub const CODIE_DIMENSIONAL: [&str; 5] = [
    "dim",   // Dimension reference
    "axis",  // Axis reference (x, y, z, w, ...)
    "plane", // 2D plane in space
    "space", // 3D space reference
    "hyper", // Hyperdimensional (4D+)
];

/// Meta/generation keywords
pub const CODIE_META: [&str; 4] = [
    "breed",  // Identify/generate programming languages (polymorphism)
    "speak",  // Generate prompts/output
    "morph",  // Transform between representations
    "cast",   // Type/language casting
];

/// CODIE keyword enum - ALL keywords
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodieKeyword {
    // ===== Core 12 Semantic Keywords =====
    /// Entry point - "Begin here"
    Pug,
    /// Fetch/get/pull - "Go get this"
    Bark,
    /// Loop/repeat - "Keep doing"
    Spin,
    /// Define function - "Here's how"
    Cali,
    /// Bind variable - "Call this X"
    Elf,
    /// Incomplete marker - "Needs work"
    Turk,
    /// Constraints - "Not this"
    Fence,
    /// Exact specification - "Exactly this"
    Pin,
    /// Immutable - "Can't change"
    Bone,
    /// Flexible - "Whatever works"
    Blob,
    /// Goal/output - "End up here"
    Biz,
    /// Save state - "Remember this"
    Anchor,

    // ===== Logic Gates =====
    /// Logical AND - both must be true
    And,
    /// Logical OR - either can be true
    Or,
    /// Logical NOT - negation/opposite
    Not,
    /// Exclusive OR - one but not both
    Xor,
    /// NOT AND - false if both true
    Nand,
    /// NOT OR - true only if both false
    Nor,

    // ===== Control Flow =====
    /// Conditional execution
    If,
    /// Alternative branch
    Else,
    /// Begin execution block
    Start,
    /// Counted iteration
    For,
    /// Parallel execution / split
    Fork,
    /// Conditional branch point
    Branch,
    /// Conditional loop
    While,
    /// Exit loop
    Break,
    /// Skip to next iteration
    Continue,
    /// Return value from function
    Return,

    // ===== Boolean Literals =====
    /// Boolean true
    True,
    /// Boolean false
    False,

    // ===== Geometric Transformations =====
    /// Mirror/reflect - opposite or negative
    Mirror,
    /// Fold dimension
    Fold,
    /// Rotate in space
    Rotate,
    /// Translate position
    Translate,
    /// Scale size
    Scale,

    // ===== Dimensional References =====
    /// Dimension reference (dim[0], dim[1], ...)
    Dim,
    /// Axis reference (axis.x, axis.y, ...)
    Axis,
    /// 2D plane in higher space
    Plane,
    /// 3D space reference
    Space,
    /// Hyperdimensional (4D+)
    Hyper,

    // ===== Meta/Generation =====
    /// Identify/generate programming languages (polymorphism)
    Breed,
    /// Generate prompts/output
    Speak,
    /// Transform between representations
    Morph,
    /// Type/language casting
    Cast,
}

impl CodieKeyword {
    /// Parse a keyword from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            // Core 12
            "pug" => Some(Self::Pug),
            "bark" => Some(Self::Bark),
            "spin" => Some(Self::Spin),
            "cali" => Some(Self::Cali),
            "elf" => Some(Self::Elf),
            "turk" => Some(Self::Turk),
            "fence" => Some(Self::Fence),
            "pin" => Some(Self::Pin),
            "bone" => Some(Self::Bone),
            "blob" => Some(Self::Blob),
            "biz" => Some(Self::Biz),
            "anchor" => Some(Self::Anchor),

            // Logic gates
            "and" => Some(Self::And),
            "or" => Some(Self::Or),
            "not" => Some(Self::Not),
            "xor" => Some(Self::Xor),
            "nand" => Some(Self::Nand),
            "nor" => Some(Self::Nor),

            // Control flow
            "if" => Some(Self::If),
            "else" => Some(Self::Else),
            "start" => Some(Self::Start),
            "for" => Some(Self::For),
            "fork" => Some(Self::Fork),
            "branch" => Some(Self::Branch),
            "while" => Some(Self::While),
            "break" => Some(Self::Break),
            "continue" => Some(Self::Continue),
            "return" => Some(Self::Return),

            // Booleans
            "true" => Some(Self::True),
            "false" => Some(Self::False),

            // Geometric
            "mirror" => Some(Self::Mirror),
            "fold" => Some(Self::Fold),
            "rotate" => Some(Self::Rotate),
            "translate" => Some(Self::Translate),
            "scale" => Some(Self::Scale),

            // Dimensional
            "dim" => Some(Self::Dim),
            "axis" => Some(Self::Axis),
            "plane" => Some(Self::Plane),
            "space" => Some(Self::Space),
            "hyper" => Some(Self::Hyper),

            // Meta/generation
            "breed" => Some(Self::Breed),
            "speak" => Some(Self::Speak),
            "morph" => Some(Self::Morph),
            "cast" => Some(Self::Cast),

            _ => None,
        }
    }

    /// Get the keyword as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            // Core 12
            Self::Pug => "pug",
            Self::Bark => "bark",
            Self::Spin => "spin",
            Self::Cali => "cali",
            Self::Elf => "elf",
            Self::Turk => "turk",
            Self::Fence => "fence",
            Self::Pin => "pin",
            Self::Bone => "bone",
            Self::Blob => "blob",
            Self::Biz => "biz",
            Self::Anchor => "anchor",

            // Logic gates
            Self::And => "and",
            Self::Or => "or",
            Self::Not => "not",
            Self::Xor => "xor",
            Self::Nand => "nand",
            Self::Nor => "nor",

            // Control flow
            Self::If => "if",
            Self::Else => "else",
            Self::Start => "start",
            Self::For => "for",
            Self::Fork => "fork",
            Self::Branch => "branch",
            Self::While => "while",
            Self::Break => "break",
            Self::Continue => "continue",
            Self::Return => "return",

            // Booleans
            Self::True => "true",
            Self::False => "false",

            // Geometric
            Self::Mirror => "mirror",
            Self::Fold => "fold",
            Self::Rotate => "rotate",
            Self::Translate => "translate",
            Self::Scale => "scale",

            // Dimensional
            Self::Dim => "dim",
            Self::Axis => "axis",
            Self::Plane => "plane",
            Self::Space => "space",
            Self::Hyper => "hyper",

            // Meta/generation
            Self::Breed => "breed",
            Self::Speak => "speak",
            Self::Morph => "morph",
            Self::Cast => "cast",
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            // Core 12
            Self::Pug => "Entry point - Begin here",
            Self::Bark => "Fetch/get/pull - Go get this",
            Self::Spin => "Loop/repeat - Keep doing",
            Self::Cali => "Define function - Here's how",
            Self::Elf => "Bind variable - Call this X",
            Self::Turk => "Incomplete marker - Needs work",
            Self::Fence => "Constraints - Not this",
            Self::Pin => "Exact specification - Exactly this",
            Self::Bone => "Immutable - Can't change",
            Self::Blob => "Flexible - Whatever works",
            Self::Biz => "Goal/output - End up here",
            Self::Anchor => "Save state - Remember this",

            // Logic gates
            Self::And => "Logical AND - Both must be true",
            Self::Or => "Logical OR - Either can be true",
            Self::Not => "Logical NOT - Negation/opposite",
            Self::Xor => "Exclusive OR - One but not both",
            Self::Nand => "NOT AND - False if both true",
            Self::Nor => "NOT OR - True only if both false",

            // Control flow
            Self::If => "Conditional - Execute if true",
            Self::Else => "Alternative - Execute if false",
            Self::Start => "Begin - Start execution block",
            Self::For => "Counted loop - Iterate N times",
            Self::Fork => "Parallel - Split execution",
            Self::Branch => "Branch point - Conditional split",
            Self::While => "While loop - Repeat while true",
            Self::Break => "Break - Exit loop early",
            Self::Continue => "Continue - Skip to next iteration",
            Self::Return => "Return - Exit with value",

            // Booleans
            Self::True => "Boolean true",
            Self::False => "Boolean false",

            // Geometric
            Self::Mirror => "Mirror - Reflection/opposite/negative",
            Self::Fold => "Fold - Collapse dimension",
            Self::Rotate => "Rotate - Angular transformation",
            Self::Translate => "Translate - Position shift",
            Self::Scale => "Scale - Size transformation",

            // Dimensional
            Self::Dim => "Dimension - Reference by index",
            Self::Axis => "Axis - Named direction (x,y,z,w)",
            Self::Plane => "Plane - 2D surface in space",
            Self::Space => "Space - 3D volume reference",
            Self::Hyper => "Hyper - 4D+ dimensional reference",

            // Meta/generation
            Self::Breed => "Breed - Identify/generate programming languages",
            Self::Speak => "Speak - Generate prompts/output",
            Self::Morph => "Morph - Transform between representations",
            Self::Cast => "Cast - Type/language casting",
        }
    }

    /// Check if this is a core semantic keyword
    pub fn is_core(&self) -> bool {
        matches!(
            self,
            Self::Pug
                | Self::Bark
                | Self::Spin
                | Self::Cali
                | Self::Elf
                | Self::Turk
                | Self::Fence
                | Self::Pin
                | Self::Bone
                | Self::Blob
                | Self::Biz
                | Self::Anchor
        )
    }

    /// Check if this is a logic gate
    pub fn is_logic_gate(&self) -> bool {
        matches!(
            self,
            Self::And | Self::Or | Self::Not | Self::Xor | Self::Nand | Self::Nor
        )
    }

    /// Check if this is a control flow keyword
    pub fn is_control_flow(&self) -> bool {
        matches!(
            self,
            Self::If
                | Self::Else
                | Self::Start
                | Self::For
                | Self::Fork
                | Self::Branch
                | Self::While
                | Self::Break
                | Self::Continue
                | Self::Return
        )
    }

    /// Check if this is a geometric transformation
    pub fn is_geometric(&self) -> bool {
        matches!(
            self,
            Self::Mirror | Self::Fold | Self::Rotate | Self::Translate | Self::Scale
        )
    }

    /// Check if this is a dimensional reference
    pub fn is_dimensional(&self) -> bool {
        matches!(
            self,
            Self::Dim | Self::Axis | Self::Plane | Self::Space | Self::Hyper
        )
    }

    /// Check if this is a boolean literal
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::True | Self::False)
    }

    /// Check if this is a meta/generation keyword
    pub fn is_meta(&self) -> bool {
        matches!(self, Self::Breed | Self::Speak | Self::Morph | Self::Cast)
    }
}

impl std::fmt::Display for CodieKeyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A token in the CODIE language
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CodieToken {
    // Keywords (all categories)
    Keyword(CodieKeyword),

    // Structural symbols
    /// | - continues (same level)
    Pipe,
    /// ├── - branch (sibling)
    TreeBranch,
    /// └── - last branch
    TreeLastBranch,
    /// → - flows to
    Arrow,
    /// ← - comes from
    BackArrow,
    /// ↔ - bidirectional
    BiArrow,
    /// ? - if/maybe
    Question,
    /// # - hash reference
    Hash,
    /// $ - secret (vault)
    Dollar,
    /// @ - external thing
    At,
    /// : - type annotation or key-value
    Colon,
    /// :: - scope resolution
    DoubleColon,
    /// , - separator
    Comma,
    /// . - property access
    Dot,
    /// .. - range
    DotDot,
    /// ... - spread
    Ellipsis,
    /// ( - open paren
    OpenParen,
    /// ) - close paren
    CloseParen,
    /// { - open brace
    OpenBrace,
    /// } - close brace
    CloseBrace,
    /// [ - open bracket
    OpenBracket,
    /// ] - close bracket
    CloseBracket,
    /// < - less than / open angle
    OpenAngle,
    /// > - greater than / close angle
    CloseAngle,

    // Operators
    /// + - addition
    Plus,
    /// - - subtraction
    Minus,
    /// * - multiplication
    Star,
    /// / - division
    Slash,
    /// % - modulo
    Percent,
    /// ^ - power / xor
    Caret,
    /// = - assignment
    Equals,
    /// == - equality
    DoubleEquals,
    /// != - inequality
    NotEquals,
    /// <= - less or equal
    LessEquals,
    /// >= - greater or equal
    GreaterEquals,
    /// && - logical and (symbol form)
    DoubleAnd,
    /// || - logical or (symbol form)
    DoubleOr,
    /// ! - logical not (symbol form)
    Bang,
    /// ~ - bitwise not / mirror operator
    Tilde,
    /// | - bitwise or (when not tree structure)
    BitOr,
    /// & - bitwise and
    BitAnd,

    // Literals
    /// Identifier (variable/function name)
    Identifier(String),
    /// String literal
    StringLiteral(String),
    /// Number literal
    NumberLiteral(f64),
    /// Hash reference (#abc123)
    HashRef(String),
    /// Dimensional index (dim[0], dim[1])
    DimIndex(usize),
    /// Axis name (axis.x, axis.y, axis.z, axis.w)
    AxisName(char),

    // Special tokens
    /// IN keyword (for spin loops)
    In,
    /// FOREVER keyword
    Forever,
    /// TIMES keyword
    Times,

    // Whitespace/structure
    /// Newline
    Newline,
    /// Indentation level
    Indent(usize),
    /// End of file
    Eof,
}

impl CodieToken {
    /// Check if token is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(self, CodieToken::Keyword(_))
    }

    /// Check if token starts a block
    pub fn starts_block(&self) -> bool {
        matches!(
            self,
            CodieToken::Keyword(CodieKeyword::Pug)
                | CodieToken::Keyword(CodieKeyword::Cali)
                | CodieToken::Keyword(CodieKeyword::Spin)
                | CodieToken::Keyword(CodieKeyword::Fence)
                | CodieToken::Keyword(CodieKeyword::Pin)
                | CodieToken::Keyword(CodieKeyword::Blob)
                | CodieToken::Keyword(CodieKeyword::If)
                | CodieToken::Keyword(CodieKeyword::Else)
                | CodieToken::Keyword(CodieKeyword::For)
                | CodieToken::Keyword(CodieKeyword::While)
                | CodieToken::Keyword(CodieKeyword::Fork)
                | CodieToken::Keyword(CodieKeyword::Start)
        )
    }

    /// Check if token is a logic operator
    pub fn is_logic_operator(&self) -> bool {
        matches!(
            self,
            CodieToken::Keyword(kw) if kw.is_logic_gate()
        ) || matches!(
            self,
            CodieToken::DoubleAnd | CodieToken::DoubleOr | CodieToken::Bang
        )
    }

    /// Check if token is a comparison operator
    pub fn is_comparison(&self) -> bool {
        matches!(
            self,
            CodieToken::DoubleEquals
                | CodieToken::NotEquals
                | CodieToken::OpenAngle
                | CodieToken::CloseAngle
                | CodieToken::LessEquals
                | CodieToken::GreaterEquals
        )
    }

    /// Get keyword if this is a keyword token
    pub fn as_keyword(&self) -> Option<CodieKeyword> {
        match self {
            CodieToken::Keyword(k) => Some(*k),
            _ => None,
        }
    }
}

impl std::fmt::Display for CodieToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodieToken::Keyword(k) => write!(f, "{}", k),
            CodieToken::Pipe => write!(f, "│"),
            CodieToken::TreeBranch => write!(f, "├──"),
            CodieToken::TreeLastBranch => write!(f, "└──"),
            CodieToken::Arrow => write!(f, "→"),
            CodieToken::BackArrow => write!(f, "←"),
            CodieToken::BiArrow => write!(f, "↔"),
            CodieToken::Question => write!(f, "?"),
            CodieToken::Hash => write!(f, "#"),
            CodieToken::Dollar => write!(f, "$"),
            CodieToken::At => write!(f, "@"),
            CodieToken::Colon => write!(f, ":"),
            CodieToken::DoubleColon => write!(f, "::"),
            CodieToken::Comma => write!(f, ","),
            CodieToken::Dot => write!(f, "."),
            CodieToken::DotDot => write!(f, ".."),
            CodieToken::Ellipsis => write!(f, "..."),
            CodieToken::OpenParen => write!(f, "("),
            CodieToken::CloseParen => write!(f, ")"),
            CodieToken::OpenBrace => write!(f, "{{"),
            CodieToken::CloseBrace => write!(f, "}}"),
            CodieToken::OpenBracket => write!(f, "["),
            CodieToken::CloseBracket => write!(f, "]"),
            CodieToken::OpenAngle => write!(f, "<"),
            CodieToken::CloseAngle => write!(f, ">"),
            CodieToken::Plus => write!(f, "+"),
            CodieToken::Minus => write!(f, "-"),
            CodieToken::Star => write!(f, "*"),
            CodieToken::Slash => write!(f, "/"),
            CodieToken::Percent => write!(f, "%"),
            CodieToken::Caret => write!(f, "^"),
            CodieToken::Equals => write!(f, "="),
            CodieToken::DoubleEquals => write!(f, "=="),
            CodieToken::NotEquals => write!(f, "!="),
            CodieToken::LessEquals => write!(f, "<="),
            CodieToken::GreaterEquals => write!(f, ">="),
            CodieToken::DoubleAnd => write!(f, "&&"),
            CodieToken::DoubleOr => write!(f, "||"),
            CodieToken::Bang => write!(f, "!"),
            CodieToken::Tilde => write!(f, "~"),
            CodieToken::BitOr => write!(f, "|"),
            CodieToken::BitAnd => write!(f, "&"),
            CodieToken::Identifier(s) => write!(f, "{}", s),
            CodieToken::StringLiteral(s) => write!(f, "\"{}\"", s),
            CodieToken::NumberLiteral(n) => write!(f, "{}", n),
            CodieToken::HashRef(h) => write!(f, "#{}", h),
            CodieToken::DimIndex(i) => write!(f, "dim[{}]", i),
            CodieToken::AxisName(c) => write!(f, "axis.{}", c),
            CodieToken::In => write!(f, "IN"),
            CodieToken::Forever => write!(f, "FOREVER"),
            CodieToken::Times => write!(f, "TIMES"),
            CodieToken::Newline => write!(f, "\\n"),
            CodieToken::Indent(n) => write!(f, "[indent:{}]", n),
            CodieToken::Eof => write!(f, "EOF"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_keyword_parsing() {
        assert_eq!(CodieKeyword::from_str("pug"), Some(CodieKeyword::Pug));
        assert_eq!(CodieKeyword::from_str("PUG"), Some(CodieKeyword::Pug));
        assert_eq!(CodieKeyword::from_str("bark"), Some(CodieKeyword::Bark));
        assert_eq!(CodieKeyword::from_str("invalid"), None);
    }

    #[test]
    fn test_logic_gate_parsing() {
        assert_eq!(CodieKeyword::from_str("and"), Some(CodieKeyword::And));
        assert_eq!(CodieKeyword::from_str("or"), Some(CodieKeyword::Or));
        assert_eq!(CodieKeyword::from_str("not"), Some(CodieKeyword::Not));
        assert_eq!(CodieKeyword::from_str("xor"), Some(CodieKeyword::Xor));
        assert_eq!(CodieKeyword::from_str("nand"), Some(CodieKeyword::Nand));
        assert_eq!(CodieKeyword::from_str("nor"), Some(CodieKeyword::Nor));
    }

    #[test]
    fn test_control_flow_parsing() {
        assert_eq!(CodieKeyword::from_str("if"), Some(CodieKeyword::If));
        assert_eq!(CodieKeyword::from_str("else"), Some(CodieKeyword::Else));
        assert_eq!(CodieKeyword::from_str("start"), Some(CodieKeyword::Start));
        assert_eq!(CodieKeyword::from_str("for"), Some(CodieKeyword::For));
        assert_eq!(CodieKeyword::from_str("fork"), Some(CodieKeyword::Fork));
        assert_eq!(CodieKeyword::from_str("branch"), Some(CodieKeyword::Branch));
        assert_eq!(CodieKeyword::from_str("while"), Some(CodieKeyword::While));
    }

    #[test]
    fn test_boolean_parsing() {
        assert_eq!(CodieKeyword::from_str("true"), Some(CodieKeyword::True));
        assert_eq!(CodieKeyword::from_str("false"), Some(CodieKeyword::False));
        assert!(CodieKeyword::True.is_boolean());
        assert!(CodieKeyword::False.is_boolean());
    }

    #[test]
    fn test_geometric_parsing() {
        assert_eq!(CodieKeyword::from_str("mirror"), Some(CodieKeyword::Mirror));
        assert_eq!(CodieKeyword::from_str("fold"), Some(CodieKeyword::Fold));
        assert_eq!(CodieKeyword::from_str("rotate"), Some(CodieKeyword::Rotate));
        assert!(CodieKeyword::Mirror.is_geometric());
    }

    #[test]
    fn test_dimensional_parsing() {
        assert_eq!(CodieKeyword::from_str("dim"), Some(CodieKeyword::Dim));
        assert_eq!(CodieKeyword::from_str("axis"), Some(CodieKeyword::Axis));
        assert_eq!(CodieKeyword::from_str("plane"), Some(CodieKeyword::Plane));
        assert_eq!(CodieKeyword::from_str("space"), Some(CodieKeyword::Space));
        assert_eq!(CodieKeyword::from_str("hyper"), Some(CodieKeyword::Hyper));
        assert!(CodieKeyword::Dim.is_dimensional());
    }

    #[test]
    fn test_all_core_keywords_defined() {
        for kw in CODIE_CORE_KEYWORDS {
            assert!(
                CodieKeyword::from_str(kw).is_some(),
                "Core keyword {} should parse",
                kw
            );
        }
    }

    #[test]
    fn test_all_logic_gates_defined() {
        for kw in CODIE_LOGIC_GATES {
            let parsed = CodieKeyword::from_str(kw);
            assert!(parsed.is_some(), "Logic gate {} should parse", kw);
            assert!(
                parsed.unwrap().is_logic_gate(),
                "Logic gate {} should be marked as logic gate",
                kw
            );
        }
    }

    #[test]
    fn test_all_control_flow_defined() {
        for kw in CODIE_CONTROL_FLOW {
            let parsed = CodieKeyword::from_str(kw);
            assert!(parsed.is_some(), "Control flow {} should parse", kw);
            assert!(
                parsed.unwrap().is_control_flow(),
                "Control flow {} should be marked as control flow",
                kw
            );
        }
    }

    #[test]
    fn test_keyword_categories() {
        assert!(CodieKeyword::Pug.is_core());
        assert!(!CodieKeyword::Pug.is_logic_gate());

        assert!(CodieKeyword::And.is_logic_gate());
        assert!(!CodieKeyword::And.is_core());

        assert!(CodieKeyword::If.is_control_flow());
        assert!(!CodieKeyword::If.is_geometric());

        assert!(CodieKeyword::Mirror.is_geometric());
        assert!(CodieKeyword::Dim.is_dimensional());
    }

    #[test]
    fn test_keyword_roundtrip() {
        let all_keywords = [
            CodieKeyword::Pug,
            CodieKeyword::And,
            CodieKeyword::If,
            CodieKeyword::True,
            CodieKeyword::Mirror,
            CodieKeyword::Dim,
        ];

        for kw in all_keywords {
            let s = kw.as_str();
            let parsed = CodieKeyword::from_str(s).unwrap();
            assert_eq!(parsed, kw);
        }
    }

    #[test]
    fn test_meta_parsing() {
        assert_eq!(CodieKeyword::from_str("breed"), Some(CodieKeyword::Breed));
        assert_eq!(CodieKeyword::from_str("speak"), Some(CodieKeyword::Speak));
        assert_eq!(CodieKeyword::from_str("morph"), Some(CodieKeyword::Morph));
        assert_eq!(CodieKeyword::from_str("cast"), Some(CodieKeyword::Cast));
        assert!(CodieKeyword::Breed.is_meta());
        assert!(CodieKeyword::Speak.is_meta());
    }

    #[test]
    fn test_total_keyword_count() {
        // 12 core + 6 logic + 10 control + 2 bool + 5 geometric + 5 dimensional + 4 meta = 44
        let total = CODIE_CORE_KEYWORDS.len()
            + CODIE_LOGIC_GATES.len()
            + CODIE_CONTROL_FLOW.len()
            + CODIE_BOOLEANS.len()
            + CODIE_GEOMETRIC.len()
            + CODIE_DIMENSIONAL.len()
            + CODIE_META.len();
        assert_eq!(total, 44);
    }
}
