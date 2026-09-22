use shakmaty::{Bitboard, Color, Piece, Role, Square};
use std::collections::HashMap;
use std::ops::Range;

/// Square Set expression evaluating to a 64-bit Bitboard on a given chess position
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SquareSetExpr {
    /// Board pieces: `B` (white bishops), `n` (black knights), `occupied`, `empty`, `white_pieces`, `black_pieces`
    Piece(SquareContent),

    /// Explicit squares/ranges: `[c1, f1]`, `[a1..h8]`, `light`, `dark`, `a-h1`
    Squares(Bitboard),

    /// Bound variable reference: `$target`, `$attacker`
    Variable(String),

    /// Set Intersection (A ∩ B): `A & B` or juxtaposition `A B` (e.g. `B [c1, f1]`)
    Intersection(Box<SquareSetExpr>, Box<SquareSetExpr>),

    /// Set Union (A ∪ B): `A | B` or `[N, B]`
    Union(Box<SquareSetExpr>, Box<SquareSetExpr>),

    /// Set Difference (A \ B): `A \ B` or `A - B` (e.g. `occupied \ [d4, e5]`)
    Difference(Box<SquareSetExpr>, Box<SquareSetExpr>),

    /// Set Complement (~A): all 64 squares not in A
    Complement(Box<SquareSetExpr>),

    /// Geometric / Attack targets: `attacks(attacker_set, target_set)`
    /// Returns the subset of target_set that is attacked by any piece in attacker_set
    Attacks {
        attacker: Box<SquareSetExpr>,
        target: Box<SquareSetExpr>,
    },

    /// Attack origins: `attackers(attacker_set, target_set)`
    /// Returns the subset of attacker_set that attacks any piece in target_set
    Attackers {
        attacker: Box<SquareSetExpr>,
        target: Box<SquareSetExpr>,
    },

    /// Directional ray expansion: `ray(up, d4)`, `ray(diagonal, [e4, d5])`
    Ray {
        direction: Direction,
        origin: Box<SquareSetExpr>,
    },

    /// Squares strictly between two sets of pieces/squares: `between(sq1, sq2)`
    Between {
        from: Box<SquareSetExpr>,
        to: Box<SquareSetExpr>,
    },

    /// Directional spatial shift / translation: `northwest 2 Q`, `up 1 k`, `right 1..3 [d4, e5]`
    /// Shifts each square in `expr` by min_dist..=max_dist in the specified direction.
    Shift {
        direction: Direction,
        min_dist: usize,
        max_dist: usize,
        expr: Box<SquareSetExpr>,
    },
}

/// Set Predicate evaluated in SearchQuery
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetPredicate {
    /// Non-empty boolean test: evaluates to `!set.is_empty()`
    /// Example: `B [c1, f1]`, `attacks(R, k)`
    NonEmpty(SquareSetExpr),

    /// Count comparison against integer literal: `count(expr) op count`
    /// Example: `attacks(R, k) >= 2`, `B [a1..h8] == 2`
    CountComparison {
        expr: SquareSetExpr,
        op: ComparisonOp,
        count: usize,
    },

    /// Set size comparison between two expressions: `count(left) op count(right)`
    /// Example: `attacks(white, e4) > attacks(black, e4)`
    SetComparison {
        left: SquareSetExpr,
        op: ComparisonOp,
        right: SquareSetExpr,
    },
}

/// Comparison operators for numeric and string matching
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
}

impl ComparisonOp {
    pub fn invert(&self) -> Self {
        match self {
            ComparisonOp::GreaterThan => ComparisonOp::LessThan,
            ComparisonOp::GreaterThanOrEqual => ComparisonOp::LessThanOrEqual,
            ComparisonOp::LessThan => ComparisonOp::GreaterThan,
            ComparisonOp::LessThanOrEqual => ComparisonOp::GreaterThanOrEqual,
            other => *other,
        }
    }
}

