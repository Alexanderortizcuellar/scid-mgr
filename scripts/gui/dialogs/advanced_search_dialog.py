import chess
from PyQt5.QtCore import Qt
from PyQt5.QtGui import QFont
from PyQt5.QtWidgets import (
    QDialog, QVBoxLayout, QHBoxLayout, QGridLayout, QLabel, QLineEdit,
    QPushButton, QComboBox, QCheckBox, QTabWidget, QWidget, QRadioButton,
    QButtonGroup, QFrame, QScrollArea
)
from ..widgets.board_widget import ChessBoardEditorWidget

class AdvancedSearchDialog(QDialog):
    """
    ChessBase-style Advanced Multi-Tab Search Dialog.
    Tabs:
      1. 🏷️ Game Info (Players, Result, ECO, Date, Event, Site, Status)
      2. ♟️ Position / Board (Visual Board Editor, FEN string, Depth / Max Ply, Presets)
      3. ⚖️ Material (Piece counts for White & Black, Presets, Scope mode)
    """
    def __init__(self, current_filter: Optional[dict] = None, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Advanced Search (ChessBase style)")
        self.resize(800, 680)

        main_layout = QVBoxLayout(self)
        self.tabs = QTabWidget()
        main_layout.addWidget(self.tabs)

        # TAB 1: Game Info
        self.tab_info = QWidget()
        info_layout = QGridLayout(self.tab_info)
        info_layout.setContentsMargins(15, 15, 15, 15)
        info_layout.setSpacing(10)

        info_layout.addWidget(QLabel("Player (Any):"), 0, 0)
        self.in_player = QLineEdit()
        self.in_player.setPlaceholderText("e.g. Carlsen or Kasparov")
        info_layout.addWidget(self.in_player, 0, 1)

        info_layout.addWidget(QLabel("Result:"), 0, 2)
        self.in_result = QComboBox()
        self.in_result.addItems(["All", "1-0", "0-1", "1/2-1/2", "*"])
        info_layout.addWidget(self.in_result, 0, 3)

        info_layout.addWidget(QLabel("White:"), 1, 0)
        self.in_white = QLineEdit()
        info_layout.addWidget(self.in_white, 1, 1)

        info_layout.addWidget(QLabel("Black:"), 1, 2)
        self.in_black = QLineEdit()
        info_layout.addWidget(self.in_black, 1, 3)

        info_layout.addWidget(QLabel("ECO Code:"), 2, 0)
        self.in_eco = QLineEdit()
        self.in_eco.setPlaceholderText("e.g. B90 or E97")
        info_layout.addWidget(self.in_eco, 2, 1)

        info_layout.addWidget(QLabel("Date / Year:"), 2, 2)
        self.in_date = QLineEdit()
        self.in_date.setPlaceholderText("e.g. 2024 or 1999.01")
        info_layout.addWidget(self.in_date, 2, 3)

        info_layout.addWidget(QLabel("Event:"), 3, 0)
        self.in_event = QLineEdit()
        info_layout.addWidget(self.in_event, 3, 1)

        info_layout.addWidget(QLabel("Site:"), 3, 2)
        self.in_site = QLineEdit()
        info_layout.addWidget(self.in_site, 3, 3)

        del_box = QGroupBox("Status Filter")
        del_box_layout = QHBoxLayout(del_box)
        self.chk_include_del = QCheckBox("Include Deleted Games")
        self.chk_include_del.setChecked(True)
        del_box_layout.addWidget(self.chk_include_del)
        self.chk_only_del = QCheckBox("Only Deleted Games")
        del_box_layout.addWidget(self.chk_only_del)
        info_layout.addWidget(del_box, 4, 0, 1, 4)

        self.tabs.addTab(self.tab_info, "🏷️ Game Info")

        # TAB 2: Position (Board Editor & FEN)
        self.tab_pos = QWidget()
        pos_layout = QHBoxLayout(self.tab_pos)
        pos_layout.setContentsMargins(12, 12, 12, 12)
        pos_layout.setSpacing(15)

        # Left Column: Visual Chess Board Editor
        board_col = QVBoxLayout()
        self.board_editor = ChessBoardEditorWidget(self)
        self.board_editor.fen_changed.connect(self.on_board_fen_changed)
        board_col.addWidget(self.board_editor)

        board_btn_row = QHBoxLayout()
        btn_clear_b = QPushButton("Clear Board (Empty)")
        btn_clear_b.clicked.connect(self.board_editor.clear_board)
        board_btn_row.addWidget(btn_clear_b)

        btn_init_b = QPushButton("Initial Position")
        btn_init_b.clicked.connect(self.board_editor.reset_to_initial)
        board_btn_row.addWidget(btn_init_b)

        btn_qd4 = QPushButton("Queen on d4 (Demo)")
        btn_qd4.clicked.connect(lambda: self.set_single_piece_demo(chess.QUEEN, chess.WHITE, chess.D4))
        board_btn_row.addWidget(btn_qd4)

        board_col.addLayout(board_btn_row)
        pos_layout.addLayout(board_col, 0)

        # Right Column: Position Settings & Presets
        controls_col = QVBoxLayout()
        controls_col.setSpacing(10)

        controls_col.addWidget(QLabel("<b>Board FEN / Piece Placement:</b>"))
        self.in_fen = QLineEdit()
        self.in_fen.setPlaceholderText("e.g. 8/8/8/8/3Q4/8/8/8 or full FEN")
        self.in_fen.textChanged.connect(self.on_fen_text_edited)
        controls_col.addWidget(self.in_fen)

        depth_row = QHBoxLayout()
        depth_row.addWidget(QLabel("Max Search Ply (Depth):"))
        self.spin_max_ply = QSpinBox()
        self.spin_max_ply.setRange(1, 1000)
        self.spin_max_ply.setValue(250)
        depth_row.addWidget(self.spin_max_ply)
        depth_row.addStretch()
        controls_col.addLayout(depth_row)

        desc_lbl = QLabel(
            "💡 <i>Tip: Setup any piece pattern above (e.g. a single Queen on d4). "
            "The search engine will find all games where those pieces appear on those squares!</i>"
        )
        desc_lbl.setWordWrap(True)
        desc_lbl.setStyleSheet("color: #555; font-size: 11px; padding: 4px;")
        controls_col.addWidget(desc_lbl)

        preset_box = QGroupBox("Common Position Presets")
        preset_grid = QGridLayout(preset_box)

        btn_start_pos = QPushButton("Standard Start Pos")
        btn_start_pos.clicked.connect(lambda: self.board_editor.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"))
        preset_grid.addWidget(btn_start_pos, 0, 0)

        btn_najdorf = QPushButton("Sicilian Najdorf (6.Be2)")
        btn_najdorf.clicked.connect(lambda: self.board_editor.set_fen("rnbqkb1r/1p2pppp/p2p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6"))
        preset_grid.addWidget(btn_najdorf, 0, 1)

        btn_french = QPushButton("French Defense (3.Nc3)")
        btn_french.clicked.connect(lambda: self.board_editor.set_fen("rnbqkbnr/ppp2ppp/4p3/3p4/3PP3/2N5/PPP2PPP/R1BQKBNR b KQkq - 1 3"))
        preset_grid.addWidget(btn_french, 1, 0)

        btn_italian = QPushButton("Italian Game (Giocco Piano)")
        btn_italian.clicked.connect(lambda: self.board_editor.set_fen("r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4"))
        preset_grid.addWidget(btn_italian, 1, 1)

        btn_qgd = QPushButton("Queen's Gambit Declined")
        btn_qgd.clicked.connect(lambda: self.board_editor.set_fen("rnbqkb1r/ppp2ppp/4pn2/3p4/2PP4/2N5/PP2PPPP/R1BQKBNR w KQkq - 0 4"))
        preset_grid.addWidget(btn_qgd, 2, 0)

        btn_clear_fen = QPushButton("Clear FEN")
        btn_clear_fen.clicked.connect(lambda: self.board_editor.clear_board())
        preset_grid.addWidget(btn_clear_fen, 2, 1)

        controls_col.addWidget(preset_box)
        controls_col.addStretch()
        pos_layout.addLayout(controls_col, 1)

        self.tabs.addTab(self.tab_pos, "♟️ Position / Board")

        # TAB 3: Material Search
        self.tab_mat = QWidget()
        mat_layout = QVBoxLayout(self.tab_mat)
        mat_layout.setContentsMargins(15, 15, 15, 15)
        mat_layout.setSpacing(12)

        # Presets dropdown
        preset_row = QHBoxLayout()
        preset_row.addWidget(QLabel("Material Preset:"))
        self.combo_mat_preset = QComboBox()
        self.combo_mat_preset.addItems([
            "-- Custom Material --",
            "Rook Endgame (R+P vs R+P)",
            "Queen Endgame (Q+P vs Q+P)",
            "Minor Piece Endgame (B vs N)",
            "Queen vs Rook (Q vs R)",
            "Opposite-Colored Bishops (WB=1, BB=1)",
            "Queen Sacrifice / Queenless (WQ=0, BQ=1)",
            "Pawn Endgame (Pawns only)",
            "Reset All Pieces to Any",
        ])
        self.combo_mat_preset.currentIndexChanged.connect(self.on_material_preset_changed)
        preset_row.addWidget(self.combo_mat_preset, 1)
        mat_layout.addLayout(preset_row)

        # Piece Count Matrix
        grid_box = QGroupBox("Exact Piece Counts (Leave 'Any' for unconstrained)")
        grid = QGridLayout(grid_box)
        grid.setSpacing(8)

        pieces = [
            ("♕ Queen", "q"),
            ("♖ Rook", "r"),
            ("♗ Bishop", "b"),
            ("♘ Knight", "n"),
            ("♙ Pawn", "p"),
        ]

        grid.addWidget(QLabel("<b>Color</b>"), 0, 0)
        for col_idx, (pname, _) in enumerate(pieces, start=1):
            grid.addWidget(QLabel(f"<b>{pname}</b>"), 0, col_idx)

        grid.addWidget(QLabel("<b>White:</b>"), 1, 0)
        self.mat_white = {}
        for col_idx, (_, pkey) in enumerate(pieces, start=1):
            cb = QComboBox()
            cb.addItems(["Any"] + [str(i) for i in (range(9) if pkey == 'p' else range(3))])
            self.mat_white[pkey] = cb
            grid.addWidget(cb, 1, col_idx)

        grid.addWidget(QLabel("<b>Black:</b>"), 2, 0)
        self.mat_black = {}
        for col_idx, (_, pkey) in enumerate(pieces, start=1):
            cb = QComboBox()
            cb.addItems(["Any"] + [str(i) for i in (range(9) if pkey == 'p' else range(3))])
            self.mat_black[pkey] = cb
            grid.addWidget(cb, 2, col_idx)

        mat_layout.addWidget(grid_box)

        # Bishop Color Sub-options
        bish_box = QGroupBox("Bishop Color Verification")
        bish_layout = QHBoxLayout(bish_box)
        self.chk_opposite_bishops = QCheckBox("Opposite-Colored Bishops (White & Black on different color squares)")
        self.chk_same_bishops = QCheckBox("Same-Colored Bishops (White & Black on same color squares)")
        self.chk_opposite_bishops.toggled.connect(lambda on: on and self.chk_same_bishops.setChecked(False))
        self.chk_same_bishops.toggled.connect(lambda on: on and self.chk_opposite_bishops.setChecked(False))
        bish_layout.addWidget(self.chk_opposite_bishops)
        bish_layout.addWidget(self.chk_same_bishops)
        mat_layout.addWidget(bish_box)

        # Match Scope Box
        mode_box = QGroupBox("Search Scope")
        mode_layout = QVBoxLayout(mode_box)
        self.rb_final_pos = QRadioButton("Final Position only (Endgames - Ultra fast ~20ms)")
        self.rb_final_pos.setChecked(True)
        mode_layout.addWidget(self.rb_final_pos)
        self.rb_any_move = QRadioButton("Any Move during game (Middlegame / Sacrifices / Combinations ~150ms)")
        mode_layout.addWidget(self.rb_any_move)
        mat_layout.addWidget(mode_box)

        mat_layout.addStretch()
        self.tabs.addTab(self.tab_mat, "⚖️ Material")

        # Bottom Action Buttons
        btn_box = QHBoxLayout()
        btn_reset = QPushButton("🔄 Reset All Filters")
        btn_reset.clicked.connect(self.reset_all)
        btn_box.addWidget(btn_reset)
        btn_box.addStretch()

        btn_cancel = QPushButton("Cancel")
        btn_cancel.clicked.connect(self.reject)
        btn_box.addWidget(btn_cancel)

        btn_search = QPushButton("🔍 Search Games")
        btn_search.setStyleSheet("font-weight: bold; background-color: #1976d2; color: white; padding: 6px 20px;")
        btn_search.clicked.connect(self.accept)
        btn_box.addWidget(btn_search)

        main_layout.addLayout(btn_box)

        if current_filter:
            self.load_filter(current_filter)

    def on_material_preset_changed(self, idx: int):
        if idx == 0:
            return
        
        # Reset all to Any first
        for cb in self.mat_white.values(): cb.setCurrentIndex(0)
        for cb in self.mat_black.values(): cb.setCurrentIndex(0)
        self.chk_opposite_bishops.setChecked(False)
        self.chk_same_bishops.setChecked(False)
        self.rb_final_pos.setChecked(True)

        def set_val(side, pkey, val):
            d = self.mat_white if side == "w" else self.mat_black
            idx = d[pkey].findText(str(val))
            if idx >= 0: d[pkey].setCurrentIndex(idx)

        if idx == 1: # Rook Endgame (R+P vs R+P)
            set_val("w", "q", 0); set_val("w", "r", 1); set_val("w", "b", 0); set_val("w", "n", 0)
            set_val("b", "q", 0); set_val("b", "r", 1); set_val("b", "b", 0); set_val("b", "n", 0)
        elif idx == 2: # Queen Endgame (Q+P vs Q+P)
            set_val("w", "q", 1); set_val("w", "r", 0); set_val("w", "b", 0); set_val("w", "n", 0)
            set_val("b", "q", 1); set_val("b", "r", 0); set_val("b", "b", 0); set_val("b", "n", 0)
        elif idx == 3: # Minor Piece Endgame (B vs N)
            set_val("w", "q", 0); set_val("w", "r", 0); set_val("w", "b", 1); set_val("w", "n", 0)
            set_val("b", "q", 0); set_val("b", "r", 0); set_val("b", "b", 0); set_val("b", "n", 1)
        elif idx == 4: # Queen vs Rook
            set_val("w", "q", 1); set_val("w", "r", 0); set_val("w", "b", 0); set_val("w", "n", 0)
            set_val("b", "q", 0); set_val("b", "r", 1); set_val("b", "b", 0); set_val("b", "n", 0)
        elif idx == 5: # Opposite-Colored Bishops
            set_val("w", "q", 0); set_val("w", "r", 0); set_val("w", "b", 1); set_val("w", "n", 0)
            set_val("b", "q", 0); set_val("b", "r", 0); set_val("b", "b", 1); set_val("b", "n", 0)
            self.chk_opposite_bishops.setChecked(True)
        elif idx == 6: # Queen Sacrifice (WQ=0, BQ=1)
            set_val("w", "q", 0)
            set_val("b", "q", 1)
            self.rb_any_move.setChecked(True)
        elif idx == 7: # Pawn Endgame
            set_val("w", "q", 0); set_val("w", "r", 0); set_val("w", "b", 0); set_val("w", "n", 0)
            set_val("b", "q", 0); set_val("b", "r", 0); set_val("b", "b", 0); set_val("b", "n", 0)

    def on_board_fen_changed(self, fen: str):
        if self.in_fen.text().strip() != fen.strip():
            self.in_fen.blockSignals(True)
            self.in_fen.setText(fen)
            self.in_fen.blockSignals(False)

    def on_fen_text_edited(self, text: str):
        t = text.strip()
        if t:
            self.board_editor.blockSignals(True)
            self.board_editor.set_fen(t)
            self.board_editor.blockSignals(False)

    def set_single_piece_demo(self, role, color, square):
        self.board_editor.clear_board()
        self.board_editor.board.set_piece_at(square, chess.Piece(role, color))
        self.board_editor.update_board_ui()

    def reset_all(self):
        self.in_player.clear()
        self.in_white.clear()
        self.in_black.clear()
        self.in_result.setCurrentIndex(0)
        self.in_eco.clear()
        self.in_date.clear()
        self.in_event.clear()
        self.in_site.clear()
        self.chk_include_del.setChecked(True)
        self.chk_only_del.setChecked(False)
        self.board_editor.reset_to_initial()
        self.spin_max_ply.setValue(250)
        self.combo_mat_preset.setCurrentIndex(0)
        for cb in self.mat_white.values(): cb.setCurrentIndex(0)
        for cb in self.mat_black.values(): cb.setCurrentIndex(0)
        self.chk_opposite_bishops.setChecked(False)
        self.chk_same_bishops.setChecked(False)
        self.rb_final_pos.setChecked(True)

    def load_filter(self, f: dict):
        if "player" in f: self.in_player.setText(f["player"])
        if "white" in f: self.in_white.setText(f["white"])
        if "black" in f: self.in_black.setText(f["black"])
        if "result" in f:
            idx = self.in_result.findText(f["result"])
            if idx >= 0: self.in_result.setCurrentIndex(idx)
        if "eco" in f: self.in_eco.setText(f["eco"])
        if "date" in f: self.in_date.setText(f["date"])
        if "event" in f: self.in_event.setText(f["event"])
        if "site" in f: self.in_site.setText(f["site"])
        if "include_deleted" in f: self.chk_include_del.setChecked(f["include_deleted"])
        if "only_deleted" in f: self.chk_only_del.setChecked(f["only_deleted"])
        if "fen" in f: self.in_fen.setText(f["fen"])
        
        mat = f.get("material")
        if mat:
            mapping_w = {'white_queens': 'q', 'white_rooks': 'r', 'white_bishops': 'b', 'white_knights': 'n', 'white_pawns': 'p'}
            for f_key, pkey in mapping_w.items():
                if f_key in mat and mat[f_key] is not None:
                    idx = self.mat_white[pkey].findText(str(mat[f_key]))
                    if idx >= 0: self.mat_white[pkey].setCurrentIndex(idx)
            mapping_b = {'black_queens': 'q', 'black_rooks': 'r', 'black_bishops': 'b', 'black_knights': 'n', 'black_pawns': 'p'}
            for f_key, pkey in mapping_b.items():
                if f_key in mat and mat[f_key] is not None:
                    idx = self.mat_black[pkey].findText(str(mat[f_key]))
                    if idx >= 0: self.mat_black[pkey].setCurrentIndex(idx)
            if mat.get("opposite_bishops"):
                self.chk_opposite_bishops.setChecked(True)
            elif mat.get("same_bishops"):
                self.chk_same_bishops.setChecked(True)

            if mat.get("match_any_ply"):
                self.rb_any_move.setChecked(True)
            else:
                self.rb_final_pos.setChecked(True)

    def get_filter_dict(self) -> dict:
        f = {}
        p = self.in_player.text().strip()
        if p: f["player"] = p
        w = self.in_white.text().strip()
        if w: f["white"] = w
        b = self.in_black.text().strip()
        if b: f["black"] = b
        res = self.in_result.currentText()
        if res != "All": f["result"] = res
        eco = self.in_eco.text().strip()
        if eco: f["eco"] = eco
        dt = self.in_date.text().strip()
        if dt: f["date"] = dt
        ev = self.in_event.text().strip()
        if ev: f["event"] = ev
        st = self.in_site.text().strip()
        if st: f["site"] = st
        f["include_deleted"] = self.chk_include_del.isChecked()
        f["only_deleted"] = self.chk_only_del.isChecked()

        fen = self.in_fen.text().strip()
        if fen: f["fen"] = fen

        mat = {}
        def parse_val(cb):
            t = cb.currentText()
            return None if t == "Any" else int(t)

        mapping_w = {'q': 'white_queens', 'r': 'white_rooks', 'b': 'white_bishops', 'n': 'white_knights', 'p': 'white_pawns'}
        for pkey, f_key in mapping_w.items():
            v = parse_val(self.mat_white[pkey])
            if v is not None: mat[f_key] = v

        mapping_b = {'q': 'black_queens', 'r': 'black_rooks', 'b': 'black_bishops', 'n': 'black_knights', 'p': 'black_pawns'}
        for pkey, f_key in mapping_b.items():
            v = parse_val(self.mat_black[pkey])
            if v is not None: mat[f_key] = v

        if self.chk_opposite_bishops.isChecked():
            mat["opposite_bishops"] = True
        elif self.chk_same_bishops.isChecked():
            mat["same_bishops"] = True

        if mat:
            mat["match_any_ply"] = self.rb_any_move.isChecked()
            mat["max_ply"] = self.spin_max_ply.value()
            f["material"] = mat

        return f


