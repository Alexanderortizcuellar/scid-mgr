# ==============================================================================
# Chess Mating Themes Catalog (Translated from CQL to scid-mgr Search DSL)
# ==============================================================================

# 1. Smothered Mate
# A knight delivers mate to a king completely hemmed in by its own pieces (zero empty flight squares).
smothered_mate = checkmate and attacks(N, k) and not attacks(k, empty)

# 2. Arabian Mate
# The knight and rook team up to trap the opposing king in a corner of the board.
arabian_mate = checkmate and attacks(R, k) and attacks(N, R) and attacks(k, R) and piece k on [a8, h8, a1, h1, b8, g8, a7, h7, b1, g1, a2, h2]

# 3. Anastasia's Mate
# Knight and rook trap the opposing king on the edge of the board with a friendly blocker.
anastasia_mate = checkmate and attacks(R, k) and piece N count >= 1 and piece k on [a1-a8, h1-h8, a1-h1, a8-h8]

# 4. Back Rank Mate
# Rook or queen checkmates the king trapped on its back rank by friendly pawns.
back_rank_mate = checkmate and piece k on [a8-h8] and piece [R, Q] on [a8-h8]

# 5. Boden's Mate
# Two criss-crossing bishops deliver checkmate to a king whose flight squares are blocked.
boden_mate = checkmate and attacks(B, k) and piece B count == 2

# 6. Hook Mate
# A rook checkmates protected by a knight, with the knight protected by a pawn.
hook_mate = checkmate and attacks(R, k) and attacks(N, R) and attacks(P, N) and attacks(k, R)

# 7. Opera Mate (Morphy's checkmate)
# Rook delivers mate on back rank protected by bishop against an uncastled king.
opera_mate = checkmate and attacks(R, k) and attacks(B, R) and attacks(k, R) and piece k on [d8, e8]

# 8. Reti's Mate
# Bishop delivers mate protected by a rook or other piece in the center/flank.
reti_mate = checkmate and attacks(B, k) and attacks(R, B) and attacks(k, B)

# 9. Suffocation Mate
# A knight delivers checkmate while a bishop controls/suffocates escape squares.
suffocation_mate = checkmate and attacks(N, k) and piece B count >= 1

# 10. Vukovic Mate
# Rook delivers mate on the edge protected by king or pawn while knight covers escape squares.
vukovic_mate = checkmate and attacks(R, k) and piece N count >= 1 and piece k on [a1-a8, h1-h8, a8-h8, a1-h1]

# 11. Ladder (Lawnmower) Mate
# Two major pieces work together to drive the king to the edge and checkmate.
ladder_mate = checkmate and piece [R, Q] count >= 2 and piece k on [a1-a8, h1-h8, a8-h8, a1-h1] and attacks(R, k)

# 12. Dovetail (Cozio) Mate
# Queen delivers mate adjacent to the king, whose only two escape squares are blocked by friendly pieces.
dovetail_mate = checkmate and attacks(Q, k) and attacks(k, Q)

# 13. Swallowtail Mate
# Queen checkmates enemy king on edge, flanked on either side behind by friendly pieces.
swallowtail_mate = checkmate and attacks(Q, k) and piece k on [a1-a8, h1-h8, a8-h8, a1-h1]

# 14. Balestra Mate
# Bishop delivers checkmate while queen cuts off all lateral and vertical escape squares.
balestra_mate = checkmate and attacks(B, k) and piece Q count >= 1

# 15. Blackburne's Mate
# Two bishops and a knight team up to checkmate the king trapped on the kingside.
blackburne_mate = checkmate and attacks(B, k) and piece B count == 2 and piece N count >= 1

# 16. Blind Swine Mate
# Two rooks on the 7th rank checkmate the opposing king on the 8th rank.
blind_swine_mate = checkmate and piece R count >= 2 and piece k on [a8-h8] and piece R on [a7-h7]

# 17. Damiano's Mate
# Queen checkmates on the 7th/8th rank supported by a pawn or bishop.
damiano_mate = checkmate and attacks(Q, k) and piece [P, B] count >= 1 and piece k on [g8, h8, g1, h1]

# 18. Damiano's Bishop Mate
# Queen delivers checkmate supported by a bishop along the long diagonal.
damiano_bishop_mate = checkmate and attacks(Q, k) and attacks(B, Q)

# 19. David and Goliath Mate
# A humble pawn delivers the final checkmate blow against the enemy king.
david_and_goliath_mate = checkmate and attacks(P, k)

# 20. Greco's Mate
# Bishop and rook/queen trap the enemy king in the corner with a friendly pawn blocker.
greco_mate = checkmate and attacks(R, k) and piece B count >= 1 and piece k on [h8, h1, a8, a1]

# 21. Legal's Mate
# Two knights and a bishop checkmate the opponent king in the center.
legal_mate = checkmate and attacks(N, k) and piece B count >= 1 and piece N count >= 2

# 22. Morphy's Mate
# Bishop checkmates while rook cuts off the open file pinning the king to the corner.
morphy_mate = checkmate and attacks(B, k) and piece R count >= 1 and piece k on [h8, h1, a8, a1]

# 23. Pillsbury's Mate
# Rook delivers checkmate on the open corner file while bishop controls diagonal flight squares.
pillsbury_mate = checkmate and attacks(R, k) and piece B count >= 1 and piece k on [h8, h1]

# 24. Triangle Mate
# Queen and rook coordinate in a triangular geometry to corner and mate the king.
triangle_mate = checkmate and attacks(Q, k) and piece R count >= 1

# 25. Two Bishops Mate
# Two bishops deliver checkmate to a bare king on the edge of the board.
two_bishops_mate = checkmate and attacks(B, k) and piece B count == 2 and piece k on [a1-a8, h1-h8, a8-h8, a1-h1]

# 26. Killbox Mate
# Rook delivers mate protected by a queen/rook creating a 3x3 killbox constraint.
killbox_mate = checkmate and attacks(R, k) and piece [Q, R] count >= 2

# 27. Corner Mate
# Knight or minor piece mates king confined strictly to a1, a8, h1, or h8 corner.
corner_mate = checkmate and piece k on [a1, a8, h1, h8] and attacks(N, k)

# 28. Anderssen's Mate
# Rook or queen checkmates supported by a pawn or diagonally distant piece on the back rank.
anderssen_mate = checkmate and attacks(R, k) and piece P count >= 1 and piece k on [a8-h8]

# 29. Max Lange's Mate
# Bishop and queen work in tandem with a pawn wedge to checkmate the enemy king.
max_lange_mate = checkmate and attacks(B, k) and piece Q count >= 1 and piece P count >= 1

# 30. Mayet's Mate
# Rook delivers checkmate on the back rank supported by a distant bishop on a long diagonal.
mayet_mate = checkmate and attacks(R, k) and piece B count >= 1
