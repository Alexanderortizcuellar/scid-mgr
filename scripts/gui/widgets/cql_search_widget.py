import io
import os
import sys
from typing import Optional, List, Dict, Any
import chess
import chess.pgn
import chess.svg

from PyQt5.QtCore import Qt, pyqtSignal, QByteArray
from PyQt5.QtGui import QFont, QPixmap, QPainter, QColor
from PyQt5.QtSvg import QSvgRenderer
from PyQt5.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QSplitter, QLabel,
    QPushButton, QPlainTextEdit, QTextEdit, QComboBox,
    QTableWidget, QTableWidgetItem, QHeaderView, QFileDialog,
    QSpinBox, QGroupBox, QRadioButton, QButtonGroup, QMessageBox,
    QProgressBar, QApplication, QDialog, QTabWidget, QDialogButtonBox,
    QListWidget, QListWidgetItem
)

from ..backend_client import BackendClient

PRESET_THEMES = {
    "👑 Mates: Smothered Mate (Both sides)": "flipcolor { checkmate and attacks(N, k) and not attacks(k, empty) }",
    "👑 Mates: Arabian Mate": "flipcolor { checkmate and attacks(R, k) and attacks(N, R) and attacks(k, R) and piece k on [a8, h8, a1, h1, b8, g8, a7, h7, b1, g1, a2, h2] }",
    "👑 Mates: Anastasia's Mate": "flipcolor { checkmate and attacks(R, k) and piece N count >= 1 and piece k on [a1-a8, h1-h8, a1-h1, a8-h8] }",
    "👑 Mates: Back Rank Mate": "flipcolor { checkmate and piece k on [a8-h8] and piece [R, Q] on [a8-h8] }",
    "👑 Mates: Boden's Mate": "checkmate and attacks(B, k) and piece B count == 2",
    "👑 Mates: Hook Mate": "checkmate and attacks(R, k) and attacks(N, R) and attacks(P, N) and attacks(k, R)",
    "👑 Mates: Opera Mate": "checkmate and attacks(R, k) and attacks(B, R) and attacks(k, R) and piece k on [d8, e8]",
    "👑 Mates: Reti's Mate": "checkmate and attacks(B, k) and attacks(R, B) and attacks(k, B)",
    "👑 Mates: Suffocation Mate": "checkmate and attacks(N, k) and piece B count >= 1",
    "👑 Mates: Vukovic Mate": "checkmate and attacks(R, k) and piece N count >= 1 and piece k on [a1-a8, h1-h8, a8-h8, a1-h1]",
    "👑 Mates: Ladder (Lawnmower) Mate": "checkmate and piece [R, Q] count >= 2 and piece k on [a1-a8, h1-h8, a8-h8, a1-h1] and attacks(R, k)",
    "👑 Mates: Dovetail Mate": "checkmate and attacks(Q, k) and attacks(k, Q)",
    "👑 Mates: Swallowtail Mate": "checkmate and attacks(Q, k) and piece k on [a1-a8, h1-h8, a8-h8, a1-h1]",
    "👑 Mates: Balestra Mate": "checkmate and attacks(B, k) and piece Q count >= 1",
    "👑 Mates: Blackburne's Mate": "checkmate and attacks(B, k) and piece B count == 2 and piece N count >= 1",
    "👑 Mates: Blind Swine Mate": "checkmate and piece R count >= 2 and piece k on [a8-h8] and piece R on [a7-h7]",
    "👑 Mates: Damiano's Mate": "checkmate and attacks(Q, k) and piece [P, B] count >= 1 and piece k on [g8, h8, g1, h1]",
    "👑 Mates: David and Goliath Mate": "checkmate and attacks(P, k)",
    "👑 Mates: Greco's Mate": "checkmate and attacks(R, k) and piece B count >= 1 and piece k on [h8, h1, a8, a1]",
    "👑 Mates: Legal's Mate": "checkmate and attacks(N, k) and piece B count >= 1 and piece N count >= 2",
    "👑 Mates: Morphy's Mate": "checkmate and attacks(B, k) and piece R count >= 1 and piece k on [h8, h1, a8, a1]",
    "👑 Mates: Pillsbury's Mate": "checkmate and attacks(R, k) and piece B count >= 1 and piece k on [h8, h1]",
    "👑 Mates: Triangle Mate": "checkmate and attacks(Q, k) and piece R count >= 1",
    "👑 Mates: Two Bishops Mate": "checkmate and attacks(B, k) and piece B count == 2 and piece k on [a1-a8, h1-h8, a8-h8, a1-h1]",
    "⚔️ Tactics: Pin from Queen/Rook/Bishop": "pin(pinner in [Q, R, B])",
    "⚔️ Tactics: Fork with Knight/Bishop/Pawn": "fork(attacker in [N, B, P], targets count >= 2)",
    "📐 Geometry: Long Diagonal Bishop Ray": "piece B on [a1..h8]",
    "📐 Geometry: King on First or Second Rank": "piece K on [a1-h2]",
    "♟️ Endgames: Pawn Ending": "queens == 0 and rooks == 0 and minors == 0",
    "♟️ Endgames: Rook Ending": "queens == 0 and minors == 0 and rooks >= 1",
    "♟️ Endgames: Bishop Ending": "queens == 0 and rooks == 0 and knights == 0 and bishops >= 1",
    "♟️ Endgames: Knight Ending": "queens == 0 and rooks == 0 and bishops == 0 and knights >= 1",
}