/// Predicate for matching PGN header tags and metadata
#[derive(Debug, Clone, PartialEq)]
pub enum HeaderPredicate {
    /// Match a specific tag by name and string value
    Tag {
        name: String,
        op: ComparisonOp,
        value: String,
        case_sensitive: bool,
    },
    /// Match player name (matches either White or Black)
    Player {
        name: String,
        op: ComparisonOp,
        case_sensitive: bool,
    },
    /// Match White player name
    White {
        name: String,
        op: ComparisonOp,
        case_sensitive: bool,
    },
    /// Match Black player name
    Black {
        name: String,
        op: ComparisonOp,
        case_sensitive: bool,
    },
    /// Match White rating / Elo
    WhiteElo { op: ComparisonOp, value: u16 },
    /// Match Black rating / Elo
    BlackElo { op: ComparisonOp, value: u16 },
    /// Match either player's rating / Elo
    AnyElo { op: ComparisonOp, value: u16 },
    /// Match average rating of both players
    AvgElo { op: ComparisonOp, value: u16 },
    /// Match rating difference: `|WhiteElo - BlackElo|` or `WhiteElo - BlackElo`
    EloDiff {
        op: ComparisonOp,
        value: i32,
        absolute: bool,
    },
    /// Match game result ("1-0", "0-1", "1/2-1/2", "*")
    Result { expected: String },
    /// Match ECO code prefix or pattern (e.g. "B", "B9", "B90")
    Eco { code: String, op: ComparisonOp },
    /// Match date range (e.g. YYYY, YYYY.MM, YYYY.MM.DD)
    Date { op: ComparisonOp, value: String },
    /// Match Event tag
    Event {
        name: String,
        op: ComparisonOp,
        case_sensitive: bool,
    },
    /// Match Site tag
    Site {
        name: String,
        op: ComparisonOp,
        case_sensitive: bool,
    },
    /// Match Round tag
    Round { value: String },
    /// Custom metadata key-value check
    Custom {
        key: String,
        op: ComparisonOp,
        value: String,
    },
}

/// Piece placement requirement on a specific square or set of squares
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SquareContent {
    Empty,
    Occupied,
    Piece(Piece),
    Role(Role),
    Color(Color),
    AnyOf(Vec<Piece>),
    NoneOf(Vec<Piece>),
}

/// Position search pattern for evaluating board state
#[derive(Debug, Clone, PartialEq)]
pub enum PositionPattern {
    /// Exact full position match by FEN string
    ExactFen(String),
    /// Piece placement match only (ignoring turn, castling, en passant, move count)
    PiecePlacement(String),
    /// Zobrist 64-bit hash match
    ZobristHash(u64),
    /// Specified piece contents on specific squares
    Squares(HashMap<Square, SquareContent>),
    /// Count of specific pieces on board (e.g. White Queen count == 1, Black Pawns <= 4, or filtered to square subset)
    PieceCount {
        content: SquareContent,
        squares: Option<Vec<Square>>,
        op: ComparisonOp,
        count: usize,
    },
    /// Turn to move
    Turn(Color),
    /// Castling availability flag
    Castling {
        color: Color,
        kingside: Option<bool>,
        queenside: Option<bool>,
    },
    /// Check, Checkmate, or Stalemate state
    BoardState {
        is_check: Option<bool>,
        is_checkmate: Option<bool>,
        is_stalemate: Option<bool>,
    },
    /// Geometric / Attack predicate (e.g., square attacker, pin, ray)
    Attack { from: Square, to: Square },
    /// Square is attacked by a color
    IsAttacked { square: Square, by_color: Color },
    /// Piece placed on any square in the specified list (e.g., Knight on any of [d4, b4, c4])
    MultiSquare {
        content: SquareContent,
        squares: Vec<Square>,
    },
    /// Position pattern evaluated under board symmetry / geometric transformations
    Symmetric {
        pattern: Box<PositionPattern>,
        symmetry: super::transform::BoardSymmetry,
    },
    /// Ply number in the game (e.g., ply == 10, ply <= 20)
    Ply { op: ComparisonOp, value: usize },
    /// Move number in the game (1-indexed: 1. e4 is move 1, 1... e5 is move 1, 2. Nf3 is move 2)
    MoveNumber { op: ComparisonOp, value: usize },
    /// Filter bound variable to a square subset (e.g. `$n on [c3, d5]`, `$p on light`)
    VariableSquareFilter {
        var_name: String,
        squares: Vec<Square>,
    },
}

/// Compass and geometric board direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
    Diagonal,
    Orthogonal,
    Vertical,
    Horizontal,
    AnyDirection,
}

