import chess
from PyQt5.QtCore import Qt
from PyQt5.QtGui import QFont, QColor
from PyQt5.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QLabel, QPushButton, QFrame,
    QSplitter, QTableWidget, QTableWidgetItem, QHeaderView, QMessageBox, QComboBox
)
from ..backend_client import BackendClient
from .board_widget import ChessBoardEditorWidget

class OpeningTreeWidget(QWidget):
    """
    Interactive Opening Explorer & Tree View (Lichess/ChessBase style).
    - Clicking any move instantly fetches the next branch in < 1ms.
    - Displays win/draw/loss % bars and average ELO.
    """
    def __init__(self, client: BackendClient, main_window, parent=None):
        super().__init__(parent)
        self.client = client
        self.main_window = main_window
        self.board = chess.Board()
        self.move_history = [] # list of (Move, san)
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

        # Splitter (Board on left, Moves Tree Table on right)
        splitter = QSplitter(Qt.Horizontal)

        # Left: Board Container
        board_panel = QWidget()
        b_box = QVBoxLayout(board_panel)
        b_box.setContentsMargins(0, 0, 0, 0)
        self.board_editor = ChessBoardEditorWidget(self)
        b_box.addWidget(self.board_editor)
        splitter.addWidget(board_panel)

        # Right: Tree Table & Top Games
        right_panel = QWidget()
        r_box = QVBoxLayout(right_panel)
        r_box.setContentsMargins(0, 0, 0, 0)
        r_box.setSpacing(6)

        # Tree Table
        self.tree_table = QTableWidget()
        self.tree_table.setColumnCount(7)
        self.tree_table.setHorizontalHeaderLabels([
            "Move", "Games", "Score", "1-0 %", "1/2 %", "0-1 %", "Avg Elo (W/B)"
        ])
        header = self.tree_table.horizontalHeader()
        header.setSectionResizeMode(QHeaderView.Interactive)
        header.setStretchLastSection(True)
        self.tree_table.setSelectionBehavior(QTableWidget.SelectRows)
        self.tree_table.setEditTriggers(QTableWidget.NoEditTriggers)
        self.tree_table.doubleClicked.connect(self.on_row_double_clicked)
        r_box.addWidget(self.tree_table)

        splitter.addWidget(right_panel)
        splitter.setStretchFactor(0, 4)
        splitter.setStretchFactor(1, 6)

        main_layout.addWidget(splitter)

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
        self.tree_table.setRowCount(len(moves))

        for row, m in enumerate(moves):
            san = m.get("san", "")
            g_count = m.get("total_games", 0)
            mw_pct = m.get("white_pct", 0.0)
            md_pct = m.get("draw_pct", 0.0)
            mb_pct = m.get("black_pct", 0.0)
            score = mw_pct + (md_pct / 2.0)
            avg_w = m.get("avg_white_elo")
            avg_b = m.get("avg_black_elo")
            elo_str = f"{avg_w or '-'}/{avg_b or '-'}"

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
            item_elo = QTableWidgetItem(elo_str)
            item_elo.setTextAlignment(Qt.AlignCenter)

            self.tree_table.setItem(row, 0, item_san)
            self.tree_table.setItem(row, 1, item_games)
            self.tree_table.setItem(row, 2, item_score)
            self.tree_table.setItem(row, 3, item_w)
            self.tree_table.setItem(row, 4, item_d)
            self.tree_table.setItem(row, 5, item_b)
            self.tree_table.setItem(row, 6, item_elo)

    def unload_index(self):
        if self.client.is_running():
            self.client.send_request("unload_pos_index")
            self.main_window.status_bar.showMessage("Position index unloaded from RAM.", 4000)
            self.lbl_index_badge.setText("⚪ Fast Index: On Disk (Unloaded)")
            self.lbl_index_badge.setStyleSheet("color: #757575; font-weight: bold; font-size: 11px;")



