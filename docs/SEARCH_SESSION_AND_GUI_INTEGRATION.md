# 🔍 Search Session & GUI Integration Guide

This guide explains how frontend desktop and web applications (e.g. PyQt5/C++/Electron/React) interact with the `scid-mgr` search engine. It details the **2-phase search session architecture**, the internal representation of **Game IDs and Matching Plies**, on-demand multi-column sorting, paginated streaming, and instant chessboard navigation.

---

## 1. High-Level Architecture Overview

Whenever a search is executed—whether through **CQL language queries**, **direct position/FEN lookups**, or **bitboard material filters**—the backend engine computes results into an in-memory session.

```
┌────────────────────────────────────────────────────────────────────────┐
│                              Client / GUI                              │
│  1. Initiate Search (CQL / FEN / Material)                             │
│  2. Receive `search_id` & total match count                            │
│  3. Request paginated slices (`query_games` with `search_id`)          │
│  4. Highlight game row & jump chessboard directly to `matching_plies`  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ JSON-RPC (stdin/stdout)
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                       scid-mgr Rust Server Engine                      │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    Search Execution Engine                       │  │
│  │   - 16-Bit Search Booster (`.boost.idx` parallel flat-stream)    │  │
│  │   - CQL / DSL Query Evaluator (Moves, Paths, Motifs, Symmetries) │  │
│  │   - Inverted Position Index (`.pos.idx` candidate lookup)        │  │
│  │   - Bitboard Material Analyzer (Piece counts, Bishop colors)     │  │
│  └──────────────────────────────────┬───────────────────────────────┘  │
│                                     ▼
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                     Active `SearchSession`                       │  │
│  │   - `search_id`: Unique identifier (e.g. "search_1")             │  │
│  │   - `matches`: Vec<ScidMatchResult>                              │  │
│  │       ├── game_id: usize (0-indexed game identifier)             │  │
│  │       └── match_details: QueryMatchResult                        │  │
│  │             ├── is_match: bool                                   │  │
│  │             ├── match_count: usize                               │  │
│  │             └── matching_plies: Vec<usize> (matching ply numbers)│  │
│  │   - `sorted_cache`: Rayon parallel sort cache per column/order   │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Core Data Models

### `ScidMatchResult`
In the Rust engine (`src/search/scid_adapter.rs`), each matched game record stores:
* **`game_id`** (`usize`): 0-indexed position of the game in the database.
* **`match_details`** (`QueryMatchResult`):
  * **`is_match`** (`bool`): `true` if the game satisfies the query.
  * **`match_count`** (`usize`): Total number of matching positions found in the game.
  * **`matching_plies`** (`Vec<usize>`): 0-indexed list of half-move numbers (plies) within the game where the query condition was met (e.g. ply `0` for startpos, ply `1` for `1. e4`, ply `21` for move `11. Bxb5+`, etc.).

### `SearchSession`
Cached in the server (`src/server/search_session.rs`):
* **`search_id`** (`String`): Identifier token returned to the client (`main_1`, `ref_2`, etc.).
* **`owner`** (`SessionOwner`): Distinguishes the consumer context:
  * **`SessionOwner::Main` (`"main"`)**: Primary database filter / CQL search applied to the main database grid view. Permanent and never evicted by explorer navigation.
  * **`SessionOwner::Reference` (`"reference"`)**: Opening tree reference explorer session. Stored in a bounded LRU pool (default capacity: 3).
* **`query`** (`SessionQuery`): Encapsulates query identity and metadata (`PurePosition`, `FilteredPosition`, `HeaderSearch`, `CqlSearch`, `General`).
* **`matches`** (`Vec<ScidMatchResult>`): Complete vector of matching games and their plies.
* **`sorted_cache`** (`HashMap<(Option<String>, bool), Vec<ScidMatchResult>>`): Caches sorted order per column and direction so repeated page requests or scrolling are instantaneous.

---

## 3. Dual-Pool Architecture & Session Isolation

To prevent rapid board navigation in the Reference Explorer / Opening Tree from evicting active database filters, `scid-mgr` uses a dual-pool session manager:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        SearchSessionManager                            │
├───────────────────────────────────┬────────────────────────────────────┤
│         Main Table Pool           │      Reference Explorer Pool       │
│  (Advanced Search / CQL / Filter) │     (Opening Tree Board Moves)     │
├───────────────────────────────────┼────────────────────────────────────┤
│ • `main_session: Option<Search>`  │ • `reference_sessions: HashMap`    │
│ • **Permanent Lifetime**          │ • **Bounded LRU Cache (cap: 3)**   │
│ • Never evicted by tree moves     │ • Automatically evicts oldest move │
└───────────────────────────────────┴────────────────────────────────────┘
```