impl Direction {
    pub fn delta_vectors(&self) -> &'static [(i32, i32)] {
        match self {
            Direction::Up => &[(0, 1)],
            Direction::Down => &[(0, -1)],
            Direction::Right => &[(1, 0)],
            Direction::Left => &[(-1, 0)],
            Direction::NorthEast => &[(1, 1)],
            Direction::NorthWest => &[(-1, 1)],
            Direction::SouthEast => &[(1, -1)],
            Direction::SouthWest => &[(-1, -1)],
            Direction::Vertical => &[(0, 1), (0, -1)],
            Direction::Horizontal => &[(-1, 0), (1, 0)],
            Direction::Orthogonal => &[(0, 1), (0, -1), (1, 0), (-1, 0)],
            Direction::Diagonal => &[(1, 1), (-1, 1), (1, -1), (-1, -1)],
            Direction::AnyDirection => &[
                (0, 1),
                (0, -1),
                (1, 0),
                (-1, 0),
                (1, 1),
                (-1, 1),
                (1, -1),
                (-1, -1),
            ],
        }
    }

    pub fn expand_square(&self, sq: Square, min_dist: usize, max_dist: usize) -> Vec<Square> {
        let f = sq.file() as i32;
        let r = sq.rank() as i32;
        let mut out = Vec::new();
        for &(df, dr) in self.delta_vectors() {
            for step in min_dist..=max_dist {
                let nf = f + (step as i32) * df;
                let nr = r + (step as i32) * dr;
                if (0..=7).contains(&nf) && (0..=7).contains(&nr) {
                    out.push(Square::from_coords(
                        shakmaty::File::new(nf as u32),
                        shakmaty::Rank::new(nr as u32),
                    ));
                }
            }
        }
        out
    }

    pub fn expand_squares(&self, sqs: &[Square], min_dist: usize, max_dist: usize) -> Vec<Square> {
        let mut out = Vec::new();
        for &sq in sqs {
            out.extend(self.expand_square(sq, min_dist, max_dist));
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// Pattern matching an individual move in a game or legal move analysis
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MovePattern {
    /// SAN notation string (e.g. "e4", "Nf3", "exd5", "O-O", "Qh5#")
    pub san: Option<String>,
    /// UCI notation string (e.g. "e2e4", "g1f3", "e7e8q")
    pub uci: Option<String>,
    /// Source square
    pub from: Option<Square>,
    /// Source square set (e.g. from [e1, e8] or from [a1..h8])
    pub from_squares: Option<Vec<Square>>,
    /// Source piece specifiers (e.g. from B, from [B, N], from A)
    pub from_pieces: Option<Vec<SquareContent>>,
    /// Destination square
    pub to: Option<Square>,
    /// Destination square set (e.g. to [c1, g1, c8, g8] or to [d8])
    pub to_squares: Option<Vec<Square>>,
    /// Destination piece specifiers (e.g. to R, to [r, q], to a)
    pub to_pieces: Option<Vec<SquareContent>>,
    /// Moving piece role
    pub role: Option<Role>,
    /// Moving piece color
    pub color: Option<Color>,
    /// Whether the move is a capture
    pub is_capture: Option<bool>,
    /// Captured piece specifiers (e.g. `capture p`, `capture [q, r]`, `capture black_pieces`)
    pub captured_pieces: Option<Vec<SquareContent>>,
    /// Promotion piece role
    pub promotion: Option<Role>,
    /// Allowed promotion piece roles (for multi-piece underpromotions e.g. "RBN")
    pub promotions: Option<Vec<Role>>,
    /// Whether the move gives check
    pub is_check: Option<bool>,
    /// Whether the move gives checkmate (mate in 1)
    pub is_checkmate: Option<bool>,
    /// Whether the move is a castling move (O-O or O-O-O)
    pub is_castle: Option<bool>,
    /// Whether the move is an en passant capture
    pub is_en_passant: Option<bool>,
    /// Relative move direction constraint from source square (e.g. `up 1`, `right 1`, `diagonal`, `orthogonal`)
    pub direction: Option<(Direction, usize, Option<usize>)>,
    /// If true, queries the previous move that arrived at the current position instead of the departure move
    pub is_previous: bool,
    /// If true, queries legal moves available in current position
    pub is_legal: bool,
    /// Count constraint on matching moves (e.g. `legal count == 0` or `move count >= 1`)
    pub count_predicate: Option<(ComparisonOp, usize)>,
}

/// An element in a sequential move path (move pattern or gap quantifier)
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum PathStep {
    /// A move pattern to match at current ply, with optional repetition bounds (min, max)
    Move {
        pattern: MovePattern,
        repeat_min: usize,
        repeat_max: Option<usize>,
    },
    /// An explicit gap / repetition quantifier between moves (min_plies, max_plies)
    Gap { min: usize, max: Option<usize> },
}

/// Sequential path pattern matching a line or sequence of moves
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PathPattern {
    /// Ordered list of path steps (moves and explicit gaps)
    pub steps: Vec<PathStep>,
    /// Ordered list of move patterns to match
    pub moves: Vec<MovePattern>,
    /// Whether the moves must be strictly consecutive (ply by ply without gaps)
    pub consecutive: bool,
    /// If Some, matches only moves by a single color (None = auto-detect from first matched move or White/Black if specified)
    pub single_color: Option<Option<Color>>,
    /// Maximum allowed plies between matched moves (if not consecutive)
    pub max_gap_plies: Option<usize>,
    /// Start ply range where the sequence must begin
    pub start_ply_range: Option<Range<usize>>,
}

/// A constituent in a CQL 6.2 `cql_path` filter: either a Move, a Positional Filter, or a repeated chain
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum CqlPathConstituent {
    /// Move constituent (e.g. `Bxh7+`, `kxh7`, `N--g5 check`, `e4 not check`)
    Move(MovePattern),
    /// Filter constituent evaluated against the current board state (e.g. `check`, `not check`, `{ attacks(N, q) }`)
    Filter(Box<SearchQuery>),
    /// Repetition of a constituent: `*`, `+`, `?`, or `{min, max}`
    Repetition {
        constituent: Box<CqlPathConstituent>,
        min: usize,
        max: Option<usize>,
    },
    /// A chain of constituents enclosed in parentheses `( C1 C2 ... )`
    Chain(Vec<CqlPathConstituent>),
}

/// CQL 6.2 Path Pattern (`cql_path` / `cqlpath` / `turnstile` / `sequence`)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CqlPathPattern {
    /// Ordered list of constituents (moves and filters)
    pub constituents: Vec<CqlPathConstituent>,
    /// Optional single color constraint (e.g. `cql_path singlecolor { ... }`, `cql_path white { ... }`)
    pub single_color: Option<Option<Color>>,
    /// Optional start ply range
    pub start_ply_range: Option<Range<usize>>,
}

impl CqlPathPattern {
    pub fn requires_san_strings(&self) -> bool {
        self.constituents.iter().any(|c| c.requires_san_strings())
    }
}

/// Direction of transition in a CQLi line filter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum LineDirection {
    #[default]
    Forward, // -->
    Backward, // <--
}