class CqlExplainDialog(QDialog):
    """
    Dialog displaying the AST analysis, canonical DSL query, and symmetry expansion branches.
    """

    def __init__(self, explanation: Dict[str, Any], parent=None):
        super().__init__(parent)
        self.explanation = explanation
        self.setWindowTitle("💡 CQL Query Plan & Symmetries Explanation")
        self.resize(780, 560)
        self.init_ui()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(12, 12, 12, 12)
        layout.setSpacing(10)

        # Overview
        is_header_only = self.explanation.get("is_header_only", False)
        has_symmetries = self.explanation.get("has_symmetries", False)
        branches = self.explanation.get("branches", [])
        canonical_dsl = self.explanation.get("canonical_dsl", "")

        info_box = QGroupBox("📊 Query Properties & Optimization Plan")
        info_layout = QVBoxLayout(info_box)

        header_status = (
            '<span style="color: #15803d; font-weight: bold;">Yes</span> (Fast metadata/tag search)'
            if is_header_only
            else '<span style="color: #1d4ed8; font-weight: bold;">No</span> (Evaluates board positions / moves)'
        )
        sym_status = (
            f'<span style="color: #d97706; font-weight: bold;">Yes</span> ({len(branches)} branch(es))'
            if has_symmetries
            else '<span style="color: #6b7280;">No</span>'
        )

        info_txt = (
            f"<b>Header-Only Optimization:</b> {header_status}<br>"
            f"<b>Symmetries Active:</b> {sym_status}<br>"
            f"<b>Execution Plan:</b> {len(branches)} parallel symmetry branch(es) to evaluate"
        )
        lbl_info = QLabel(info_txt)
        lbl_info.setTextFormat(Qt.RichText)
        info_layout.addWidget(lbl_info)
        layout.addWidget(info_box)

        # Canonical Query Section
        canon_box = QGroupBox("✨ Canonical Parsed Query")
        canon_layout = QVBoxLayout(canon_box)

        self.txt_canonical = QPlainTextEdit()
        self.txt_canonical.setPlainText(canonical_dsl)
        self.txt_canonical.setReadOnly(True)
        self.txt_canonical.setFont(QFont("Consolas, Courier New, monospace", 10))
        self.txt_canonical.setMaximumHeight(85)
        self.txt_canonical.setStyleSheet(
            "background-color: #1e1e1e; color: #4ade80; border-radius: 4px; padding: 6px;"
        )
        canon_layout.addWidget(self.txt_canonical)
        layout.addWidget(canon_box)

        # Branches / Symmetries Tabs
        branches_box = QGroupBox("🔀 Symmetry Expansion Branches")
        branches_layout = QVBoxLayout(branches_box)

        if branches:
            tab_widget = QTabWidget()
            for idx, branch in enumerate(branches, 1):
                sym_name = branch.get("symmetry_name", f"Branch {idx}")
                branch_dsl = branch.get("dsl", "")
                fens = branch.get("transformed_fens", [])

                branch_page = QWidget()
                page_layout = QVBoxLayout(branch_page)
                page_layout.setContentsMargins(8, 8, 8, 8)
                page_layout.setSpacing(6)

                page_layout.addWidget(QLabel(f"<b>Transformation:</b> <code>{sym_name}</code>"))

                txt_branch = QPlainTextEdit()
                txt_branch.setPlainText(branch_dsl)
                txt_branch.setReadOnly(True)
                txt_branch.setFont(QFont("Consolas, Courier New, monospace", 9))
                txt_branch.setStyleSheet(
                    "background-color: #f8fafc; color: #0f172a; border: 1px solid #cbd5e1; border-radius: 4px; padding: 6px;"
                )
                page_layout.addWidget(txt_branch, stretch=1)

                if fens:
                    page_layout.addWidget(QLabel(f"<b>Transformed FENs ({len(fens)}):</b>"))
                    txt_fens = QPlainTextEdit()
                    txt_fens.setPlainText("\n".join(fens))
                    txt_fens.setReadOnly(True)
                    txt_fens.setFont(QFont("Consolas, Courier New, monospace", 9))
                    txt_fens.setMaximumHeight(65)
                    page_layout.addWidget(txt_fens)

                btn_copy_branch = QPushButton(f"📋 Copy {sym_name} DSL")
                btn_copy_branch.setStyleSheet("padding: 3px 10px;")
                btn_copy_branch.clicked.connect(lambda _, text=branch_dsl: QApplication.clipboard().setText(text))
                page_layout.addWidget(btn_copy_branch, alignment=Qt.AlignRight)

                tab_widget.addTab(branch_page, f"{idx}. {sym_name}")
            branches_layout.addWidget(tab_widget)
        else:
            branches_layout.addWidget(QLabel("No symmetry transformations needed."))

        layout.addWidget(branches_box, stretch=1)

        # Button Bar
        btn_box = QHBoxLayout()
        btn_copy_all = QPushButton("📋 Copy Canonical Query")
        btn_copy_all.setStyleSheet("padding: 4px 12px;")
        btn_copy_all.clicked.connect(lambda: QApplication.clipboard().setText(canonical_dsl))
        btn_box.addWidget(btn_copy_all)

        btn_box.addStretch(1)

        btn_close = QPushButton("Close")
        btn_close.clicked.connect(self.accept)
        btn_close.setStyleSheet("padding: 5px 20px; font-weight: bold; background-color: #1976d2; color: white; border-radius: 4px;")
        btn_box.addWidget(btn_close)

        layout.addLayout(btn_box)