When requesting games via `query_games`:
* The client can optionally pass `"owner": "main"` or `"owner": "reference"`.
* If omitted, `scid-mgr` resolves against `main_session` first, then falls back to `reference_sessions`.
* If an explicit owner is provided and mismatched, an explicit error is returned (e.g. `"Search session 'ref_1' not found for owner 'main'"`).

---

## 4. The 2-Phase Communication Protocol

### Phase 1: Initiating the Search

The client sends one of three search commands depending on user interaction:

#### Option A: CQL Language Search (`search`)
```json
{
  "id": 1,
  "command": "search",
  "params": {
    "query": "move from B to _ and attacks(Q, k)"
  }
}
```

#### Option B: Direct Position / FEN Search (`search_position`)
```json
{
  "id": 1,
  "command": "search_position",
  "params": {
    "fen": "r1bqk2r/pppp1ppp/2n5/4p3/2B1n3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4",
    "match_mode": "exact",
    "max_ply": 40
  }
}
```

#### Option C: Material Search (`search_material`)
```json
{
  "id": 1,
  "command": "search_material",
  "params": {
    "white_rooks": 1,
    "black_rooks": 1,
    "opposite_bishops": true
  }
}
```

#### Server Response (Phase 1)
The server returns immediate execution diagnostics and the `search_id`:
```json
{
  "id": 1,
  "status": "ok",
  "data": {
    "search_id": "search_1",
    "total_searched": 150000,
    "matched_count": 42,
    "duration_ms": 14,
    "cached": false
  },
  "error": null
}
```

---

### Phase 2: Paginated & Sorted Game Retrieval (`query_games`)

Once the GUI has the `search_id`, it requests pages of games as the user scrolls the table or changes sort criteria:

#### Request
```json
{
  "id": 2,
  "command": "query_games",
  "params": {
    "search_id": "search_1",
    "page": 0,
    "page_size": 50,
    "sort_by": "date",
    "sort_asc": false
  }
}
```

#### Server Response (Phase 2)
The server resolves game header metadata **only for the requested 50-game slice** and injects `matching_plies` and `match_count`:

```json
{
  "id": 2,
  "status": "ok",
  "data": {
    "page": 0,
    "page_size": 50,
    "total": 42,
    "search_id": "search_1",
    "sort_by": "date",
    "sort_asc": false,
    "games": [
      {
        "id": 1042,
        "white": "Morphy, Paul",
        "black": "Duke of Brunswick / Count Isouard",
        "date": "1858.??.??",
        "result": "1-0",
        "eco": "C41",
        "event": "Paris",
        "site": "Paris Opera House",
        "white_elo": 0,
        "black_elo": 0,
        "round": "1",
        "matching_plies": [21, 33],
        "match_count": 2
      },
      {
        "id": 48201,
        "white": "Kasparov, Garry",
        "black": "Topalov, Veselin",
        "date": "1999.01.20",
        "result": "1-0",
        "eco": "B07",
        "event": "Wijk aan Zee",
        "site": "Wijk aan Zee NED",
        "white_elo": 2812,
        "black_elo": 2700,
        "round": "4",
        "matching_plies": [47],
        "match_count": 1
      }
    ]
  },
  "error": null
}
```

---

## 5. Frontend GUI Implementation Guide

### A. Game Table Display
1. Configure table model with `total` matching games.
2. Render standard headers (`White`, `Black`, `Date`, `ECO`, `Result`, etc.).
3. Optionally display a **Matches** column showing `game.match_count` (e.g. `2`).

### B. Board & Move-List Navigation using `matching_plies`
When a user clicks on a game row in the GUI:

1. **Fetch PGN**: Request standard PGN text via `get_pgn` with `"index": game.id`.
2. **Decode Moves**: Load the move list into the board controller.
3. **Auto-Jump to Match**: Jump the active move cursor directly to the first match:
   ```python
   # Example Python (PyQt5) Game Navigation
   target_ply = game["matching_plies"][0] if game.get("matching_plies") else 0
   board_widget.goto_ply(target_ply)
   ```
4. **Multi-Match Stepping**: If `game["matching_plies"].length > 1` (e.g. `[21, 33]`):
   * Provide **Next Match (▶)** and **Prev Match (◀)** buttons in the UI.
   * Clicking Next/Prev cycles through `game["matching_plies"]`, moving the board forward or backward directly to the next matching tactic or position.

---

## 6. Summary of Supported Sorting Fields in `query_games`

