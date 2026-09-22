# 🏷️ Chapter 1: Header Filters & Metadata

Header filters query game metadata, player names, tournament information, ratings, and custom PGN/SCID tags.

---

## 📌 Keywords & Syntax

| Keyword | Description | Supported Operators | Example |
| :--- | :--- | :--- | :--- |
| **`player`** | Matches either White or Black player | `==`, `!=`, `contains`, `has`, `startswith`, `endswith`, `~` (regex), `:` | `player "Kasparov"` |
| **`white`** | White player name | `==`, `!=`, `contains`, `has`, `startswith`, `endswith`, `~`, `:` | `white "Carlsen, Magnus"` |
| **`black`** | Black player name | `==`, `!=`, `contains`, `has`, `startswith`, `endswith`, `~`, `:` | `black "Nakamura"` |
| **`white_elo`** / **`whiteelo`** | White's Elo rating | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `white_elo >= 2750` |
| **`black_elo`** / **`blackelo`** | Black's Elo rating | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `black_elo >= 2700` |
| **`elo`** / **`any_elo`** / **`anyelo`** | Either player's Elo rating | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `elo >= 2800` |
| **`avg_elo`** / **`avgelo`** | Average Elo of both players | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `avg_elo >= 2650` |
| **`elo_diff`** / **`elodiff`** | Absolute Elo difference `\|White - Black\|` | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `elo_diff >= 200` |
| **`raw_elo_diff`** | Signed Elo difference `(White - Black)` | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `raw_elo_diff >= 100` |
| **`result`** | Game outcome (`"1-0"`, `"0-1"`, `"1/2-1/2"`, `"*"`) | `==`, `!=`, `:` | `result "1-0"` |
| **`eco`** | ECO opening code prefix or pattern | `==`, `!=`, `startswith`, `has`, `~`, `:` | `eco "B80"` or `eco startswith "E"` |
| **`date`** | Date or year range (`YYYY`, `YYYY.MM`, `YYYY.MM.DD`) | `==`, `!=`, `>=`, `<=`, `>`, `<`, `:` | `date >= "2015"` |
| **`event`** | Tournament / event name | `==`, `!=`, `contains`, `has`, `startswith`, `~`, `:` | `event "World Championship"` |
| **`site`** | Venue / location | `==`, `!=`, `contains`, `has`, `startswith`, `~`, `:` | `site "Wijk aan Zee"` |
| **`round`** | Round number/identifier | `==`, `!=`, `contains`, `:` | `round "1"` |
| **`tag`** / **`header`** / **`custom`** | Arbitrary standard or custom PGN / SCID tag | `==`, `!=`, `contains`, `has`, `~`, `:` | `tag "Annotator" contains "Nunn"` |

---

## 💡 String & Comparison Operators

| Operator | Meaning | Example |
| :--- | :--- | :--- |
| **`==`** or **`:`** | Exact equality (case-insensitive by default) | `white == "Kasparov"` |
| **`!=`** | Not equal to | `result != "1/2-1/2"` |
| **`contains`** or **`has`** | Substring search | `event contains "Candidates"` |
| **`startswith`** | Prefix match | `eco startswith "B"` |
| **`endswith`** | Suffix match | `white endswith "ov"` |
| **`~`**, **`=~`**, or **`regex(...)`** | Regular expression match (PCRE/Rust regex) | `white ~ "alex|pedro"` |

---

## 🔍 Regular Expression Matching (`~` / `=~` / `regex`)

Header filters support full regular expression pattern matching via the **`~`**, **`=~`**, or **`regex(...)`** operators across all text headers (`player`, `white`, `black`, `event`, `site`, `eco`, and `tag`).

### Key Regex Patterns & Features:
* **Alternation / Multi-name search (`|`)**:
  * `white ~ "alex|pedro"` — Matches White player named either "alex" or "pedro".
  * `player ~ "Carlsen|Kasparov|Fischer"` — Matches games where Carlsen, Kasparov, or Fischer played on either side.
* **Anchor & Prefix/Suffix Patterns (`^`, `$`)**:
  * `white ~ "^Kasparov"` — Player name starting strictly with Kasparov.
  * `black ~ "ov$"` — Player name ending in "ov".
* **Character Sets and Wildcards (`.*`, `[0-9]`)**:
  * `tag "TimeControl" ~ "180\+.*"` — Time controls starting with 180s (3+0, 3+1, 3+2).
  * `event ~ ".*Candidates.*(2022|2024)"` — Candidates tournament from 2022 or 2024.
* **Alternative Functional Syntax (`regex`)**:
  * `white regex("alex|pedro")`
  * `tag "Annotator" regex("Stockfish [0-9]+")`

---

## 🎯 Verified Examples

### 1. High-Rated Decisive Games
```text
avg_elo >= 2700 and result != "1/2-1/2" and date >= "2018"
```

### 2. Specific Player Matches with Regex Alternation
```text
player ~ "Kasparov|Karpov" and eco startswith "E" and date >= "1984" and date <= "1990"
```

### 3. Multiple Target Players
```text
white ~ "alex|pedro" or black ~ "alex|pedro"
```

### 4. Big Rating Upset (White Rated 300+ Points Higher but Lost)
```text
raw_elo_diff >= 300 and result "0-1"
```

### 5. Custom Tags (TimeControl, Annotator, FEN)
```text
tag "TimeControl" ~ "300\+.*"
header "Annotator" contains "Stockfish"
tag "Variant" != "Standard"
```
