import chess
from PyQt5.QtCore import Qt
from PyQt5.QtGui import QFont, QColor
from PyQt5.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QLabel, QPushButton, QFrame,
    QSplitter, QTableWidget, QTableWidgetItem, QHeaderView, QMessageBox,
    QComboBox, QCheckBox
)
from ..backend_client import BackendClient
from .board_widget import ChessBoardEditorWidget

class OpeningTreeWidget(QWidget):
    """
    Interactive Opening Explorer & Tree View (Lichess/ChessBase style).
    - Clicking any move instantly fetches the next branch in < 1ms.
    - Displays win/draw/loss % bars, Last Played (or Average ELO).
    - Displays sample games with 1-click preview and double-click to load.
    """
    def __init__(self, client: BackendClient, main_window, parent=None):
        super().__init__(parent)
        self.client = client
        self.main_window = main_window
        self.board = chess.Board()
        self.move_history = []  # list of (Move, san)
        self.current_report = None

        self.init_ui()

    def init_ui(self):
        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(8, 8, 8, 8)
        main_layout.setSpacing(6)

        # Header toolbar
        tb_layout = QHBoxLayout()
        self.btn_start = QPushButton("⏮ Start")
        self.btn_start.clicked.connect(self.go_to_start)
        tb_layout.addWidget(self.btn_start)

        self.btn_back = QPushButton("◀ Back")
        self.btn_back.clicked.connect(self.go_back)
        tb_layout.addWidget(self.btn_back)

        self.lbl_moves_seq = QLabel("1. Starting Position")
        self.lbl_moves_seq.setStyleSheet("font-weight: bold; font-size: 12px; color: #1976d2; margin-left: 8px;")
        tb_layout.addWidget(self.lbl_moves_seq)

        tb_layout.addStretch()

        self.chk_last_played = QCheckBox("Last Played")
        self.chk_last_played.setChecked(True)
        self.chk_last_played.setToolTip("Toggle displaying 'Last Played' date vs 'Avg Elo (W/B)' in the candidate moves table")
        self.chk_last_played.toggled.connect(self.on_last_played_toggled)
        tb_layout.addWidget(self.chk_last_played)

        self.combo_scope = QComboBox()
        self.combo_scope.addItems(["🌐 Entire Database", "🔍 Search Results"])
        self.combo_scope.setToolTip("Choose whether to calculate tree statistics across the entire database or only for current search results")
        self.combo_scope.setStyleSheet("padding: 2px 6px; font-size: 11px; font-weight: bold;")
        self.combo_scope.currentIndexChanged.connect(self.on_scope_changed)
        tb_layout.addWidget(self.combo_scope)

        self.btn_unload = QPushButton("🧹 Free Memory")
        self.btn_unload.setToolTip("Unloads the position index from RAM")
        self.btn_unload.setStyleSheet("padding: 3px 8px; font-size: 11px;")
        self.btn_unload.clicked.connect(self.unload_index)
        tb_layout.addWidget(self.btn_unload)

        self.btn_rebuild = QPushButton("⚡ Rebuild Index")
        self.btn_rebuild.setStyleSheet("font-weight: bold; padding: 3px 8px; font-size: 11px;")
        self.btn_rebuild.clicked.connect(self.main_window.prompt_build_pos_index)
        tb_layout.addWidget(self.btn_rebuild)

        main_layout.addLayout(tb_layout)

        # Summary Bar (Games count, Win/Draw/Loss percentages)
        self.summary_card = QFrame()
        self.summary_card.setFrameShape(QFrame.StyledPanel)
        self.summary_card.setStyleSheet("background-color: #f1f3f4; border-radius: 4px; padding: 4px;")
        sum_box = QHBoxLayout(self.summary_card)
        sum_box.setContentsMargins(8, 4, 8, 4)

        self.lbl_summary_games = QLabel("Total Games: -")
        self.lbl_summary_games.setStyleSheet("font-weight: bold; font-size: 12px;")
        sum_box.addWidget(self.lbl_summary_games)

        sum_box.addSpacing(15)
        self.lbl_summary_score = QLabel("⚪ White: -% | 🤝 Draw: -% | ⚫ Black: -%")
        self.lbl_summary_score.setStyleSheet("font-weight: bold; font-size: 12px;")
        sum_box.addWidget(self.lbl_summary_score)

        sum_box.addStretch()
        self.lbl_index_badge = QLabel("⚡ Fast Index Active")
        self.lbl_index_badge.setStyleSheet("color: #2e7d32; font-weight: bold; font-size: 11px;")
        sum_box.addWidget(self.lbl_index_badge)

        main_layout.addWidget(self.summary_card)

        # Splitter (Board on left, Explorer Panels on right)
        splitter = QSplitter(Qt.Horizontal)

        # Left: Board Container
        board_panel = QWidget()
        b_box = QVBoxLayout(board_panel)
        b_box.setContentsMargins(0, 0, 0, 0)
        self.board_editor = ChessBoardEditorWidget(self)
        b_box.addWidget(self.board_editor)
        splitter.addWidget(board_panel)

        # Right: Vertical Splitter (Top: Candidate Moves, Bottom: Sample Games)
        right_splitter = QSplitter(Qt.Vertical)

        # 1. Candidate Moves Panel
        moves_panel = QWidget()
        m_box = QVBoxLayout(moves_panel)
        m_box.setContentsMargins(0, 0, 0, 0)
        m_box.setSpacing(4)

        lbl_moves_title = QLabel("Candidate Moves")
        lbl_moves_title.setStyleSheet("font-weight: bold; font-size: 11px; color: #424242;")
        m_box.addWidget(lbl_moves_title)

        self.tree_table = QTableWidget()
        self.tree_table.setColumnCount(7)
        self._update_table_headers()
        tree_header = self.tree_table.horizontalHeader()
        tree_header.setSectionResizeMode(QHeaderView.Interactive)
        tree_header.setStretchLastSection(True)
        self.tree_table.setSelectionBehavior(QTableWidget.SelectRows)
        self.tree_table.setEditTriggers(QTableWidget.NoEditTriggers)
        self.tree_table.doubleClicked.connect(self.on_row_double_clicked)
        self.tree_table.itemSelectionChanged.connect(self.on_tree_selection_changed)
        m_box.addWidget(self.tree_table)
        right_splitter.addWidget(moves_panel)

        # 2. Sample Games Panel
        sample_panel = QWidget()
        s_box = QVBoxLayout(sample_panel)
        s_box.setContentsMargins(0, 4, 0, 0)
        s_box.setSpacing(4)

        self.lbl_sample_games_title = QLabel("Sample Games in Position (Double-click to load)")
        self.lbl_sample_games_title.setStyleSheet("font-weight: bold; font-size: 11px; color: #424242;")
        s_box.addWidget(self.lbl_sample_games_title)

        self.sample_games_table = QTableWidget()
        self.sample_games_table.setColumnCount(7)
        self.sample_games_table.setHorizontalHeaderLabels([
            "ID", "White", "Black", "Result", "Date", "ECO", "Event"
        ])
        s_header = self.sample_games_table.horizontalHeader()
        s_header.setSectionResizeMode(QHeaderView.Interactive)
        s_header.setStretchLastSection(True)
        self.sample_games_table.setColumnWidth(0, 50)
        self.sample_games_table.setColumnWidth(1, 130)
        self.sample_games_table.setColumnWidth(2, 130)
        self.sample_games_table.setColumnWidth(3, 65)
        self.sample_games_table.setColumnWidth(4, 80)
        self.sample_games_table.setColumnWidth(5, 55)
        self.sample_games_table.setSelectionBehavior(QTableWidget.SelectRows)
        self.sample_games_table.setEditTriggers(QTableWidget.NoEditTriggers)
        self.sample_games_table.doubleClicked.connect(self.on_sample_game_double_clicked)
        s_box.addWidget(self.sample_games_table)
        right_splitter.addWidget(sample_panel)

        right_splitter.setStretchFactor(0, 6)
        right_splitter.setStretchFactor(1, 4)

        splitter.addWidget(right_splitter)
        splitter.setStretchFactor(0, 4)
        splitter.setStretchFactor(1, 6)

        main_layout.addWidget(splitter)

    def _update_table_headers(self):
        last_col = "Last Played" if self.chk_last_played.isChecked() else "Avg Elo (W/B)"
        self.tree_table.setHorizontalHeaderLabels([
            "Move", "Games", "Score", "1-0 %", "1/2 %", "0-1 %", last_col
        ])

    def on_last_played_toggled(self, checked: bool):
        self._update_table_headers()
        if self.current_report:
            self._render_tree_table(self.current_report.get("moves", []))

    def on_scope_changed(self, index: int):
        if index == 1:
            self.lbl_index_badge.setText("🔍 Filtered Search Results (Live)")
            self.lbl_index_badge.setStyleSheet("color: #1565c0; font-weight: bold; font-size: 11px;")
        else:
            self.lbl_index_badge.setText("⚡ Entire Database")
            self.lbl_index_badge.setStyleSheet("color: #2e7d32; font-weight: bold; font-size: 11px;")
        self.refresh_current_position()

    def refresh_current_position(self):
        if not self.client.is_running():
            return
        fen = self.board.fen()
        self.board_editor.board = self.board.copy()
        self.board_editor.update_board_ui()
        self._update_history_label()

        use_search_results = (self.combo_scope.currentIndex() == 1) if hasattr(self, 'combo_scope') else False
        params = {"fen": fen}
        if use_search_results:
            params["use_search_results"] = True
        self.client.send_request("opening_tree", params)

    def go_to_start(self):
        self.board = chess.Board()
        self.move_history.clear()
        self.refresh_current_position()

    def go_back(self):
        if self.move_history:
            self.board.pop()
            self.move_history.pop()
            self.refresh_current_position()

    def play_move_san(self, san: str):
        try:
            mv = self.board.parse_san(san)
            self.board.push(mv)
            self.move_history.append((mv, san))
            self.refresh_current_position()
        except Exception as e:
            QMessageBox.warning(self, "Invalid Move", f"Could not play move {san}: {e}")

    def on_row_double_clicked(self, index):
        row = index.row()
        item = self.tree_table.item(row, 0)
        if item:
            san = item.text().strip()
            self.play_move_san(san)

    def on_tree_selection_changed(self):
        selected_rows = self.tree_table.selectionModel().selectedRows()
        if not selected_rows:
            if self.current_report:
                self.lbl_sample_games_title.setText("Sample Games in Position (Double-click to load)")
                self._populate_sample_games(self.current_report.get("sample_games", []))
            return

        row = selected_rows[0].row()
        if not self.current_report or row >= len(self.current_report.get("moves", [])):
            return

        move_data = self.current_report["moves"][row]
        san = move_data.get("san", "")
        sample_ids = move_data.get("sample_game_ids", [])
        if sample_ids:
            self.lbl_sample_games_title.setText(f"Sample Games for {san} ({len(sample_ids)} games) (Double-click to load)")
            self.client.send_request("get_game_summaries", {"game_ids": sample_ids})
        else:
            self.sample_games_table.setRowCount(0)

    def on_sample_game_double_clicked(self, index):
        row = index.row()
        item = self.sample_games_table.item(row, 0)
        if item:
            try:
                game_id = int(item.text())
                self.main_window.load_game_by_id(game_id)
            except ValueError:
                pass

    def on_game_summaries_received(self, summaries: list):
        self._populate_sample_games(summaries)

    def _populate_sample_games(self, games: list):
        self.sample_games_table.setRowCount(len(games))
        for row, g in enumerate(games):
            gid = str(g.get("id", row + 1))
            white = g.get("white", "?")
            black = g.get("black", "?")
            result = g.get("result", "*")
            date = g.get("date", "????.??.??")
            eco = g.get("eco", "")
            event = g.get("event", "")

            it_id = QTableWidgetItem(gid)
            it_id.setTextAlignment(Qt.AlignCenter)
            it_w = QTableWidgetItem(white)
            it_b = QTableWidgetItem(black)
            it_r = QTableWidgetItem(result)
            it_r.setTextAlignment(Qt.AlignCenter)
            it_d = QTableWidgetItem(date)
            it_d.setTextAlignment(Qt.AlignCenter)
            it_eco = QTableWidgetItem(eco)
            it_eco.setTextAlignment(Qt.AlignCenter)
            it_ev = QTableWidgetItem(event)

            self.sample_games_table.setItem(row, 0, it_id)
            self.sample_games_table.setItem(row, 1, it_w)
            self.sample_games_table.setItem(row, 2, it_b)
            self.sample_games_table.setItem(row, 3, it_r)
            self.sample_games_table.setItem(row, 4, it_d)
            self.sample_games_table.setItem(row, 5, it_eco)
            self.sample_games_table.setItem(row, 6, it_ev)

    def _update_history_label(self):
        if not self.move_history:
            self.lbl_moves_seq.setText("1. Starting Position")
            return
        text_parts = []
        for i, (_, san) in enumerate(self.move_history):
            if i % 2 == 0:
                text_parts.append(f"{i // 2 + 1}. {san}")
            else:
                text_parts.append(san)
        self.lbl_moves_seq.setText(" ".join(text_parts))

    def on_tree_report(self, report: dict):
        self.current_report = report
        total_games = report.get("total_games", 0)
        w_pct = report.get("white_pct", 0.0)
        d_pct = report.get("draw_pct", 0.0)
        b_pct = report.get("black_pct", 0.0)

        self.lbl_summary_games.setText(f"Total Games in Position: {total_games:,}")
        self.lbl_summary_score.setText(f"⚪ White: {w_pct:.1f}% | 🤝 Draw: {d_pct:.1f}% | ⚫ Black: {b_pct:.1f}%")

        moves = report.get("moves", [])
        self._render_tree_table(moves)

        # Populate initial sample games for the position
        sample_games = report.get("sample_games", [])
        self.lbl_sample_games_title.setText(f"Sample Games in Position ({len(sample_games)} games) (Double-click to load)")
        self._populate_sample_games(sample_games)

    def _render_tree_table(self, moves: list):
        show_last_played = self.chk_last_played.isChecked()
        self.tree_table.blockSignals(True)
        self.tree_table.setRowCount(len(moves))

        for row, m in enumerate(moves):
            san = m.get("san", "")
            g_count = m.get("total_games", 0)
            mw_pct = m.get("white_pct", 0.0)
            md_pct = m.get("draw_pct", 0.0)
            mb_pct = m.get("black_pct", 0.0)
            score = mw_pct + (md_pct / 2.0)

            if show_last_played:
                stat_str = m.get("last_played") or "-"
            else:
                avg_w = m.get("avg_white_elo")
                avg_b = m.get("avg_black_elo")
                stat_str = f"{avg_w or '-'}/{avg_b or '-'}"

            item_san = QTableWidgetItem(san)
            item_san.setFont(QFont("Arial", 10, QFont.Bold))
            item_games = QTableWidgetItem(f"{g_count:,}")
            item_games.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)
            item_score = QTableWidgetItem(f"{score:.1f}%")
            item_score.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)
            item_w = QTableWidgetItem(f"{mw_pct:.1f}%")
            item_w.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)
            item_w.setForeground(QColor("#2e7d32"))
            item_d = QTableWidgetItem(f"{md_pct:.1f}%")
            item_d.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)
            item_d.setForeground(QColor("#757575"))
            item_b = QTableWidgetItem(f"{mb_pct:.1f}%")
            item_b.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)
            item_b.setForeground(QColor("#c62828"))
            item_stat = QTableWidgetItem(stat_str)
            item_stat.setTextAlignment(Qt.AlignCenter)

            self.tree_table.setItem(row, 0, item_san)
            self.tree_table.setItem(row, 1, item_games)
            self.tree_table.setItem(row, 2, item_score)
            self.tree_table.setItem(row, 3, item_w)
            self.tree_table.setItem(row, 4, item_d)
            self.tree_table.setItem(row, 5, item_b)
            self.tree_table.setItem(row, 6, item_stat)

        self.tree_table.blockSignals(False)

    def unload_index(self):
        if self.client.is_running():
            self.client.send_request("unload_pos_index")
            self.main_window.status_bar.showMessage("Position index unloaded from RAM.", 4000)
            self.lbl_index_badge.setText("⚪ Fast Index: On Disk (Unloaded)")
            self.lbl_index_badge.setStyleSheet("color: #757575; font-weight: bold; font-size: 11px;")