| `sort_by` Value | Description |
| :--- | :--- |
| `"date"` | Chronological game date (YYYY.MM.DD) |
| `"white"` | White player name (alphabetical via rank tables) |
| `"black"` | Black player name (alphabetical via rank tables) |
| `"white_elo"` | White ELO rating (descending / ascending) |
| `"black_elo"` | Black ELO rating (descending / ascending) |
| `"eco"` | ECO opening code (e.g. `B90`) |
| `"result"` | Game result (`1-0`, `0-1`, `1/2-1/2`) |
| `"event"` | Tournament / Event name |
| `"site"` | Location / Site name |
| `"id"` / `"index"` | Natural database index order |
| `"matches"` / `"match_count"` | Number of matching plies in the game |
| `"first_ply"` / `"ply"` | First ply number where the match occurred |

---

## 7. Frontend Widget Wiring & Best Practices

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Desktop GUI Application                          │
├──────────────────────────────────────┬──────────────────────────────────────┤
│       Main Database Games Grid       │     Reference Explorer / Tree Tab    │
│  (e.g. QTableView / Virtual Scroll)  │      (Live Interactive Board)        │
├──────────────────────────────────────┼──────────────────────────────────────┤
│ 1. User sets Filter / CQL query      │ 1. User plays move on board (FEN)    │
│ 2. `search` -> receives `main_1`     │ 2. `search_position` -> `ref_1`      │
│ 3. `query_games(search_id: "main_1", │ 3. `query_games(search_id: "ref_1",  │
│                 owner: "main")`      │                 owner: "reference")` │
│ 4. Retained permanently while active │ 4. Keeps last 3 moves in LRU pool    │
└──────────────────────────────────────┴──────────────────────────────────────┘
```

### Key Principles for GUI Developers & AI Agents

1. **Explicit Owner Parameterization**:
   - When querying games for the main database table, pass `"owner": "main"`.
   - When querying games for the live opening tree or board reference explorer, pass `"owner": "reference"`.
   - This ensures the server validates that the session belongs to the expected view, providing clear error diagnosis if a stale or mismatched ID is used.

2. **Main Table Persistence**:
   - The main database search is held in **1 dedicated slot**. It is never pushed into an LRU queue and will **never** be evicted regardless of how many hundreds of moves the user clicks through in the Opening Tree or Reference Explorer.

3. **Reference Pool LRU Lifecycle**:
   - The Reference Explorer LRU pool holds the **3 most recent board positions**.
   - Navigating forward and backward across the last 3 positions reuses cached results with 0 ms latency.
   - If an older position is evicted and requested, the server responds with `{ "status": "error", "error": "Search session 'ref_X' not found in reference explorer" }`. The frontend simply sends a fresh `search_position` request.

### Example Integration Blueprint (Python / PyQt)

```python
class ChessGuiController:
    def __init__(self, rpc_client):
        self.rpc = rpc_client
        self.main_search_id = None
        self.reference_search_id = None

    def apply_main_database_filter(self, cql_query: str):
        """Applies a global filter to the main database grid view."""
        resp = self.rpc.send_command("search", {"query": cql_query})
        if resp["status"] == "ok":
            self.main_search_id = resp["data"]["search_id"]
            self.load_main_table_page(page=0)

    def load_main_table_page(self, page: int, page_size: int = 50, sort_by: str = "date"):
        """Loads a paginated slice for the main database table."""
        resp = self.rpc.send_command("query_games", {
            "search_id": self.main_search_id,
            "owner": "main",
            "page": page,
            "page_size": page_size,
            "sort_by": sort_by,
            "sort_asc": False
        })
        if resp["status"] == "ok":
            self.main_grid.update_rows(resp["data"]["games"], total=resp["data"]["total"])

    def on_board_move_changed(self, fen: str):
        """Triggered when the user navigates moves in the Reference Explorer."""
        resp = self.rpc.send_command("search_position", {
            "fen": fen,
            "match_mode": "exact"
        })
        if resp["status"] == "ok":
            self.reference_search_id = resp["data"]["search_id"]
            self.load_reference_games(page=0)

    def load_reference_games(self, page: int):
        """Loads games for the current Reference Explorer board position."""
        resp = self.rpc.send_command("query_games", {
            "search_id": self.reference_search_id,
            "owner": "reference",
            "page": page,
            "page_size": 20
        })
        if resp["status"] == "ok":
            self.reference_list.update_rows(resp["data"]["games"])
        elif "not found" in resp.get("error", ""):
            # Session was evicted from LRU; re-query position
            self.on_board_move_changed(self.current_board_fen)
```