/// CQLi Line Pattern (`line` / `cqlline` / `cql_line`)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CqlLinePattern {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub direction: LineDirection,
    pub single_color: Option<Option<Color>>,
    pub first_match: bool,
    pub last_position: bool,
    pub nest_ban: bool,
    pub primary_only: bool,
    pub start_ply_range: Option<Range<usize>>,
    pub constituents: Vec<CqlPathConstituent>,
}

impl CqlLinePattern {
    pub fn requires_san_strings(&self) -> bool {
        self.constituents.iter().any(|c| c.requires_san_strings())
    }
}

impl CqlPathConstituent {
    pub fn requires_san_strings(&self) -> bool {
        match self {
            CqlPathConstituent::Move(m) => m.san.is_some(),
            CqlPathConstituent::Filter(q) => q.requires_san_strings(),
            CqlPathConstituent::Repetition { constituent, .. } => {
                constituent.requires_san_strings()
            }
            CqlPathConstituent::Chain(subs) => subs.iter().any(|s| s.requires_san_strings()),
        }
    }
}

/// Hardware-accelerated bitboard material composition predicate
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MaterialPredicate {
    pub white_pawns: Option<usize>,
    pub white_knights: Option<usize>,
    pub white_bishops: Option<usize>,
    pub white_light_bishops: Option<usize>,
    pub white_dark_bishops: Option<usize>,
    pub white_rooks: Option<usize>,
    pub white_queens: Option<usize>,
    pub black_pawns: Option<usize>,
    pub black_knights: Option<usize>,
    pub black_bishops: Option<usize>,
    pub black_light_bishops: Option<usize>,
    pub black_dark_bishops: Option<usize>,
    pub light_bishops: Option<usize>,
    pub dark_bishops: Option<usize>,
    pub black_rooks: Option<usize>,
    pub black_queens: Option<usize>,
    /// Total material points difference (White points - Black points: P=1, N=3, B=3, R=5, Q=9)
    pub material_difference: Option<(ComparisonOp, i32)>,
    /// Opposite-colored bishops (White has 1 light/dark bishop and Black has 1 opposite bishop)
    pub opposite_bishops: Option<bool>,
    /// Same-colored bishops
    pub same_colored_bishops: Option<bool>,
}

