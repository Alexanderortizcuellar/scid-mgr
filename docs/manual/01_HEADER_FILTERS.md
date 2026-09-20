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
| **`~`** or **`regex(...)`** | Regular expression match | `player ~ "(?i)alexander.*"` |

---

## 🎯 Verified Examples

### 1. High-Rated Decisive Games
```text
avg_elo >= 2700 and result != "1/2-1/2" and date >= "2018"
```

### 2. Specific Player Matches with Regex
```text
player ~ "Kasparov|Karpov" and eco startswith "E" and date in "1984".."1990"
```

### 3. Big Rating Upset (White Rated 300+ Points Higher but Lost)
```text
raw_elo_diff >= 300 and result "0-1"
```

### 4. Custom Tags (TimeControl, Annotator, FEN)
```text
tag "TimeControl" == "300+0"
header "Annotator" contains "Stockfish"
tag "Variant" != "Standard"
```
