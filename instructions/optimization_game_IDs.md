We are continuing the existing SCID position-index implementation. **Do not start over or redesign the existing index format.** The current implementation already has the position index, position/move structures, and the game-ID collection logic. The delta + varint encoding for sorted game IDs has already been implemented and tested.

The next task is to optimize the storage of the game-ID arrays because they can become very large when indexing millions of games.

## Goal

Introduce an adaptive `GameSet` encoding that can represent a sorted collection of game IDs using either:

1. The existing **delta + varint** encoding.
2. A **Roaring Bitmap** representation.

The index builder should calculate which representation is smaller for each game-ID set and automatically choose the more compact representation.

The rest of the application should not need to care which physical representation was selected.

---

## 1. Keep the existing delta/varint implementation

Do not remove or rewrite the existing delta + varint encoder.

The current flow already produces something conceptually like:

```text
sorted game IDs:
[1020, 1025, 1031, 1040, 1055]

delta:
[1020, 5, 6, 9, 15]

varint encode:
[first ID + varints for the deltas]
```

Keep this implementation as one possible backend for `GameSet`.

If the current functions have appropriate names, reuse them rather than creating duplicate implementations.

---

## 2. Introduce an explicit GameSet encoding type

Create a small enum/flag representing the physical representation.

For example:

```rust
enum GameSetType {
    DeltaVarint = 0,
    Roaring = 1,
}
```

Use the actual naming conventions already present in the codebase.

The serialized game-set payload should contain enough information for the decoder to know which representation is being used.

Conceptually:

```text
GameSet
├── encoding_type
└── encoded_data
```

The encoding type should be a compact value, ideally one byte.

Do not use strings such as `"delta"` or `"roaring"` in the binary format.

---

## 3. Create a unified GameSet abstraction

The rest of the index/query code should work against a logical game set rather than knowing whether it is delta encoded or Roaring.

The abstraction should support at least:

```text
count()
contains(game_id)
iter()
```

And ideally operations useful for filtering:

```text
intersection_count(other)
intersect(other)
```

The exact Rust API should follow the architecture already present in the project.

The important architectural principle is:

```text
Position / Move
       |
       v
    GameSet
       |
       +---- DeltaVarint
       |
       +---- Roaring
```

The caller should not have to manually decode one format versus the other.

---

## 4. Implement Roaring Bitmap support

Add Roaring Bitmap support using an appropriate Rust implementation/library if the project does not already contain one.

Do not implement the entire Roaring Bitmap specification manually unless there is a strong existing reason to do so.

The important requirement is that the Roaring representation must be serializable into the SCID position-index file and reconstructable when reading the index.

We need a deterministic binary representation:

```text
GameSetType::Roaring
    +
serialized roaring data
```

Make sure the serialized representation can be decoded without ambiguity.

The game IDs are integer IDs in the range used by the SCID database, so use the smallest appropriate integer representation supported by the existing data model.

---

## 5. Choose the representation by actual encoded size

This is the most important part.

Do NOT introduce an arbitrary rule such as:

```text
if game_count > 10,000:
    use Roaring
else:
    use DeltaVarint
```

Instead, the index builder should evaluate the actual storage cost.

Conceptually:

```rust
let delta_data = encode_delta_varint(&game_ids);
let roaring_data = encode_roaring(&game_ids);

if roaring_data.len() < delta_data.len() {
    use Roaring
} else {
    use DeltaVarint
}
```

The exact implementation can optimize allocations later, but correctness and clear architecture come first.

The builder should effectively ask:

> "For this exact set of game IDs, which representation requires fewer bytes?"

Then store the smaller one.

Include the encoding-type byte in the size calculation if appropriate.

For example:

```text
delta_size   = 1 byte type + delta payload
roaring_size = 1 byte type + roaring payload
```

Choose the smaller total.

This makes the encoding adaptive to both:

* number of games
* distribution/density of game IDs

rather than relying on a magic threshold.

---

## 6. Important: game IDs must remain complete

Do NOT reduce the game-ID arrays to samples.

We need the complete membership information because the position index is not only for unfiltered Opening Explorer statistics.

The game IDs will also be used for filtered searches such as:

```text
position + player
position + year
position + result
position + date range
position + Elo
```

Therefore, if a position/move was reached by:

```text
400,000 games
```

the index must retain membership for all 400,000 games.

The optimization is only about **how those IDs are represented**, not about throwing IDs away.

---

## 7. Preserve the distinction between position and move membership

Be careful not to accidentally collapse these two concepts:

```text
Position game set
    = games that reached the position

Move game set
    = games that played this specific move from the position
```

They are not necessarily identical.

For example:

```text
Position:
    1,000,000 games

e4:
      600,000 games

d4:
      300,000 games

Nf3:
      100,000 games
```

If the current index architecture stores game sets for moves, those sets must remain move-specific.

Do not replace move membership with the parent position's membership.

Likewise, do not add duplicate copies of the position hash or other data unnecessarily.

---

## 8. Reading/decoding

Update the reader so it can transparently decode:

```text
GameSetType::DeltaVarint
```

and

```text
GameSetType::Roaring
```

The caller should simply request the logical game set.

For example, conceptually:

```rust
let game_set = read_game_set(...);

for game_id in game_set.iter() {
    ...
}
```

The caller should not need code like:

```rust
if type == Delta {
    ...
} else {
    ...
}
```

unless that distinction is required internally by the implementation.

---

## 9. Filtering should take advantage of the representation

The main reason for introducing Roaring is not only smaller files.

We also want efficient set operations.