/// Flexible piece pattern matcher for tactical queries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceMatcher {
    pub color: Option<Color>,
    pub role: Option<Role>,
}

impl PieceMatcher {
    pub const ANY: Self = Self {
        color: None,
        role: None,
    };
    pub const WHITE: Self = Self {
        color: Some(Color::White),
        role: None,
    };
    pub const BLACK: Self = Self {
        color: Some(Color::Black),
        role: None,
    };

    pub fn new(color: Option<Color>, role: Option<Role>) -> Self {
        Self { color, role }
    }

    pub fn matches(&self, piece: &Piece) -> bool {
        if let Some(c) = self.color {
            if piece.color != c {
                return false;
            }
        }
        if let Some(r) = self.role {
            if piece.role != r {
                return false;
            }
        }
        true
    }
}

/// Square, piece, or variable specifier for spatial/distance/attack queries
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SquareOrPiece {
    Square(Square),
    Piece(PieceMatcher),
    Variable(String),
    Empty,
}

/// Domain of a bound piece/square variable
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableDomain {
    Piece(PieceMatcher),
    SquareSet(Vec<Square>),
    AnyPiece,
}

/// Tactical motif and geometric relationship predicates
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TacticalPredicate {
    /// Pin: Pinner attacks Target through Pinned piece
    Pin {
        pinners: Vec<PieceMatcher>,
        pinneds: Vec<PieceMatcher>,
        targets: Vec<PieceMatcher>,
    },
    /// Fork: Single attacker simultaneously attacks multiple enemy pieces
    Fork {
        attackers: Vec<PieceMatcher>,
        target_slots: Vec<Vec<PieceMatcher>>,
        targets_pool: Vec<PieceMatcher>,
        min_targets: usize,
    },
    /// Discovered Attack / Discovered Check
    DiscoveredAttack { color: Color, is_check: bool },
    /// Skewer: Attacker attacks Front piece, exposing Rear piece behind it
    Skewer {
        attackers: Vec<PieceMatcher>,
        fronts: Vec<PieceMatcher>,
        rears: Vec<PieceMatcher>,
    },
    /// Trapped piece with 0 safe/legal escape squares
    TrappedPiece { piece: PieceMatcher },
    /// Outpost piece (typically Knight on 4th/5th/6th rank guarded by friendly pawn)
    Outpost {
        piece: PieceMatcher,
        square: Option<Square>,
    },
    /// Rook on the 7th rank (7th rank for White, 2nd rank for Black)
    RookOnSeventh { color: Color },
    /// Open or semi-open file
    OpenFile {
        file: Option<shakmaty::File>,
        semi_open_for: Option<Color>,
    },
    /// Chebyshev (King step) distance between two squares or pieces
    Distance {
        sq1: SquareOrPiece,
        sq2: SquareOrPiece,
        op: ComparisonOp,
        distance: usize,
    },
    /// Piece attacks target piece, color, or square
    Attacks {
        attacker: SquareOrPiece,
        target: SquareOrPiece,
    },
}