class CqlSearchWidget(QWidget):
    """
    Dedicated Interactive CQL / Search DSL Testing and Development Panel.
    Allows composing queries, validating syntax live, explaining AST and symmetries,
    and executing searches against the active database (.si5, .si4, .pgn) or standalone PGN files.
    """

    def __init__(self, client: BackendClient, parent=None):
        super().__init__(parent)
        self.client = client
        self.matches_data: List[Dict[str, Any]] = []
        self.current_board = chess.Board()
        self.current_game_pgn = ""
        self.current_plies_list: List[int] = []
        self.current_ply_index = 0
        self.game_moves: List[chess.Move] = []
        self.game_positions: List[chess.Board] = []
        self.current_db_path = ""

        self.init_ui()
        if hasattr(self.client, "event_received"):
            self.client.event_received.connect(self.on_backend_event)

    def init_ui(self):
        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(8, 8, 8, 8)
        main_layout.setSpacing(6)

        # Top Control / Query Editor Section
        top_box = QGroupBox("🔍 Search Engine & CQL Query Editor")
        top_layout = QVBoxLayout(top_box)
        top_layout.setContentsMargins(10, 10, 10, 10)
        top_layout.setSpacing(6)

        # 1. Preset Selector Bar
        preset_bar = QHBoxLayout()
        preset_bar.addWidget(QLabel("📚 Preset Theme / Template:"))
        self.combo_presets = QComboBox()
        self.combo_presets.addItem("— Select Theme or Pattern Template —", "")
        for label, query in PRESET_THEMES.items():
            self.combo_presets.addItem(label, query)
        self.combo_presets.currentIndexChanged.connect(self.on_preset_selected)
        preset_bar.addWidget(self.combo_presets, stretch=1)

        self.btn_explain = QPushButton("💡 Explain Query")
        self.btn_explain.setStyleSheet("font-weight: bold; background-color: #fef3c7; color: #92400e; padding: 4px 12px; border-radius: 4px; border: 1px solid #fde68a;")
        self.btn_explain.setToolTip("Inspect canonical query, symmetry branch expansions, and execution plan")
        self.btn_explain.clicked.connect(self.explain_current_query)
        preset_bar.addWidget(self.btn_explain)

        self.btn_validate = QPushButton("✓ Validate Syntax")
        self.btn_validate.setStyleSheet("font-weight: bold; background-color: #f0f4f8; padding: 4px 12px; border-radius: 4px;")
        self.btn_validate.setToolTip("Verify syntax and check for parse errors")
        self.btn_validate.clicked.connect(self.validate_current_query)
        preset_bar.addWidget(self.btn_validate)

        top_layout.addLayout(preset_bar)

        # 2. Query Text Editor
        self.txt_query = QPlainTextEdit()
        self.txt_query.setPlaceholderText(
            "Enter CQL / Search DSL query (e.g. `checkmate and attacks(N, k)` or `piece B on [a1..h8]`)..."
        )
        font = QFont("Consolas, Courier New, monospace", 10)
        self.txt_query.setFont(font)
        self.txt_query.setMaximumHeight(90)
        self.txt_query.setStyleSheet(
            "QPlainTextEdit { background-color: #1e1e1e; color: #dcdcdc; border-radius: 4px; padding: 6px; }"
        )
        self.txt_query.textChanged.connect(self.on_query_text_changed)
        top_layout.addWidget(self.txt_query)

        # 3. Validation Status Label
        self.lbl_validation = QLabel("Ready")
        self.lbl_validation.setStyleSheet("color: #666; font-size: 11px;")
        top_layout.addWidget(self.lbl_validation)

        # 4. Search Scope & Run Bar
        run_bar = QHBoxLayout()
        run_bar.addWidget(QLabel("🎯 Target Source:"))

        self.btn_open_db = QPushButton("📂 Open Database...")
        self.btn_open_db.setStyleSheet(
            "font-weight: bold; background-color: #eff6ff; color: #1d4ed8; padding: 4px 12px; border-radius: 4px; border: 1px solid #bfdbfe;"
        )
        self.btn_open_db.setToolTip("Open a SCID database (.si5, .si4) or PGN file (.pgn)")
        self.btn_open_db.clicked.connect(self.open_database_dialog)
        run_bar.addWidget(self.btn_open_db)

        self.radio_active_db = QRadioButton("Currently Open Database")
        self.radio_active_db.setChecked(True)
        self.radio_active_db.setStyleSheet("font-weight: bold; color: #1e3a8a;")
        run_bar.addWidget(self.radio_active_db)

        self.radio_custom_pgn = QRadioButton("Custom PGN File:")
        run_bar.addWidget(self.radio_custom_pgn)

        self.txt_pgn_path = QPlainTextEdit()
        self.txt_pgn_path.setMaximumHeight(28)
        self.txt_pgn_path.setPlaceholderText("Browse or enter path to .pgn file...")
        self.txt_pgn_path.setEnabled(False)
        run_bar.addWidget(self.txt_pgn_path, stretch=1)

        self.btn_browse_pgn = QPushButton("Browse...")
        self.btn_browse_pgn.setEnabled(False)
        self.btn_browse_pgn.clicked.connect(self.browse_pgn_file)
        run_bar.addWidget(self.btn_browse_pgn)

        self.radio_custom_pgn.toggled.connect(self.on_source_mode_changed)

        run_bar.addWidget(QLabel("Limit:"))
        self.spin_limit = QSpinBox()
        self.spin_limit.setRange(10, 100000)
        self.spin_limit.setValue(500)
        run_bar.addWidget(self.spin_limit)

        self.btn_search = QPushButton("▶ Run Search")
        self.btn_search.setStyleSheet(
            "font-weight: bold; background-color: #1976d2; color: white; padding: 6px 18px; border-radius: 4px;"
        )
        self.btn_search.clicked.connect(self.execute_search)
        run_bar.addWidget(self.btn_search)

        top_layout.addLayout(run_bar)
        main_layout.addWidget(top_box)

        # Bottom Splitter: Results Table on Left, Interactive Board & PGN on Right
        splitter = QSplitter(Qt.Horizontal)

        # Left Container: Results Table
        results_widget = QWidget()
        res_layout = QVBoxLayout(results_widget)
        res_layout.setContentsMargins(0, 0, 0, 0)
        res_layout.setSpacing(4)

        self.lbl_results_header = QLabel("Search Results (0 matches)")
        self.lbl_results_header.setStyleSheet("font-weight: bold; font-size: 12px; color: #333;")
        res_layout.addWidget(self.lbl_results_header)

        self.progress_bar = QProgressBar()
        self.progress_bar.setRange(0, 100)
        self.progress_bar.setValue(0)
        self.progress_bar.setTextVisible(True)
        self.progress_bar.setVisible(False)
        self.progress_bar.setStyleSheet("""
            QProgressBar {
                border: 1px solid #ccc;
                border-radius: 4px;
                text-align: center;
                height: 18px;
                font-weight: bold;
                font-size: 11px;
            }
            QProgressBar::chunk {
                background-color: #1976d2;
                border-radius: 3px;
            }
        """)
        res_layout.addWidget(self.progress_bar)

        self.table_results = QTableWidget()
        self.table_results.setColumnCount(8)
        self.table_results.setHorizontalHeaderLabels([
            "#", "White", "Black", "Result", "Date", "Event", "Matches", "First Ply"
        ])
        self.table_results.horizontalHeader().setSectionResizeMode(QHeaderView.Interactive)
        self.table_results.horizontalHeader().setStretchLastSection(True)
        self.table_results.setSelectionBehavior(QTableWidget.SelectRows)
        self.table_results.setSelectionMode(QTableWidget.SingleSelection)
        self.table_results.setEditTriggers(QTableWidget.NoEditTriggers)
        self.table_results.itemSelectionChanged.connect(self.on_result_row_selected)
        self.table_results.cellClicked.connect(lambda _r, _c: self.on_result_row_selected())
        res_layout.addWidget(self.table_results)

        splitter.addWidget(results_widget)

        # Right Container: Board Preview & Game Moves
        right_container = QWidget()
        right_layout = QVBoxLayout(right_container)
        right_layout.setContentsMargins(4, 0, 4, 0)
        right_layout.setSpacing(6)

        # Board Display Label
        self.lbl_board_title = QLabel("Position Preview")
        self.lbl_board_title.setStyleSheet("font-weight: bold; font-size: 12px;")
        right_layout.addWidget(self.lbl_board_title)

        self.board_label = QLabel()
        self.board_label.setMinimumSize(180, 180)
        self.board_label.setMaximumSize(280, 280)
        self.board_label.setStyleSheet("border: 1px solid #ccc; background-color: #f7f7f7;")
        self.board_label.setAlignment(Qt.AlignCenter)
        right_layout.addWidget(self.board_label, alignment=Qt.AlignCenter)

        # Move Navigation Bar
        nav_bar = QHBoxLayout()
        self.btn_first = QPushButton("|<")
        self.btn_first.setToolTip("Start of game")
        self.btn_first.clicked.connect(self.go_first_move)
        nav_bar.addWidget(self.btn_first)

        self.btn_prev = QPushButton("<")
        self.btn_prev.setToolTip("Previous move (Left arrow)")
        self.btn_prev.clicked.connect(self.go_prev_move)
        nav_bar.addWidget(self.btn_prev)

        self.lbl_ply = QLabel("Ply: 0")
        self.lbl_ply.setAlignment(Qt.AlignCenter)
        self.lbl_ply.setStyleSheet("font-weight: bold;")
        nav_bar.addWidget(self.lbl_ply, stretch=1)

        self.btn_next = QPushButton(">")
        self.btn_next.setToolTip("Next move (Right arrow)")
        self.btn_next.clicked.connect(self.go_next_move)
        nav_bar.addWidget(self.btn_next)

        self.btn_last = QPushButton(">|")
        self.btn_last.setToolTip("End of game")
        self.btn_last.clicked.connect(self.go_last_move)
        nav_bar.addWidget(self.btn_last)

        self.btn_jump_match = QPushButton("🎯 Match")
        self.btn_jump_match.setToolTip("Jump to matching tactical/mating position")
        self.btn_jump_match.setStyleSheet("font-weight: bold; color: #1565c0;")
        self.btn_jump_match.clicked.connect(self.jump_to_match_ply)
        nav_bar.addWidget(self.btn_jump_match)

        # Right Bottom: Tabbed panel with Legal Moves & Game PGN
        self.right_tabs = QTabWidget()

        # Tab 1: Legal Moves & Hypothetical Candidate Outcomes
        legal_tab = QWidget()
        legal_layout = QVBoxLayout(legal_tab)
        legal_layout.setContentsMargins(4, 4, 4, 4)
        legal_layout.setSpacing(4)

        self.lbl_legal_summary = QLabel("Legal Moves: 0")
        self.lbl_legal_summary.setStyleSheet("font-weight: bold; font-size: 11px; color: #444;")
        legal_layout.addWidget(self.lbl_legal_summary)

        self.list_legal_moves = QListWidget()
        self.list_legal_moves.setStyleSheet("""
            QListWidget {
                background-color: #ffffff;
                border: 1px solid #ddd;
                border-radius: 4px;
                font-family: Consolas, "Courier New", monospace;
                font-size: 11px;
            }
            QListWidget::item {
                padding: 4px 6px;
                border-bottom: 1px solid #f0f0f0;
                border-radius: 3px;
            }
            QListWidget::item:selected {
                background-color: #e0f2fe;
                color: #0369a1;
            }
        """)
        self.list_legal_moves.itemClicked.connect(self.on_legal_move_clicked)
        legal_layout.addWidget(self.list_legal_moves, stretch=1)

        self.right_tabs.addTab(legal_tab, "⚡ Legal Moves")

        # Tab 2: Full Game PGN Viewer
        self.txt_pgn = QTextEdit()
        self.txt_pgn.setReadOnly(True)
        self.txt_pgn.setFont(QFont("Consolas, Courier New, monospace", 9))
        self.txt_pgn.setStyleSheet("background-color: #ffffff; color: #222; border: 1px solid #ddd; border-radius: 4px;")
        self.right_tabs.addTab(self.txt_pgn, "📜 Game PGN")

        right_layout.addWidget(self.right_tabs, stretch=1)

        splitter.addWidget(right_container)
        splitter.setStretchFactor(0, 3)
        splitter.setStretchFactor(1, 2)

        main_layout.addWidget(splitter, stretch=1)

        self.table_results.installEventFilter(self)
        self.txt_pgn.installEventFilter(self)
        self.list_legal_moves.installEventFilter(self)

        self.update_board_display(chess.Board())
        self.update_legal_moves_display(chess.Board(), None)

    def eventFilter(self, obj, event):
        if event.type() == event.KeyPress:
            if event.key() == Qt.Key_Left:
                self.go_prev_move()
                return True
            elif event.key() == Qt.Key_Right:
                self.go_next_move()
                return True
            elif event.key() == Qt.Key_Home and obj is not self.txt_pgn:
                self.go_first_move()
                return True
            elif event.key() == Qt.Key_End and obj is not self.txt_pgn:
                self.go_last_move()
                return True
        return super().eventFilter(obj, event)

    def keyPressEvent(self, event):
        if event.key() == Qt.Key_Left:
            self.go_prev_move()
            event.accept()
        elif event.key() == Qt.Key_Right:
            self.go_next_move()
            event.accept()
        elif event.key() == Qt.Key_Home:
            self.go_first_move()
            event.accept()
        elif event.key() == Qt.Key_End:
            self.go_last_move()
            event.accept()
        else:
            super().keyPressEvent(event)

    def on_preset_selected(self, index: int):
        query = self.combo_presets.currentData()
        if query:
            self.txt_query.setPlainText(query)
            self.validate_current_query()

    def on_query_text_changed(self):
        self.lbl_validation.setText("Query modified (click Validate or Run Search)")
        self.lbl_validation.setStyleSheet("color: #888; font-size: 11px;")

    def on_source_mode_changed(self, is_custom_pgn: bool):
        self.txt_pgn_path.setEnabled(is_custom_pgn)
        self.btn_browse_pgn.setEnabled(is_custom_pgn)

    def browse_pgn_file(self):
        path, _ = QFileDialog.getOpenFileName(self, "Select PGN File", "", "PGN Files (*.pgn);;All Files (*)")
        if path:
            self.txt_pgn_path.setPlainText(path)

    def open_database_dialog(self):
        path, _ = QFileDialog.getOpenFileName(
            self,
            "Open Chess Database",
            "",
            "Chess Databases (*.si5 *.si4 *.pgn);;SCID 5 (*.si5);;SCID 4 (*.si4);;PGN Files (*.pgn);;All Files (*)"
        )
        if path:
            self.open_database(path)

    def open_database(self, path: str):
        if not path or not os.path.exists(path):
            QMessageBox.warning(self, "Invalid Database", f"Database file not found: {path}")
            return

        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Backend engine is not running.")
            return

        self.lbl_results_header.setText(f"Opening database: {os.path.basename(path)}...")

        def on_db_opened(resp: dict):
            if resp.get("status") == "ok":
                data = resp.get("data", {})
                stats = data.get("stats", {})
                fmt = stats.get("format") or data.get("format", "db")
                total_games = stats.get("total_games") or data.get("total_games", 0)

                self.current_db_path = path
                base_name = os.path.basename(path)
                self.radio_active_db.setText(f"Active DB: {base_name} ({total_games:,} games) [{fmt.upper()}]")
                self.radio_active_db.setToolTip(f"Full path: {path}\nFormat: {fmt.upper()}\nGames: {total_games:,}")
                self.radio_active_db.setChecked(True)

                self.lbl_results_header.setText(f"📁 Loaded {base_name} ({total_games:,} games)")
                self.clear_game_display()
                self.populate_results_table([])
            else:
                err = resp.get("error", "Failed to open database")
                self.lbl_results_header.setText(f"Error opening database: {err}")
                QMessageBox.critical(self, "Open Database Failed", f"Failed to open {path}:\n\n{err}")

        self.client.send_request("open_db", {"path": path}, on_db_opened)

    def explain_current_query(self):
        query = self.txt_query.toPlainText().strip()
        if not query:
            QMessageBox.warning(self, "Empty Query", "Please enter a query to explain.")
            return

        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Backend engine is not running.")
            return

        def on_explain_done(resp: dict):
            if resp.get("status") == "ok":
                explanation = resp.get("data", {})
                dialog = CqlExplainDialog(explanation, parent=self)
                dialog.exec_()
            else:
                err = resp.get("error", "Failed to explain query")
                QMessageBox.critical(self, "Explain Error", f"Failed to explain query:\n\n{err}")

        self.client.send_request("explain_dsl", {"query": query}, on_explain_done)

    def validate_current_query(self):
        query = self.txt_query.toPlainText().strip()
        if not query:
            self.lbl_validation.setText("Query is empty")
            self.lbl_validation.setStyleSheet("color: #888;")
            return

        if not self.client.is_running():
            self.lbl_validation.setText("⚠️ Backend is not running")
            self.lbl_validation.setStyleSheet("color: #d97706;")
            return

        def on_valid_resp(resp: dict):
            if resp.get("status") == "ok":
                self.lbl_validation.setText("✓ Valid Search Query Syntax")
                self.lbl_validation.setStyleSheet("color: #15803d; font-weight: bold;")
                self.lbl_validation.setToolTip("")
            else:
                data = resp.get("data") or {}
                err = resp.get("error", "Unknown parse error")
                line = data.get("line")
                col = data.get("column")
                snippet = data.get("snippet")
                help_msg = data.get("help")

                if line is not None and col is not None:
                    # Show concise one-line summary in label, and full details in tooltip
                    err_first_line = err.splitlines()[0] if err else "Parse error"
                    self.lbl_validation.setText(f"✗ Line {line}, Col {col}: {err_first_line}")
                else:
                    self.lbl_validation.setText(f"✗ {err.splitlines()[0] if err else 'Parse error'}")

                self.lbl_validation.setStyleSheet("color: #b91c1c; font-weight: bold;")

                tooltip_parts = []
                if err:
                    tooltip_parts.append(err)
                if help_msg and help_msg not in err:
                    tooltip_parts.append(f"\n💡 Hint: {help_msg}")
                self.lbl_validation.setToolTip("\n".join(tooltip_parts))

        self.client.send_request("validate_dsl", {"query": query}, on_valid_resp)

    def execute_search(self):
        query = self.txt_query.toPlainText().strip()
        if not query:
            QMessageBox.warning(self, "Empty Query", "Please enter a query or select a preset theme.")
            return

        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Please start the backend engine first.")
            return

        params = {
            "query": query,
            "limit": self.spin_limit.value()
        }

        if self.radio_custom_pgn.isChecked():
            pgn_path = self.txt_pgn_path.toPlainText().strip()
            if not pgn_path or not os.path.exists(pgn_path):
                QMessageBox.warning(self, "Invalid File", "Please select a valid existing PGN file.")
                return
            params["pgn_path"] = pgn_path

        self.btn_search.setEnabled(False)
        self.btn_search.setText("Searching...")
        self.progress_bar.setValue(0)
        self.progress_bar.setVisible(True)
        self.lbl_results_header.setText("🔍 Initializing search scanner...")

        def on_search_done(resp: dict):
            self.btn_search.setEnabled(True)
            self.btn_search.setText("▶ Run Search")
            self.progress_bar.setValue(100)
            self.progress_bar.setVisible(False)

            if resp.get("status") != "ok":
                err = resp.get("error", "Search failed")
                self.lbl_results_header.setText(f"Error: {err}")
                QMessageBox.critical(self, "Search Failed", f"Search error:\n{err}")
                return

            data = resp.get("data", {})
            total_searched = data.get("total_searched", 0)
            matched_count = data.get("matched_count", 0)
            duration_ms = data.get("duration_ms", 0)
            self.matches_data = data.get("matches", [])

            self.lbl_results_header.setText(
                f"✅ Found {matched_count} matching games across {total_searched:,} games in {duration_ms} ms"
            )

            self.populate_results_table(self.matches_data)

        self.client.send_request("dsl_search", params, on_search_done)

    def on_backend_event(self, event: str, data: dict):
        if event == "search_progress":
            scanned = data.get("scanned", 0)
            total = data.get("total", 0)
            matches = data.get("matches", 0)
            pct = data.get("percent", 0.0)
            self.progress_bar.setVisible(True)
            self.progress_bar.setValue(min(100, int(pct)))
            self.lbl_results_header.setText(
                f"🔍 Searching... Scanned {scanned:,} / {total:,} games ({pct:.1f}%) — Found {matches:,} matches"
            )

    def populate_results_table(self, matches: List[Dict[str, Any]]):
        self.table_results.blockSignals(True)
        self.table_results.clearSelection()
        self.table_results.setRowCount(len(matches))
        for row, item in enumerate(matches):
            game_id = str(item.get("game_id", row + 1))
            white = item.get("white", "?")
            black = item.get("black", "?")
            result = item.get("result", "*")
            date = item.get("date", "????.??.??")
            event = item.get("event", "?")
            match_count = str(item.get("match_count", 1))
            matching_plies = item.get("matching_plies", [])
            first_ply = str(matching_plies[0]) if matching_plies else "-"

            self.table_results.setItem(row, 0, QTableWidgetItem(game_id))
            self.table_results.setItem(row, 1, QTableWidgetItem(white))
            self.table_results.setItem(row, 2, QTableWidgetItem(black))
            self.table_results.setItem(row, 3, QTableWidgetItem(result))
            self.table_results.setItem(row, 4, QTableWidgetItem(date))
            self.table_results.setItem(row, 5, QTableWidgetItem(event))
            self.table_results.setItem(row, 6, QTableWidgetItem(match_count))
            self.table_results.setItem(row, 7, QTableWidgetItem(first_ply))

        self.table_results.blockSignals(False)

        if matches:
            self.table_results.selectRow(0)
            self.on_result_row_selected()
        else:
            self.clear_game_display()

    def clear_game_display(self):
        self.current_game_pgn = ""
        self.txt_pgn.clear()
        self.current_plies_list = []
        self.current_ply_index = 0
        self.game_moves = []
        self.game_positions = [chess.Board()]
        self.lbl_ply.setText("Ply: 0")
        self.lbl_ply.setStyleSheet("font-weight: bold; color: #333;")
        self.lbl_board_title.setText("Position Preview")
        self.update_board_display(chess.Board())
        self.update_legal_moves_display(chess.Board(), None)

    def on_result_row_selected(self):
        row = self.table_results.currentRow()
        if row < 0:
            if self.matches_data:
                row = 0
            else:
                return
        if row >= len(self.matches_data):
            return

        match_item = self.matches_data[row]
        game_id = match_item.get("game_id")
        matching_fen = match_item.get("matching_fen")
        matching_plies = match_item.get("matching_plies", [])
        pgn_text = match_item.get("pgn")

        # If PGN text is directly embedded in match response, display immediately
        if pgn_text:
            self.display_game(pgn_text, matching_plies, matching_fen)
            return

        # Otherwise fallback to backend get_pgn
        def on_pgn_loaded(resp: dict):
            if resp.get("status") == "ok":
                text = resp.get("data", {}).get("pgn", "")
                self.display_game(text, matching_plies, matching_fen)

        self.client.send_request("get_pgn", {"index": game_id}, on_pgn_loaded)

    def display_game(self, pgn_text: str, matching_plies: List[int], matching_fen: Optional[str]):
        self.current_game_pgn = pgn_text
        self.txt_pgn.setPlainText(pgn_text)
        self.current_plies_list = matching_plies

        # Replay game moves into positions array
        self.game_moves = []
        self.game_positions = [chess.Board()]

        try:
            game = chess.pgn.read_game(io.StringIO(pgn_text))
            if game:
                board = game.board()
                self.game_positions = [board.copy()]
                for move in game.mainline_moves():
                    board.push(move)
                    self.game_moves.append(move)
                    self.game_positions.append(board.copy())
        except Exception:
            pass

        # If no positions were extracted, use matching_fen
        if len(self.game_positions) <= 1 and matching_fen:
            try:
                self.game_positions = [chess.Board(matching_fen)]
            except Exception:
                pass

        # Jump directly to first matching ply
        target_ply = matching_plies[0] if matching_plies else 0
        self.set_ply(target_ply)

    def jump_to_match_ply(self):
        if self.current_plies_list:
            self.set_ply(self.current_plies_list[0])

    def set_ply(self, ply: int):
        if not self.game_positions:
            return

        clamped = max(0, min(ply, len(self.game_positions) - 1))
        self.current_ply_index = clamped
        self.current_board = self.game_positions[clamped]
        self.lbl_board_title.setText("Position Preview")

        # Highlight status if at matching ply
        is_match = clamped in self.current_plies_list
        match_tag = " [🎯 MATCH]" if is_match else ""
        self.lbl_ply.setText(f"Ply: {clamped} / {len(self.game_positions) - 1}{match_tag}")
        if is_match:
            self.lbl_ply.setStyleSheet("font-weight: bold; color: #15803d;")
        else:
            self.lbl_ply.setStyleSheet("font-weight: bold; color: #333;")

        self.update_board_display(self.current_board)

        played_move = self.game_moves[clamped] if clamped < len(self.game_moves) else None
        self.update_legal_moves_display(self.current_board, played_move)

    def update_legal_moves_display(self, board: chess.Board, played_move: Optional[chess.Move] = None):
        self.list_legal_moves.clear()
        if not board:
            self.lbl_legal_summary.setText("Legal Moves: 0")
            return

        legal_moves = list(board.legal_moves)
        turn_str = "White to move" if board.turn == chess.WHITE else "Black to move"

        def move_sort_key(mv: chess.Move):
            is_played = (mv == played_move)
            b = board.copy()
            b.push(mv)
            is_mate = b.is_checkmate()
            is_stale = b.is_stalemate()
            is_chk = b.is_check()
            return (
                0 if is_played else (1 if is_mate else (2 if is_stale else (3 if is_chk else 4))),
                board.san(mv)
            )

        sorted_moves = sorted(legal_moves, key=move_sort_key)

        mates_count = 0
        stalemates_count = 0
        checks_count = 0

        for mv in sorted_moves:
            san = board.san(mv)
            b = board.copy()
            b.push(mv)
            is_mate = b.is_checkmate()
            is_stale = b.is_stalemate()
            is_chk = b.is_check()
            is_played = (mv == played_move)

            if is_mate:
                mates_count += 1
            if is_stale:
                stalemates_count += 1
            if is_chk:
                checks_count += 1

            tags = []
            if is_played:
                tags.append("🟢 PLAYED")
            if is_mate:
                tags.append("👑 MATE")
            elif is_stale:
                tags.append("⚖️ STALEMATE")
            elif is_chk:
                tags.append("⚡ CHECK")

            tag_str = f" [{', '.join(tags)}]" if tags else ""
            item_text = f"{san:<8} ({mv.uci()}){tag_str}"
            item = QListWidgetItem(item_text)

            # Color coding
            if is_played:
                item.setBackground(QColor("#dcfce7"))  # light green
                item.setForeground(QColor("#15803d"))  # dark green
                f = item.font()
                f.setBold(True)
                item.setFont(f)
            elif is_mate:
                item.setBackground(QColor("#fee2e2"))  # light red
                item.setForeground(QColor("#b91c1c"))  # dark red/crimson
                f = item.font()
                f.setBold(True)
                item.setFont(f)
            elif is_stale:
                item.setBackground(QColor("#fef3c7"))  # amber
                item.setForeground(QColor("#b45309"))  # dark amber
                f = item.font()
                f.setBold(True)
                item.setFont(f)
            elif is_chk:
                item.setForeground(QColor("#1d4ed8"))  # blue

            item.setData(Qt.UserRole, mv)
            self.list_legal_moves.addItem(item)

        played_str = f" | Played: <b style='color: #15803d;'>{board.san(played_move)}</b>" if played_move and played_move in board.legal_moves else ""
        mate_str = f" | <span style='color: #b91c1c; font-weight: bold;'>👑 Mates: {mates_count}</span>" if mates_count > 0 else ""
        stale_str = f" | <span style='color: #b45309; font-weight: bold;'>⚖️ Stalemates: {stalemates_count}</span>" if stalemates_count > 0 else ""

        self.lbl_legal_summary.setText(
            f"<b>{turn_str}</b> | <b>{len(legal_moves)}</b> legal move(s){played_str}{mate_str}{stale_str}"
        )
        self.right_tabs.setTabText(0, f"⚡ Legal Moves ({len(legal_moves)})")

    def on_legal_move_clicked(self, item: QListWidgetItem):
        mv = item.data(Qt.UserRole)
        if mv and self.current_board and mv in self.current_board.legal_moves:
            hyp_board = self.current_board.copy()
            hyp_board.push(mv)
            self.update_board_display(hyp_board)
            san = self.current_board.san(mv)
            self.lbl_board_title.setText(f"💡 Hypothetical: {san} ({mv.uci()}) [Use nav arrow to return]")

    def go_first_move(self):
        self.set_ply(0)

    def go_prev_move(self):
        self.set_ply(self.current_ply_index - 1)

    def go_next_move(self):
        self.set_ply(self.current_ply_index + 1)

    def go_last_move(self):
        if self.game_positions:
            self.set_ply(len(self.game_positions) - 1)

    def update_board_display(self, board: chess.Board):
        try:
            sz = max(180, min(self.board_label.width() or 240, self.board_label.height() or 240))
            svg_data = chess.svg.board(board=board, size=sz)
            renderer = QSvgRenderer(QByteArray(svg_data.encode("utf-8")))
            pixmap = QPixmap(sz, sz)
            pixmap.fill(Qt.transparent)
            painter = QPainter(pixmap)
            renderer.render(painter)
            painter.end()
            self.board_label.setPixmap(pixmap)
        except Exception as e:
            self.board_label.setText(str(e))