The eventual query flow is:

```text
Position game set
        AND
Active filter
        =
Matching games
```

For example:

```text
position_games
AND
games_where_player_is_Carlsen
AND
games_from_2024
```

The active filter can itself eventually be represented as a bitmap/set.

Design `GameSet` so that these operations can be optimized later.

For example:

```rust
intersection_count(...)
```

should avoid unnecessarily materializing a huge list when possible.

If both sides are Roaring, use Roaring's native intersection operations.

If one side is delta/varint, decode/iterate it efficiently and test membership against the other set.

Do not over-engineer the full query engine in this task. Establish the abstraction and the correct primitives so that filtering can be optimized later.

---

## 10. Do not decode everything unnecessarily

The position index can contain millions of positions and potentially hundreds of millions of game-ID references.

We must NOT decode every GameSet when opening the database.

Only decode/read the GameSet for the position/move currently being queried.

The index lookup should remain:

```text
position hash
    -> position record
        -> relevant GameSet
```

The representation choice is per GameSet.

---

## 11. Serialization format

Integrate the encoding into the existing binary format rather than creating a separate file format.

Do not duplicate:

* position hash
* game count
* move data
* headers
* other metadata

unless the current format requires it.

The GameSet should be an encoded component of the existing position/move payload.

Keep the binary format compact.

If the existing format already has a version field, use it appropriately so that introducing the new GameSet encoding does not create ambiguity when reading older indexes.

Do not silently make an old index unreadable without a version check.

---

## 12. Backward compatibility

Inspect the existing index versioning mechanism.

If the current index format cannot distinguish the new GameSet encoding from the previous format, increment the format version.

The reader should fail clearly for unsupported versions rather than interpreting bytes using the wrong schema.

Do not unnecessarily break unrelated parts of the existing format.

---

## 13. Tests

Add tests for at least:

### Delta encoding

```text
[]
[1]
[1, 2]
[100, 105, 110]
[0, 1, 2, 1000000]
```

### Roaring encoding

Use sparse and dense examples.

For example:

```text
[1, 500000, 999999]
```

and:

```text
[0, 1, 2, 3, ..., 499999]
```

### Automatic selection

Verify that:

```text
selected_encoding
```

is actually the representation with the smaller serialized size.

Do not assert that Roaring is always selected for large sets or DeltaVarint is always selected for small sets. The test should compare the actual encoded sizes.

### Round-trip

For both encodings:

```text
original IDs
    -> encode
    -> serialize
    -> read
    -> decode
    -> IDs
```

must produce exactly the original sorted IDs.

### Membership

Verify:

```text
contains(existing_id) == true
contains(non_existing_id) == false
```

### Iteration

Verify that iteration produces:

```text
exactly the same sorted game IDs
```

as the original input.

### Intersection

Test combinations:

```text
Delta ∩ Delta
Delta ∩ Roaring
Roaring ∩ Delta
Roaring ∩ Roaring
```

and verify that the result is correct.

---

## 14. Add instrumentation/diagnostics to the index builder

While developing this feature, make it possible to report statistics such as:

```text
Total game sets:        X
DeltaVarint selected:   X
Roaring selected:       X

Bytes if all Delta:     X
Bytes if all Roaring:   X
Bytes with adaptive:    X

Space saved:            X%
```

This is important because we want to measure whether the optimization actually helps on the real chess databases.

If practical, also report distributions such as:

```text
Game-set size:
1–10
11–100
101–1,000
1,001–10,000
10,001–100,000
100,001+
```

This will help us understand the real data instead of guessing thresholds.

---

## 15. Important performance consideration

The first implementation may encode both representations in order to compare their sizes.

That is acceptable.

For example:

```text
collect sorted IDs
        |
        +--> delta/varint encode
        |
        +--> roaring encode
        |
        +--> compare sizes
        |
        +--> write smaller representation
```

Do not prematurely optimize this builder path.

The index is built offline, so spending additional CPU during index creation is acceptable if it produces a significantly smaller index.

The query/read path is much more important.

Once the implementation is correct, we can optimize the builder if necessary.

---

## 16. Important future optimization: complement sets

Do not necessarily implement this in the first pass, but design the abstraction so that another encoding could eventually be added.

For extremely dense sets, for example:

```text
1,000,000 total games
990,000 games contain position
```

it may be smaller to store:

```text
AllGamesExcept
    10,000 excluded IDs
```

instead of storing the 990,000 included IDs.

Therefore, avoid designing the enum in a way that makes future encodings difficult.

Conceptually:

```rust
enum GameSetType {
    DeltaVarint = 0,
    Roaring = 1,
    // Future:
    // ComplementDelta = 2,
    // ...
}
```

Do NOT implement the complement representation yet unless it is trivial in the existing architecture.

---

## 17. Keep the implementation focused

This task is specifically about:

> optimizing the storage and access of the existing complete game-ID arrays.

Do not redesign:

* the position hash/index structure
* the SCID database
* the Opening Explorer
* the header index
* the Stockfish integration
* the PGN parser

Those systems already exist.

Make the smallest clean architectural change necessary to introduce:

```text
GameSet
    |
    +-- DeltaVarint
    |
    +-- Roaring
```

with automatic size-based selection.

At the end, summarize:

1. Which files were changed.
2. The new GameSet abstraction.
3. The binary layout added/changed.
4. How the encoding selection works.
5. How decoding works.
6. Test results.
7. File-size comparison on an existing real database, if available.

Most importantly: **preserve the existing working delta/varint implementation and build this as the next layer on top of it.**