/// Bitboard pawn structure predicate (passed pawns, isolated, doubled, backward, islands)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PawnPredicate {
    PassedPawns {
        color: Color,
        op: ComparisonOp,
        count: usize,
    },
    IsolatedPawns {
        color: Color,
        op: ComparisonOp,
        count: usize,
    },
    DoubledPawns {
        color: Color,
        op: ComparisonOp,
        count: usize,
    },
    BackwardPawns {
        color: Color,
        op: ComparisonOp,
        count: usize,
    },
    PawnIslands {
        color: Color,
        op: ComparisonOp,
        count: usize,
    },
}

/// Material / Piece Power evaluation predicate
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerPredicate {
    /// Compare White power points against a numeric value: white_power >= 30
    WhitePower { op: ComparisonOp, value: i32 },
    /// Compare Black power points against a numeric value: black_power >= 30
    BlackPower { op: ComparisonOp, value: i32 },
    /// Compare Total board power (White + Black): total_power <= 20
    TotalPower { op: ComparisonOp, value: i32 },
    /// Compare White power directly against Black power: white_power > black_power, white_power == black_power
    WhiteVsBlackPower { op: ComparisonOp },
    /// Power difference: (White power - Black power) or |White power - Black power|
    PowerDifference {
        op: ComparisonOp,
        value: i32,
        absolute: bool,
    },
}

/// Main composite search query node (CQL-inspired AST)
#[derive(Debug, Clone, PartialEq)]
pub enum SearchQuery {
    /// Matches all of the subqueries (logical AND)
    And(Vec<SearchQuery>),
    /// Matches any of the subqueries (logical OR)
    Or(Vec<SearchQuery>),
    /// Inverts the subquery match (logical NOT)
    Not(Box<SearchQuery>),
    /// Matches game metadata and PGN headers
    Header(HeaderPredicate),
    /// Matches a board position pattern
    Position(PositionPattern),
    /// Matches a pawn structure configuration
    Pawn(PawnPredicate),
    /// Matches tactical and geometric motifs
    Tactical(TacticalPredicate),
    /// Matches a specific move
    Move(MovePattern),
    /// Matches a sequence / path of moves
    Path(PathPattern),
    /// Matches bitboard material composition
    Material(MaterialPredicate),
    /// Matches piece material power and power comparisons
    Power(PowerPredicate),
    /// Matches PGN annotations, comments, or NAGs
    Annotation(super::annotation::AnnotationPredicate),
    /// Evaluates subquery under board symmetry transformations
    Symmetric {
        query: Box<SearchQuery>,
        symmetry: super::transform::BoardSymmetry,
    },
    /// Restricts position/move searches to a specific ply range (e.g. 1..20 for opening)
    PlyRange {
        range: Range<usize>,
        query: Box<SearchQuery>,
    },
    /// Restricts by occurrence count (e.g. position occurs at least 2 times)
    Occurrences {
        min: usize,
        max: Option<usize>,
        query: Box<SearchQuery>,
    },
    /// Binds a piece/square variable over matching candidates (e.g. `$ForkingKnight = N` or `piece $n in N { ... }`)
    VariableBinding {
        var_name: String,
        domain: VariableDomain,
        query: Box<SearchQuery>,
    },
    /// First-class Square Set predicate (non-empty, count comparison, or set size comparison)
    SquareSet(SetPredicate),
    /// Scopes evaluation to the parent position (ply - 1) before the current position
    Parent(Box<SearchQuery>),
    /// Scopes evaluation to the child position (ply + 1) after the current position
    Child(Box<SearchQuery>),
    /// Hypothetical move execution: simulates a matching legal/explicit move on a cloned board and evaluates outcome query
    Play {
        move_pattern: MovePattern,
        outcome_query: Box<SearchQuery>,
    },
    /// CQL 6.2 Path Pattern with interleaved moves and positional state filters
    CqlPath(CqlPathPattern),
    /// CQLi Line Pattern with position/move transitions along arrows
    CqlLine(CqlLinePattern),
}

impl SearchQuery {
    pub fn and(queries: Vec<SearchQuery>) -> Self {
        Self::And(queries)
    }

    pub fn or(queries: Vec<SearchQuery>) -> Self {
        Self::Or(queries)
    }

    pub fn negate(query: SearchQuery) -> Self {
        Self::Not(Box::new(query))
    }

