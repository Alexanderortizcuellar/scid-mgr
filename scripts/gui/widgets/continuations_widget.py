import chess
from PyQt5.QtCore import Qt, pyqtSignal
from PyQt5.QtGui import QFont, QColor
from PyQt5.QtWidgets import (
    QWidget,
    QVBoxLayout,
    QHBoxLayout,
    QLabel,
    QPushButton,
    QFrame,
    QSplitter,
    QTableWidget,
    QTableWidgetItem,
    QHeaderView,
    QMessageBox,
    QSpinBox,
    QDoubleSpinBox,
    QCheckBox,
    QGroupBox,
    QInputDialog,
    QApplication,
)

from ..backend_client import BackendClient
from .board_widget import ChessBoardEditorWidget


class ContinuationsWidget(QWidget):
    """
    Interactive Common Continuations Explorer & Multi-Move Sequence Analyzer.
    - Displays top multi-move continuation sequences from any position.
    - Zero-copy binary graph (.hot.idx) or on-the-fly candidate-accelerated dynamic search.
    - Interactive chessboard with auto-analysis, depth and branch filtering.
    - Double-click any continuation line to play it onto the board.
    """

    continuation_selected = pyqtSignal(list)  # list of SAN move strings

    def __init__(self, client: BackendClient, main_window, parent=None):
        super().__init__(parent)
        self.client = client
        self.main_window = main_window
        self.board = chess.Board()
        self.move_history = []  # list of (chess.Move, san_str)
        self.current_report = None
        self.selected_line_moves = []

        self.init_ui()

    def init_ui(self):
        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(8, 8, 8, 8)
        main_layout.setSpacing(6)

        # 1. Top Toolbar (Navigation & Controls)
        tb_layout = QHBoxLayout()

        self.btn_start = QPushButton("⏮ Start")
        self.btn_start.setToolTip("Reset to starting position")
        self.btn_start.clicked.connect(self.go_to_start)
        tb_layout.addWidget(self.btn_start)

        self.btn_back = QPushButton("◀ Back")
        self.btn_back.setToolTip("Undo last move")
        self.btn_back.clicked.connect(self.go_back)
        tb_layout.addWidget(self.btn_back)

        self.btn_fen = QPushButton("📋 Paste FEN")
        self.btn_fen.setToolTip("Set custom position from FEN string")
        self.btn_fen.clicked.connect(self.prompt_paste_fen)
        tb_layout.addWidget(self.btn_fen)

        self.lbl_moves_seq = QLabel("1. Starting Position")
        self.lbl_moves_seq.setStyleSheet(
            "font-weight: bold; font-size: 12px; color: #1976d2; margin-left: 8px;"
        )
        tb_layout.addWidget(self.lbl_moves_seq)

        tb_layout.addStretch()

        self.chk_auto = QCheckBox("⚡ Auto-Analyze")
        self.chk_auto.setChecked(True)
        self.chk_auto.setToolTip("Automatically calculate continuations when board position changes")
        tb_layout.addWidget(self.chk_auto)

        self.btn_analyze = QPushButton("▶ Analyze")
        self.btn_analyze.setStyleSheet(
            "font-weight: bold; background-color: #1976d2; color: white; padding: 4px 12px;"
        )
        self.btn_analyze.setToolTip("Run common continuations analysis for current position")
        self.btn_analyze.clicked.connect(self.refresh_current_position)
        tb_layout.addWidget(self.btn_analyze)

        self.btn_build_hot = QPushButton("⚡ Build .hot.idx")
        self.btn_build_hot.setStyleSheet("padding: 4px 8px; font-size: 11px;")
        self.btn_build_hot.setToolTip("Open index builder to generate precomputed .hot.idx graph")
        self.btn_build_hot.clicked.connect(self.main_window.prompt_build_pos_index)
        tb_layout.addWidget(self.btn_build_hot)

        main_layout.addLayout(tb_layout)

        # 2. Parameters Group Bar
        params_card = QFrame()
        params_card.setFrameShape(QFrame.StyledPanel)
        params_card.setStyleSheet("background-color: #f8f9fa; border-radius: 4px; padding: 2px;")
        params_layout = QHBoxLayout(params_card)
        params_layout.setContentsMargins(8, 4, 8, 4)
        params_layout.setSpacing(12)

        # Depth
        lbl_depth = QLabel("Depth:")
        lbl_depth.setStyleSheet("font-weight: bold; font-size: 11px;")
        params_layout.addWidget(lbl_depth)
        self.spin_depth = QSpinBox()
        self.spin_depth.setRange(1, 30)
        self.spin_depth.setValue(8)
        self.spin_depth.setSuffix(" plies")
        self.spin_depth.setToolTip("Maximum continuation depth (plies / half-moves)")
        self.spin_depth.valueChanged.connect(self._on_param_changed)
        params_layout.addWidget(self.spin_depth)

        # Max Lines
        lbl_lines = QLabel("Max Lines:")
        lbl_lines.setStyleSheet("font-weight: bold; font-size: 11px;")
        params_layout.addWidget(lbl_lines)
        self.spin_max_lines = QSpinBox()
        self.spin_max_lines.setRange(1, 50)
        self.spin_max_lines.setValue(10)
        self.spin_max_lines.setSuffix(" lines")
        self.spin_max_lines.setToolTip("Maximum number of top continuation branches to display")
        self.spin_max_lines.valueChanged.connect(self._on_param_changed)
        params_layout.addWidget(self.spin_max_lines)

        # Min Games
        lbl_min_games = QLabel("Min Games:")
        lbl_min_games.setStyleSheet("font-weight: bold; font-size: 11px;")
        params_layout.addWidget(lbl_min_games)
        self.spin_min_games = QSpinBox()
        self.spin_min_games.setRange(1, 10000)
        self.spin_min_games.setValue(1)
        self.spin_min_games.setToolTip("Filter out continuation paths occurring fewer than N times")
        self.spin_min_games.valueChanged.connect(self._on_param_changed)
        params_layout.addWidget(self.spin_min_games)

        # Min %
        lbl_min_pct = QLabel("Min %:")
        lbl_min_pct.setStyleSheet("font-weight: bold; font-size: 11px;")
        params_layout.addWidget(lbl_min_pct)
        self.spin_min_pct = QDoubleSpinBox()
        self.spin_min_pct.setRange(0.0, 100.0)
        self.spin_min_pct.setValue(0.0)
        self.spin_min_pct.setSingleStep(0.5)
        self.spin_min_pct.setSuffix("%")
        self.spin_min_pct.setToolTip("Minimum percentage share of position games")
        self.spin_min_pct.valueChanged.connect(self._on_param_changed)
        params_layout.addWidget(self.spin_min_pct)

        params_layout.addStretch()
        main_layout.addWidget(params_card)

        # 3. Summary Card (Games Reaching Position, Win/Draw/Loss stats)
        self.summary_card = QFrame()
        self.summary_card.setFrameShape(QFrame.StyledPanel)
        self.summary_card.setStyleSheet(
            "background-color: #f1f3f4; border-radius: 4px; padding: 4px;"
        )
        sum_box = QHBoxLayout(self.summary_card)
        sum_box.setContentsMargins(8, 4, 8, 4)

        self.lbl_summary_games = QLabel("Position Games: -")
        self.lbl_summary_games.setStyleSheet("font-weight: bold; font-size: 12px;")
        sum_box.addWidget(self.lbl_summary_games)

        sum_box.addSpacing(15)
        self.lbl_summary_db = QLabel("Total DB Games: -")
        self.lbl_summary_db.setStyleSheet("font-size: 11px; color: #555;")
        sum_box.addWidget(self.lbl_summary_db)

        sum_box.addStretch()
        self.lbl_mode_badge = QLabel("⚡ Ready")
        self.lbl_mode_badge.setStyleSheet(
            "color: #2e7d32; font-weight: bold; font-size: 11px;"
        )
        sum_box.addWidget(self.lbl_mode_badge)

        main_layout.addWidget(self.summary_card)

        # 4. Main Splitter: Board on Left, Continuations Table & Line Inspector on Right
        splitter = QSplitter(Qt.Horizontal)

        # Left: Board Container
        board_panel = QWidget()
        b_box = QVBoxLayout(board_panel)
        b_box.setContentsMargins(0, 0, 0, 0)
        self.board_editor = ChessBoardEditorWidget(self)
        self.board_editor.fen_changed.connect(self._on_board_editor_fen_changed)
        b_box.addWidget(self.board_editor)
        splitter.addWidget(board_panel)

        # Right: Continuations Table & Details Splitter
        right_panel = QWidget()
        r_box = QVBoxLayout(right_panel)
        r_box.setContentsMargins(0, 0, 0, 0)
        r_box.setSpacing(4)

        lbl_table_title = QLabel("Common Continuation Lines (Double-click to play entire line)")
        lbl_table_title.setStyleSheet("font-weight: bold; font-size: 11px; color: #333;")
        r_box.addWidget(lbl_table_title)

        self.cont_table = QTableWidget()
        self.cont_table.setColumnCount(8)
        self.cont_table.setHorizontalHeaderLabels([
            "#", "Continuation Line", "Games", "Share %", "1-0 %", "1/2 %", "0-1 %", "Score"
        ])
        header = self.cont_table.horizontalHeader()
        header.setSectionResizeMode(QHeaderView.Interactive)
        header.setStretchLastSection(False)
        self.cont_table.setColumnWidth(0, 35)
        self.cont_table.setColumnWidth(1, 240)
        self.cont_table.setColumnWidth(2, 65)
        self.cont_table.setColumnWidth(3, 65)
        self.cont_table.setColumnWidth(4, 55)
        self.cont_table.setColumnWidth(5, 55)
        self.cont_table.setColumnWidth(6, 55)
        self.cont_table.setColumnWidth(7, 65)
        header.setSectionResizeMode(1, QHeaderView.Stretch)
        self.cont_table.setSelectionBehavior(QTableWidget.SelectRows)
        self.cont_table.setEditTriggers(QTableWidget.NoEditTriggers)
        self.cont_table.doubleClicked.connect(self.on_row_double_clicked)
        self.cont_table.itemSelectionChanged.connect(self.on_table_selection_changed)
        r_box.addWidget(self.cont_table, 1)

        # Bottom Action Bar for selected continuation line
        action_bar = QHBoxLayout()
        self.lbl_selected_info = QLabel("Select a line to view options")
        self.lbl_selected_info.setStyleSheet("font-size: 11px; color: #555;")
        action_bar.addWidget(self.lbl_selected_info, 1)

        self.btn_play_line = QPushButton("▶ Play Line")
        self.btn_play_line.setEnabled(False)
        self.btn_play_line.setToolTip("Advance board along the selected continuation line")
        self.btn_play_line.clicked.connect(self.play_selected_line)
        action_bar.addWidget(self.btn_play_line)

        self.btn_copy_line = QPushButton("📋 Copy Moves")
        self.btn_copy_line.setEnabled(False)
        self.btn_copy_line.setToolTip("Copy SAN continuation moves to clipboard")
        self.btn_copy_line.clicked.connect(self.copy_selected_line)
        action_bar.addWidget(self.btn_copy_line)

        r_box.addLayout(action_bar)
        splitter.addWidget(right_panel)

        splitter.setSizes([380, 500])
        main_layout.addWidget(splitter, 1)

    # ------------------------------------------------------------------------
    # Interactivity & Board Navigation
    # ------------------------------------------------------------------------

    def _on_param_changed(self):
        if self.chk_auto.isChecked():
            self.refresh_current_position()

    def _on_board_editor_fen_changed(self, fen: str):
        try:
            b = chess.Board(fen)
            self.board = b
            self.move_history.clear()
            self._update_history_label()
            if self.chk_auto.isChecked():
                self.refresh_current_position()
        except Exception:
            pass

    def prompt_paste_fen(self):
        text, ok = QInputDialog.getText(
            self, "Paste FEN", "Enter FEN string for position:", text=self.board.fen()
        )
        if ok and text.strip():
            try:
                b = chess.Board(text.strip())
                self.board = b
                self.move_history.clear()
                self.board_editor.board = self.board.copy()
                self.board_editor.update_board_ui()
                self._update_history_label()
                self.refresh_current_position()
            except Exception as e:
                QMessageBox.warning(self, "Invalid FEN", f"Could not parse FEN: {e}")

    def go_to_start(self):
        self.board = chess.Board()
        self.move_history.clear()
        self.board_editor.board = self.board.copy()
        self.board_editor.update_board_ui()
        self._update_history_label()
        self.refresh_current_position()

    def go_back(self):
        if self.move_history:
            self.board.pop()
            self.move_history.pop()
            self.board_editor.board = self.board.copy()
            self.board_editor.update_board_ui()
            self._update_history_label()
            self.refresh_current_position()

    def play_move_san(self, san: str):
        try:
            mv = self.board.parse_san(san)
            self.board.push(mv)
            self.move_history.append((mv, san))
            self.board_editor.board = self.board.copy()
            self.board_editor.update_board_ui()
            self._update_history_label()
            self.refresh_current_position()
        except Exception as e:
            QMessageBox.warning(self, "Invalid Move", f"Could not play move {san}: {e}")

    def _update_history_label(self):
        if not self.move_history:
            self.lbl_moves_seq.setText("1. Starting Position")
            return

        temp_board = chess.Board()
        san_parts = []
        for i, (mv, san) in enumerate(self.move_history):
            move_num = (i // 2) + 1
            if i % 2 == 0:
                san_parts.append(f"{move_num}. {san}")
            else:
                san_parts.append(f"{san}")
            temp_board.push(mv)

        full_seq = " ".join(san_parts)
        if len(full_seq) > 60:
            full_seq = "..." + full_seq[-57:]
        self.lbl_moves_seq.setText(full_seq)

    # ------------------------------------------------------------------------
    # Backend Query Execution & Response Handling
    # ------------------------------------------------------------------------

    def refresh_current_position(self):
        if not self.client.is_running():
            self.lbl_mode_badge.setText("⚪ Backend Offline")
            self.lbl_mode_badge.setStyleSheet("color: #757575; font-weight: bold; font-size: 11px;")
            return

        fen = self.board.fen()
        params = {
            "fen": fen,
            "max_depth": self.spin_depth.value(),
            "max_lines": self.spin_max_lines.value(),
            "min_games": self.spin_min_games.value(),
            "min_percentage": self.spin_min_pct.value(),
        }

        self.lbl_mode_badge.setText("⏳ Analyzing...")
        self.lbl_mode_badge.setStyleSheet("color: #1565c0; font-weight: bold; font-size: 11px;")
        self.client.send_request("continuations", params)

    def on_continuations_report(self, data: dict):
        self.current_report = data
        total_db_games = data.get("total_games_processed", 0)
        reaching_games = data.get("games_reaching_position", 0)
        lines = data.get("lines", [])

        self.lbl_summary_db.setText(f"Total DB Games: {total_db_games:,}")

        if total_db_games > 0:
            pos_pct = (reaching_games / total_db_games) * 100.0
            self.lbl_summary_games.setText(f"Position Games: {reaching_games:,} ({pos_pct:.2f}%)")
        else:
            self.lbl_summary_games.setText(f"Position Games: {reaching_games:,}")

        self.lbl_mode_badge.setText(f"🟢 Found {len(lines)} lines")
        self.lbl_mode_badge.setStyleSheet("color: #2e7d32; font-weight: bold; font-size: 11px;")

        self._render_table(lines, reaching_games)

    def _render_table(self, lines: list, total_reaching: int):
        self.cont_table.setRowCount(len(lines))

        for row, line_data in enumerate(lines):
            san_line = line_data.get("formatted") or line_data.get("san_line") or " ".join(line_data.get("moves", []))
            games = line_data.get("games", 0)
            share_pct = line_data.get("percentage", 0.0)

            if "white_win_pct" in line_data:
                w_pct = line_data.get("white_win_pct", 0.0)
                d_pct = line_data.get("draw_pct", 0.0)
                b_pct = line_data.get("black_win_pct", 0.0)
            elif games > 0:
                w_pct = (line_data.get("white_wins", 0) / games) * 100.0
                d_pct = (line_data.get("draws", 0) / games) * 100.0
                b_pct = (line_data.get("black_wins", 0) / games) * 100.0
            else:
                w_pct = d_pct = b_pct = 0.0

            # Score = White % + 0.5 * Draw %
            score = w_pct + 0.5 * d_pct

            # Col 0: Rank
            item_rank = QTableWidgetItem(str(row + 1))
            item_rank.setTextAlignment(Qt.AlignCenter)

            # Col 1: Line
            item_line = QTableWidgetItem(san_line)
            item_line.setFont(QFont("Segoe UI", 9, QFont.Bold if row < 3 else QFont.Normal))

            # Col 2: Games
            item_games = QTableWidgetItem(f"{games:,}")
            item_games.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)
            item_games.setFont(QFont("Segoe UI", 9, QFont.Bold))

            # Col 3: Share %
            item_share = QTableWidgetItem(f"{share_pct:.1f}%")
            item_share.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)

            # Col 4: White %
            item_w = QTableWidgetItem(f"{w_pct:.1f}%")
            item_w.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)
            item_w.setForeground(QColor("#2e7d32"))

            # Col 5: Draw %
            item_d = QTableWidgetItem(f"{d_pct:.1f}%")
            item_d.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)
            item_d.setForeground(QColor("#757575"))

            # Col 6: Black %
            item_b = QTableWidgetItem(f"{b_pct:.1f}%")
            item_b.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)
            item_b.setForeground(QColor("#c62828"))

            # Col 7: Score %
            item_score = QTableWidgetItem(f"{score:.1f}%")
            item_score.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)
            item_score.setFont(QFont("Segoe UI", 9, QFont.Bold))

            self.cont_table.setItem(row, 0, item_rank)
            self.cont_table.setItem(row, 1, item_line)
            self.cont_table.setItem(row, 2, item_games)
            self.cont_table.setItem(row, 3, item_share)
            self.cont_table.setItem(row, 4, item_w)
            self.cont_table.setItem(row, 5, item_d)
            self.cont_table.setItem(row, 6, item_b)
            self.cont_table.setItem(row, 7, item_score)

        self.on_table_selection_changed()

    # ------------------------------------------------------------------------
    # Table Selection & Line Actions
    # ------------------------------------------------------------------------

    def on_table_selection_changed(self):
        selected = self.cont_table.selectionModel().selectedRows()
        if not selected or not self.current_report:
            self.lbl_selected_info.setText("Select a line to view options")
            self.btn_play_line.setEnabled(False)
            self.btn_copy_line.setEnabled(False)
            self.selected_line_moves = []
            return

        row = selected[0].row()
        lines = self.current_report.get("lines", [])
        if row >= len(lines):
            return

        line_data = lines[row]
        san_line = line_data.get("formatted") or line_data.get("san_line") or " ".join(line_data.get("moves", []))
        games = line_data.get("games", 0)
        self.selected_line_moves = line_data.get("moves", [])

        self.lbl_selected_info.setText(f"Selected Line #{row + 1}: {san_line} ({games:,} games)")
        self.btn_play_line.setEnabled(True)
        self.btn_copy_line.setEnabled(True)

    def on_row_double_clicked(self, index):
        row = index.row()
        if not self.current_report:
            return
        lines = self.current_report.get("lines", [])
        if row < len(lines):
            line_data = lines[row]
            moves = line_data.get("moves", [])
            self._play_continuation_sequence(moves)

    def play_selected_line(self):
        if self.selected_line_moves:
            self._play_continuation_sequence(self.selected_line_moves)

    def _play_continuation_sequence(self, moves: list):
        """Pushes each move in the continuation sequence onto the board."""
        for san in moves:
            try:
                mv = self.board.parse_san(san)
                self.board.push(mv)
                self.move_history.append((mv, san))
            except Exception as e:
                QMessageBox.warning(self, "Move Error", f"Failed applying move {san}: {e}")
                break

        self.board_editor.board = self.board.copy()
        self.board_editor.update_board_ui()
        self._update_history_label()
        self.refresh_current_position()

    def copy_selected_line(self):
        if self.selected_line_moves:
            text = " ".join(self.selected_line_moves)
            QApplication.clipboard().setText(text)
            self.main_window.status_bar.showMessage(f"Copied continuation line: {text}", 3000)