    pub fn is_header_only(&self) -> bool {
        match self {
            SearchQuery::Header(_) => true,
            SearchQuery::And(subs) | SearchQuery::Or(subs) => {
                subs.iter().all(|s| s.is_header_only())
            }
            SearchQuery::Not(sub)
            | SearchQuery::Parent(sub)
            | SearchQuery::Child(sub)
            | SearchQuery::PlyRange { query: sub, .. }
            | SearchQuery::Occurrences { query: sub, .. } => sub.is_header_only(),
            _ => false,
        }
    }

    pub fn requires_san_strings(&self) -> bool {
        match self {
            SearchQuery::Move(m) => m.san.is_some(),
            SearchQuery::Path(p) => p.moves.iter().any(|m| m.san.is_some()),
            SearchQuery::CqlPath(p) => p.requires_san_strings(),
            SearchQuery::CqlLine(p) => p.requires_san_strings(),
            SearchQuery::And(subs) | SearchQuery::Or(subs) => {
                subs.iter().any(|s| s.requires_san_strings())
            }
            SearchQuery::Not(sub)
            | SearchQuery::Parent(sub)
            | SearchQuery::Child(sub)
            | SearchQuery::PlyRange { query: sub, .. }
            | SearchQuery::Occurrences { query: sub, .. }
            | SearchQuery::Play {
                outcome_query: sub, ..
            }
            | SearchQuery::VariableBinding { query: sub, .. } => sub.requires_san_strings(),
            SearchQuery::Symmetric { query: sub, .. } => sub.requires_san_strings(),
            _ => false,
        }
    }

    pub fn has_header_predicates(&self) -> bool {
        match self {
            SearchQuery::Header(_) => true,
            SearchQuery::And(subs) | SearchQuery::Or(subs) => {
                subs.iter().any(|s| s.has_header_predicates())
            }
            SearchQuery::Not(sub)
            | SearchQuery::Parent(sub)
            | SearchQuery::Child(sub)
            | SearchQuery::PlyRange { query: sub, .. }
            | SearchQuery::Occurrences { query: sub, .. }
            | SearchQuery::Play {
                outcome_query: sub, ..
            }
            | SearchQuery::VariableBinding { query: sub, .. } => sub.has_header_predicates(),
            SearchQuery::Symmetric { query: sub, .. } => sub.has_header_predicates(),
            _ => false,
        }
    }

    pub fn can_stream_early_exit(&self) -> bool {
        match self {
            SearchQuery::Position(_)
            | SearchQuery::SquareSet(_)
            | SearchQuery::Pawn(_)
            | SearchQuery::Tactical(_)
            | SearchQuery::Material(_)
            | SearchQuery::Power(_) => true,
            SearchQuery::Move(m) => {
                m.is_previous && !m.is_legal && m.count_predicate.is_none() && m.san.is_none()
            }
            SearchQuery::PlyRange { query: sub, .. } => sub.can_stream_early_exit(),
            SearchQuery::Or(subs) => subs.iter().all(|s| s.can_stream_early_exit()),
            SearchQuery::And(subs) => subs.iter().all(|s| match s {
                SearchQuery::Position(_)
                | SearchQuery::SquareSet(_)
                | SearchQuery::Pawn(_)
                | SearchQuery::Tactical(_)
                | SearchQuery::Material(_)
                | SearchQuery::Power(_) => true,
                SearchQuery::Move(m) => {
                    m.is_previous && !m.is_legal && m.count_predicate.is_none() && m.san.is_none()
                }
                SearchQuery::PlyRange { query: sub, .. } => sub.can_stream_early_exit(),
                _ => false,
            }),
            _ => false,
        }
    }

    pub fn max_ply_cutoff(&self) -> Option<usize> {
        match self {
            SearchQuery::PlyRange { range, .. } => Some(range.end),
            SearchQuery::Position(PositionPattern::Ply {
                op: ComparisonOp::LessThan | ComparisonOp::LessThanOrEqual,
                value,
            }) => Some(value + 1),
            SearchQuery::Position(PositionPattern::Ply {
                op: ComparisonOp::Equal,
                value,
            }) => Some(value + 1),
            _ => None,
        }
    }
}

impl std::ops::Not for SearchQuery {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::Not(Box::new(self))
    }
}
