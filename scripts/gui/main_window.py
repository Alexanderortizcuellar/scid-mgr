import os
import sys
import json
from typing import Optional

from PyQt5.QtCore import Qt, QTimer, QSettings
from PyQt5.QtGui import QFont, QColor
from PyQt5.QtWidgets import (
    QMainWindow, QWidget, QVBoxLayout, QHBoxLayout, QGridLayout, QSplitter,
    QLabel, QLineEdit, QPushButton, QComboBox, QTableView, QHeaderView,
    QTextEdit, QGroupBox, QFileDialog, QMessageBox, QTabWidget, QStatusBar,
    QFrame, QCheckBox, QDialog, QMenu, QAction, QApplication
)

from .backend_client import BackendClient
from .models import VirtualScidTableModel
from .widgets.opening_tree_widget import OpeningTreeWidget
from .dialogs.new_db_dialog import NewDatabaseDialog
from .dialogs.add_edit_game_dialog import AddEditGameDialog
from .dialogs.advanced_search_dialog import AdvancedSearchDialog
from .dialogs.benchmark_dialog import BenchmarkDialog
from .dialogs.columns_dialog import ColumnsConfigDialog
from .dialogs.build_pos_index_dialog import BuildPosIndexDialog

class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("SCID Chess Database Manager (chess-scid-rw)")
        self.resize(1340, 880)

        self.client = BackendClient(self)
        self.client.response_received.connect(self.on_response_received)
        self.client.process_error.connect(self.on_process_error)
        self.client.process_stopped.connect(self.on_process_stopped)

        self.table_model = VirtualScidTableModel(self.client, self)
        self.table_model.stats_updated.connect(self.on_model_stats_updated)

        self.selected_game_id: Optional[int] = None
        self.current_db_stats: Optional[dict] = None
        self.current_material_filter: Optional[dict] = None

        # 150ms scroll debounce timer
        self.scroll_timer = QTimer(self)
        self.scroll_timer.setSingleShot(True)
        self.scroll_timer.setInterval(150)
        self.scroll_timer.timeout.connect(self.on_scroll_settled)

        self.init_ui()
        self.auto_detect_defaults()

    def init_ui(self):
        main_widget = QWidget()
        self.setCentralWidget(main_widget)
        root_layout = QVBoxLayout(main_widget)
        root_layout.setContentsMargins(10, 10, 10, 10)
        root_layout.setSpacing(8)

        # 1. Connection & Database Management Group
        conn_group = QGroupBox("Backend Connection & SCID Database")
        conn_layout = QGridLayout(conn_group)
        conn_layout.setContentsMargins(10, 8, 10, 8)
        conn_layout.setSpacing(6)

        # Binary Path
        conn_layout.addWidget(QLabel("scid-mgr Binary:"), 0, 0)
        self.binary_input = QLineEdit()
        conn_layout.addWidget(self.binary_input, 0, 1)
        btn_browse_bin = QPushButton("Browse...")
        btn_browse_bin.clicked.connect(self.browse_binary)
        conn_layout.addWidget(btn_browse_bin, 0, 2)

        # Database Path (SCID or PGN)
        conn_layout.addWidget(QLabel("Chess DB / PGN:"), 1, 0)
        self.db_input = QLineEdit()
        self.db_input.setPlaceholderText("Select .si5, .si4, or .pgn file...")
        conn_layout.addWidget(self.db_input, 1, 1)
        btn_browse_db = QPushButton("Open DB / PGN...")
        btn_browse_db.clicked.connect(self.browse_db)
        conn_layout.addWidget(btn_browse_db, 1, 2)

        # SCID C++ Engine (Optional Legacy)
        conn_layout.addWidget(QLabel("SCID C++ (Optional):"), 2, 0)
        scid_cpp_row = QHBoxLayout()
        self.scid_cpp_input = QLineEdit()
        scid_cpp_row.addWidget(self.scid_cpp_input)
        self.chk_use_scid_cpp = QCheckBox("Use external SCID C++ binary instead of Native Rust (~1.2s)")
        self.chk_use_scid_cpp.setChecked(False)
        self.chk_use_scid_cpp.setStyleSheet("color: #666;")
        scid_cpp_row.addWidget(self.chk_use_scid_cpp)
        conn_layout.addLayout(scid_cpp_row, 2, 1)
        btn_browse_scid = QPushButton("Browse...")
        btn_browse_scid.clicked.connect(self.browse_scid_cpp)
        conn_layout.addWidget(btn_browse_scid, 2, 2)

        # Database action buttons row
        db_actions_layout = QHBoxLayout()
        self.btn_connect = QPushButton("Start Backend")
        self.btn_connect.setStyleSheet("font-weight: bold; background-color: #2e7d32; color: white; padding: 6px 12px;")
        self.btn_connect.clicked.connect(self.toggle_backend)
        db_actions_layout.addWidget(self.btn_connect)

        self.btn_new_db = QPushButton("New DB...")
        self.btn_new_db.clicked.connect(self.create_new_db)
        db_actions_layout.addWidget(self.btn_new_db)

        self.btn_import_pgn = QPushButton("Import PGN...")
        self.btn_import_pgn.clicked.connect(self.import_pgn)
        db_actions_layout.addWidget(self.btn_import_pgn)

        self.btn_export_pgn = QPushButton("Export PGN...")
        self.btn_export_pgn.clicked.connect(self.export_pgn)
        db_actions_layout.addWidget(self.btn_export_pgn)

        self.btn_compact = QPushButton("Compact DB")
        self.btn_compact.clicked.connect(self.compact_db)
        db_actions_layout.addWidget(self.btn_compact)

        self.btn_save = QPushButton("Save DB")
        self.btn_save.setStyleSheet("font-weight: bold; background-color: #0288d1; color: white; padding: 6px 12px;")
        self.btn_save.clicked.connect(self.save_db)
        db_actions_layout.addWidget(self.btn_save)

        self.btn_benchmark = QPushButton("📊 Metrics / Benchmark...")
        self.btn_benchmark.setStyleSheet("font-weight: bold; padding: 6px 12px;")
        self.btn_benchmark.clicked.connect(self.open_benchmark_dialog)
        db_actions_layout.addWidget(self.btn_benchmark)

        conn_layout.addLayout(db_actions_layout, 3, 0, 1, 3)
        root_layout.addWidget(conn_group)

        # 2. Database Stats Bar
        self.stats_bar = QFrame()
        self.stats_bar.setFrameShape(QFrame.StyledPanel)
        stats_layout = QHBoxLayout(self.stats_bar)
        stats_layout.setContentsMargins(10, 4, 10, 4)

        self.lbl_status = QLabel("Status: Disconnected")
        self.lbl_status.setStyleSheet("font-weight: bold; color: #d32f2f;")
        stats_layout.addWidget(self.lbl_status)

        stats_layout.addSpacing(15)
        self.lbl_format = QLabel("Format: -")
        stats_layout.addWidget(self.lbl_format)

        stats_layout.addSpacing(15)
        self.lbl_games_count = QLabel("Total Games: -")
        stats_layout.addWidget(self.lbl_games_count)

        stats_layout.addSpacing(15)
        self.lbl_active_count = QLabel("Active: -")
        stats_layout.addWidget(self.lbl_active_count)

        stats_layout.addSpacing(15)
        self.lbl_deleted_count = QLabel("Deleted: -")
        stats_layout.addWidget(self.lbl_deleted_count)

        stats_layout.addSpacing(15)
        self.lbl_players_count = QLabel("Players: -")
        stats_layout.addWidget(self.lbl_players_count)

        stats_layout.addSpacing(15)
        self.lbl_events_count = QLabel("Events: -")
        stats_layout.addWidget(self.lbl_events_count)

        stats_layout.addSpacing(15)
        self.btn_pos_index = QPushButton("⚡ Build Fast Index")
        self.btn_pos_index.setStyleSheet("font-weight: bold; font-size: 11px; padding: 2px 8px; border-radius: 3px;")
        self.btn_pos_index.setToolTip("Position Companion Index (.pos.idx) for sub-millisecond opening tree & position search")
        self.btn_pos_index.clicked.connect(self.prompt_build_pos_index)
        stats_layout.addWidget(self.btn_pos_index)

        stats_layout.addStretch()
        btn_refresh_info = QPushButton("Refresh Info")
        btn_refresh_info.clicked.connect(self.refresh_database_info)
        stats_layout.addWidget(btn_refresh_info)

        root_layout.addWidget(self.stats_bar)

        # 3. Filters Group
        filters_group = QGroupBox("Search & Filters")
        filters_layout = QGridLayout(filters_group)
        filters_layout.setContentsMargins(10, 8, 10, 8)
        filters_layout.setSpacing(6)

        # Player search
        filters_layout.addWidget(QLabel("Player (Any):"), 0, 0)
        self.filter_player = QLineEdit()
        self.filter_player.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_player, 0, 1)

        filters_layout.addWidget(QLabel("White:"), 0, 2)
        self.filter_white = QLineEdit()
        self.filter_white.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_white, 0, 3)

        filters_layout.addWidget(QLabel("Black:"), 0, 4)
        self.filter_black = QLineEdit()
        self.filter_black.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_black, 0, 5)

        # Result & ECO & Date
        filters_layout.addWidget(QLabel("Result:"), 1, 0)
        self.filter_result = QComboBox()
        self.filter_result.addItems(["All", "1-0", "0-1", "1/2-1/2", "*"])
        filters_layout.addWidget(self.filter_result, 1, 1)

        filters_layout.addWidget(QLabel("ECO Code:"), 1, 2)
        self.filter_eco = QLineEdit()
        self.filter_eco.setPlaceholderText("e.g. B85 or C")
        self.filter_eco.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_eco, 1, 3)

        filters_layout.addWidget(QLabel("Date:"), 1, 4)
        self.filter_date = QLineEdit()
        self.filter_date.setPlaceholderText("YYYY.MM.DD")
        self.filter_date.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_date, 1, 5)

        # Event & Site & Deleted flags
        filters_layout.addWidget(QLabel("Event:"), 2, 0)
        self.filter_event = QLineEdit()
        self.filter_event.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_event, 2, 1)

        filters_layout.addWidget(QLabel("Site:"), 2, 2)
        self.filter_site = QLineEdit()
        self.filter_site.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_site, 2, 3)

        flags_layout = QHBoxLayout()
        self.chk_include_deleted = QCheckBox("Include Deleted")
        self.chk_include_deleted.setChecked(True)
        flags_layout.addWidget(self.chk_include_deleted)

        self.chk_only_deleted = QCheckBox("Only Deleted")
        flags_layout.addWidget(self.chk_only_deleted)
        filters_layout.addLayout(flags_layout, 2, 4, 1, 2)

        # Position (FEN) Search Row
        filters_layout.addWidget(QLabel("Position (FEN):"), 3, 0)
        self.filter_fen = QLineEdit()
        self.filter_fen.setPlaceholderText("e.g. rnbqkb1r/1p2pppp/p2p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6")
        self.filter_fen.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_fen, 3, 1, 1, 3)

        # Action buttons
        btn_search_layout = QHBoxLayout()
        self.btn_search = QPushButton("Apply Filters / Search")
        self.btn_search.setStyleSheet("font-weight: bold; background-color: #1976d2; color: white; padding: 5px 15px;")
        self.btn_search.clicked.connect(self.on_search_clicked)
        btn_search_layout.addWidget(self.btn_search)

        self.btn_adv_search = QPushButton("🔍 Advanced Search...")
        self.btn_adv_search.setStyleSheet("font-weight: bold; background-color: #6a1b9a; color: white; padding: 5px 12px;")
        self.btn_adv_search.clicked.connect(self.open_advanced_search)
        btn_search_layout.addWidget(self.btn_adv_search)

        btn_reset = QPushButton("Reset")
        btn_reset.clicked.connect(self.reset_filters)
        btn_search_layout.addWidget(btn_reset)

        filters_layout.addLayout(btn_search_layout, 3, 4, 1, 2)
        root_layout.addWidget(filters_group)

        # 4. Main Splitter: Virtual Table on Left, Details & Logs on Right
        splitter = QSplitter(Qt.Horizontal)

        # Left Container: Virtual Games Table
        left_container = QWidget()
        left_layout = QVBoxLayout(left_container)
        left_layout.setContentsMargins(0, 0, 0, 0)

        self.table_view = QTableView()
        self.table_view.setModel(self.table_model)
        self.table_view.setSelectionBehavior(QTableView.SelectRows)
        self.table_view.setSelectionMode(QTableView.SingleSelection)

        header = self.table_view.horizontalHeader()
        header.setSectionResizeMode(QHeaderView.Interactive)
        header.setSectionsClickable(True)
        header.setStretchLastSection(True)
        header.sectionClicked.connect(self.table_model.toggle_sort_column)
        header.setContextMenuPolicy(Qt.CustomContextMenu)
        header.customContextMenuRequested.connect(self.show_header_context_menu)

        self.table_view.verticalHeader().setDefaultSectionSize(26)
        self.table_view.selectionModel().selectionChanged.connect(self.on_table_selection_changed)
        self.table_view.verticalScrollBar().valueChanged.connect(self.on_scroll_changed)
        left_layout.addWidget(self.table_view)

        self._set_default_column_widths()
        self.load_column_settings()

        # Virtual Scroll Status Bar
        vscroll_bar = QHBoxLayout()
        self.lbl_vscroll_info = QLabel("Matching Games: 0 | Cached: 0 | ⚡ Virtual Scrolling Active")
        self.lbl_vscroll_info.setStyleSheet("color: #444; font-size: 11px; padding: 2px;")
        vscroll_bar.addWidget(self.lbl_vscroll_info)
        vscroll_bar.addStretch()

        btn_columns = QPushButton("⚙ Columns...")
        btn_columns.setStyleSheet("font-size: 11px; padding: 2px 8px;")
        btn_columns.setToolTip("Configure visible columns")
        btn_columns.clicked.connect(self.open_columns_dialog)
        vscroll_bar.addWidget(btn_columns)

        left_layout.addLayout(vscroll_bar)
        splitter.addWidget(left_container)

        # Right Container: Tabs (Raw PGN Viewer + Metadata + JSON Protocol Log)
        self.tabs = QTabWidget()

        # PGN Tab
        pgn_widget = QWidget()
        pgn_layout = QVBoxLayout(pgn_widget)
        pgn_layout.setContentsMargins(6, 6, 6, 6)

        pgn_header_layout = QHBoxLayout()
        self.lbl_selected_game = QLabel("Selected Game: None")
        self.lbl_selected_game.setStyleSheet("font-weight: bold; font-size: 13px;")
        pgn_header_layout.addWidget(self.lbl_selected_game)
        pgn_header_layout.addStretch()

        self.btn_add_game = QPushButton("Add Game...")
        self.btn_add_game.clicked.connect(self.add_game_dialog)
        pgn_header_layout.addWidget(self.btn_add_game)

        self.btn_edit_game = QPushButton("Edit Game...")
        self.btn_edit_game.clicked.connect(self.edit_game_dialog)
        pgn_header_layout.addWidget(self.btn_edit_game)

        self.btn_del_game = QPushButton("Delete")
        self.btn_del_game.setStyleSheet("color: #d32f2f;")
        self.btn_del_game.clicked.connect(self.delete_selected_game)
        pgn_header_layout.addWidget(self.btn_del_game)

        self.btn_undel_game = QPushButton("Undelete")
        self.btn_undel_game.clicked.connect(self.undelete_selected_game)
        pgn_header_layout.addWidget(self.btn_undel_game)

        btn_copy_pgn = QPushButton("Copy PGN")
        btn_copy_pgn.clicked.connect(self.copy_pgn_text)
        pgn_header_layout.addWidget(btn_copy_pgn)

        pgn_layout.addLayout(pgn_header_layout)

        self.pgn_viewer = QTextEdit()
        self.pgn_viewer.setReadOnly(True)
        mono_font = QFont("Consolas" if sys.platform == "win32" else "Monospace", 10)
        self.pgn_viewer.setFont(mono_font)
        pgn_layout.addWidget(self.pgn_viewer)

        self.tabs.addTab(pgn_widget, "PGN Game Text")

        # 🌲 Opening Tree Tab
        self.opening_tree_widget = OpeningTreeWidget(self.client, self)
        self.tabs.addTab(self.opening_tree_widget, "🌲 Opening Tree")
        self.tabs.currentChanged.connect(self.on_tab_changed)

        # JSON Logs Tab
        logs_widget = QWidget()
        logs_layout = QVBoxLayout(logs_widget)
        logs_layout.setContentsMargins(6, 6, 6, 6)

        log_actions = QHBoxLayout()
        btn_clear_logs = QPushButton("Clear Logs")
        btn_clear_logs.clicked.connect(self.clear_logs)
        log_actions.addWidget(btn_clear_logs)
        log_actions.addStretch()
        logs_layout.addLayout(log_actions)

        self.log_viewer = QTextEdit()
        self.log_viewer.setReadOnly(True)
        self.log_viewer.setFont(mono_font)
        logs_layout.addWidget(self.log_viewer)

        self.tabs.addTab(logs_widget, "Protocol Logs")

        splitter.addWidget(self.tabs)
        splitter.setSizes([800, 480])
        root_layout.addWidget(splitter, 1)

        # Status Bar
        self.status_bar = QStatusBar()
        self.setStatusBar(self.status_bar)
        self.status_bar.showMessage("Ready. Select SCID database and start backend.")

    def _set_default_column_widths(self):
        widths = [50, 140, 50, 140, 50, 65, 50, 80, 130, 110, 50, 60]
        for col, w in enumerate(widths):
            self.table_view.setColumnWidth(col, w)

    def show_header_context_menu(self, pos):
        menu = QMenu(self)
        menu.setStyleSheet("font-size: 12px;")

        # Title
        title_action = menu.addAction("👁 Column Visibility:")
        title_action.setEnabled(False)
        menu.addSeparator()

        for col, name in enumerate(VirtualScidTableModel.HEADERS):
            act = QAction(f"{col + 1}. {name}", menu, checkable=True)
            act.setChecked(not self.table_view.isColumnHidden(col))
            act.setData(col)
            act.triggered.connect(lambda checked, c=col: self.toggle_column_visibility(c, checked))
            menu.addAction(act)

        menu.addSeparator()
        act_dialog = menu.addAction("⚙ Configure Columns...")
        act_dialog.triggered.connect(self.open_columns_dialog)

        act_show_all = menu.addAction("Show All Columns")
        act_show_all.triggered.connect(self.show_all_columns)

        act_reset_widths = menu.addAction("Reset Column Widths")
        act_reset_widths.triggered.connect(self._set_default_column_widths)

        menu.exec_(self.table_view.horizontalHeader().mapToGlobal(pos))

    def toggle_column_visibility(self, col: int, visible: bool):
        self.table_view.setColumnHidden(col, not visible)
        self.save_column_settings()

    def show_all_columns(self):
        for col in range(len(VirtualScidTableModel.HEADERS)):
            self.table_view.setColumnHidden(col, False)
        self.save_column_settings()

    def open_columns_dialog(self):
        dlg = ColumnsConfigDialog(self.table_view, VirtualScidTableModel.HEADERS, self)
        dlg.exec_()

    def save_column_settings(self):
        settings = QSettings("ChessScidMgr", "ScidGui")
        hidden_cols = [col for col in range(len(VirtualScidTableModel.HEADERS)) if self.table_view.isColumnHidden(col)]
        settings.setValue("columns_hidden", hidden_cols)

    def load_column_settings(self):
        settings = QSettings("ChessScidMgr", "ScidGui")
        hidden_cols = settings.value("columns_hidden", [])
        if isinstance(hidden_cols, list):
            for col in hidden_cols:
                try:
                    c = int(col)
                    if 0 <= c < len(VirtualScidTableModel.HEADERS):
                        self.table_view.setColumnHidden(c, True)
                except (ValueError, TypeError):
                    pass

    def auto_detect_defaults(self):
        project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        bin_names = ["scid-mgr.exe", "scid-mgr"]
        target_dirs = [
            os.path.join(project_root, "target", "release"),
            os.path.join(project_root, "target", "debug"),
        ]

        for t_dir in target_dirs:
            for b_name in bin_names:
                candidate = os.path.join(t_dir, b_name)
                if os.path.exists(candidate):
                    self.binary_input.setText(candidate)
                    break
            if self.binary_input.text():
                break

        # Auto-detect official SCID C++ engine
        downloads_scid = r"C:\Users\ASUS\Downloads\scid-v5.2.202603_windows_x64\scid_windows_x64\bin\scid.exe"
        scid_candidates = [
            downloads_scid,
            r"C:\Program Files\Scid\bin\scid.exe",
            r"C:\Program Files (x86)\Scid\bin\scid.exe",
        ]
        for scid_cand in scid_candidates:
            if os.path.exists(scid_cand):
                self.scid_cpp_input.setText(scid_cand)
                break

    def browse_scid_cpp(self):
        path, _ = QFileDialog.getOpenFileName(
            self, "Select Official SCID scid.exe Binary", "", "Executables (*.exe);;All Files (*)"
        )
        if path:
            self.scid_cpp_input.setText(path)

    def browse_binary(self):
        path, _ = QFileDialog.getOpenFileName(
            self, "Select scid-mgr Binary", "", "Executables (*.exe);;All Files (*)"
        )
        if path:
            self.binary_input.setText(path)

    def browse_db(self):
        path, _ = QFileDialog.getOpenFileName(
            self,
            "Select Chess Database or PGN File",
            "",
            "Chess Databases (*.si5 *.si4 *.pgn);;SCID Databases (*.si5 *.si4 *.sn5 *.sn4 *.sg5 *.sg4);;PGN Files (*.pgn);;All Files (*)",
        )
        if path:
            self.db_input.setText(path)
            if self.client.is_running():
                self.client.send_request("open", {"path": path})

    def create_new_db(self):
        dialog = NewDatabaseDialog(self)
        if dialog.exec_() == QDialog.Accepted:
            db_path, fmt = dialog.get_data()
            if not db_path:
                return
            self.db_input.setText(db_path)
            if self.client.is_running():
                self.client.send_request("create", {"path": db_path, "format": fmt})
            else:
                bin_path = self.binary_input.text().strip()
                if bin_path and os.path.exists(bin_path):
                    self.start_backend_and_open(db_path, create_format=fmt)

    def toggle_backend(self):
        if self.client.is_running():
            self.client.stop()
            self.update_ui_disconnected()
        else:
            bin_path = self.binary_input.text().strip()
            db_path = self.db_input.text().strip()
            if not bin_path or not os.path.exists(bin_path):
                QMessageBox.warning(self, "Invalid Binary", f"Cannot find binary at:\n{bin_path}")
                return

            try:
                self.client.start(bin_path, db_path if db_path and os.path.exists(db_path) else None)
                self.update_ui_connected()
                self.log_viewer.append(f"[GUI] Spawned backend: {bin_path}")
                if db_path:
                    self.client.send_request("open", {"path": db_path})
                else:
                    self.refresh_database_info()
            except Exception as e:
                QMessageBox.critical(self, "Startup Error", str(e))

    def start_backend_and_open(self, db_path: str, create_format: Optional[str] = None):
        bin_path = self.binary_input.text().strip()
        if not bin_path or not os.path.exists(bin_path):
            return
        self.client.start(bin_path)
        self.update_ui_connected()
        if create_format:
            self.client.send_request("create", {"path": db_path, "format": create_format})
        else:
            self.client.send_request("open", {"path": db_path})

    def update_ui_connected(self):
        self.lbl_status.setText("Status: Connected")
        self.lbl_status.setStyleSheet("font-weight: bold; color: #2e7d32;")
        self.btn_connect.setText("Stop Backend")
        self.btn_connect.setStyleSheet("font-weight: bold; background-color: #d32f2f; color: white; padding: 6px 12px;")

    def update_ui_disconnected(self):
        self.lbl_status.setText("Status: Disconnected")
        self.lbl_status.setStyleSheet("font-weight: bold; color: #d32f2f;")
        self.btn_connect.setText("Start Backend")
        self.btn_connect.setStyleSheet("font-weight: bold; background-color: #2e7d32; color: white; padding: 6px 12px;")
        self.table_model.clear()
        self.lbl_format.setText("Format: -")
        self.lbl_games_count.setText("Total Games: -")
        self.lbl_active_count.setText("Active: -")
        self.lbl_deleted_count.setText("Deleted: -")
        self.lbl_players_count.setText("Players: -")
        self.lbl_events_count.setText("Events: -")
        self.pgn_viewer.clear()
        self.lbl_selected_game.setText("Selected Game: None")

    def refresh_database_info(self):
        if self.client.is_running():
            self.client.send_request("info")

    def on_scroll_changed(self, _val):
        # Restart debounce timer on every scroll tick
        self.scroll_timer.start()

    def on_scroll_settled(self):
        top_row = self.table_view.rowAt(0)
        bottom_row = self.table_view.rowAt(self.table_view.viewport().height())

        if top_row == -1:
            top_row = 0
        if bottom_row == -1:
            bottom_row = min(self.table_model.total_count, top_row + 50)

        self.table_model.request_chunks_for_range(top_row, bottom_row)

    def open_advanced_search(self):
        current = {
            "player": self.filter_player.text().strip(),
            "white": self.filter_white.text().strip(),
            "black": self.filter_black.text().strip(),
            "result": self.filter_result.currentText(),
            "eco": self.filter_eco.text().strip(),
            "date": self.filter_date.text().strip(),
            "event": self.filter_event.text().strip(),
            "site": self.filter_site.text().strip(),
            "include_deleted": self.chk_include_deleted.isChecked(),
            "only_deleted": self.chk_only_deleted.isChecked(),
            "fen": self.filter_fen.text().strip(),
        }
        if self.current_material_filter:
            current["material"] = self.current_material_filter

        dlg = AdvancedSearchDialog(current_filter=current, parent=self)
        if dlg.exec_() == QDialog.Accepted:
            f = dlg.get_filter_dict()
            # Sync quick fields
            self.filter_player.setText(f.get("player", ""))
            self.filter_white.setText(f.get("white", ""))
            self.filter_black.setText(f.get("black", ""))
            res_idx = self.filter_result.findText(f.get("result", "All"))
            if res_idx >= 0:
                self.filter_result.setCurrentIndex(res_idx)
            self.filter_eco.setText(f.get("eco", ""))
            self.filter_date.setText(f.get("date", ""))
            self.filter_event.setText(f.get("event", ""))
            self.filter_site.setText(f.get("site", ""))
            self.chk_include_deleted.setChecked(f.get("include_deleted", True))
            self.chk_only_deleted.setChecked(f.get("only_deleted", False))
            self.filter_fen.setText(f.get("fen", ""))

            self.current_material_filter = f.get("material")

            if self.table_model.sort_col is not None and self.table_model.sort_col in self.table_model.COLUMN_SORT_FIELDS:
                f["sort_by"] = self.table_model.COLUMN_SORT_FIELDS[self.table_model.sort_col]
                f["sort_asc"] = self.table_model.sort_asc
            self.table_model.set_filters(f)

    def on_search_clicked(self):
        filters = {
            "player": self.filter_player.text().strip(),
            "white": self.filter_white.text().strip(),
            "black": self.filter_black.text().strip(),
            "result": self.filter_result.currentText(),
            "eco": self.filter_eco.text().strip(),
            "date": self.filter_date.text().strip(),
            "event": self.filter_event.text().strip(),
            "site": self.filter_site.text().strip(),
            "include_deleted": self.chk_include_deleted.isChecked(),
            "only_deleted": self.chk_only_deleted.isChecked(),
            "fen": self.filter_fen.text().strip(),
        }
        if self.current_material_filter:
            filters["material"] = self.current_material_filter

        if self.table_model.sort_col is not None and self.table_model.sort_col in self.table_model.COLUMN_SORT_FIELDS:
            filters["sort_by"] = self.table_model.COLUMN_SORT_FIELDS[self.table_model.sort_col]
            filters["sort_asc"] = self.table_model.sort_asc
        self.table_model.set_filters(filters)

    def reset_filters(self):
        self.filter_player.clear()
        self.filter_white.clear()
        self.filter_black.clear()
        self.filter_result.setCurrentIndex(0)
        self.filter_eco.clear()
        self.filter_date.clear()
        self.filter_event.clear()
        self.filter_site.clear()
        self.filter_fen.clear()
        self.chk_include_deleted.setChecked(True)
        self.chk_only_deleted.setChecked(False)
        self.current_material_filter = None
        self.table_model.set_filters({})

    def on_table_selection_changed(self, selected, _deselected):
        indexes = selected.indexes()
        if not indexes:
            return
        row = indexes[0].row()
        game = self.table_model.get_game_at(row)
        if game:
            game_id = game.get("id", row)
            self.selected_game_id = game_id
            self.lbl_selected_game.setText(
                f"Selected Game #{game_id}: {game.get('white')} vs {game.get('black')} ({game.get('result')})"
            )
            if self.client.is_running():
                self.client.send_request("get_pgn", {"index": game_id})

    def add_game_dialog(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Start backend first.")
            return
        dialog = AddEditGameDialog("Add Game to SCID Database", parent=self)
        if dialog.exec_() == QDialog.Accepted:
            pgn = dialog.get_pgn()
            if pgn:
                self.client.send_request("add_game", {"pgn": pgn})

    def edit_game_dialog(self):
        if self.selected_game_id is None or not self.client.is_running():
            QMessageBox.warning(self, "No Selection", "Please select a game to edit.")
            return
        current_pgn = self.pgn_viewer.toPlainText()
        dialog = AddEditGameDialog(f"Edit Game #{self.selected_game_id}", initial_pgn=current_pgn, parent=self)
        if dialog.exec_() == QDialog.Accepted:
            pgn = dialog.get_pgn()
            if pgn:
                self.client.send_request("update_game", {"index": self.selected_game_id, "pgn": pgn})

    def delete_selected_game(self):
        if self.selected_game_id is None or not self.client.is_running():
            return
        self.client.send_request("delete_game", {"index": self.selected_game_id})

    def undelete_selected_game(self):
        if self.selected_game_id is None or not self.client.is_running():
            return
        self.client.send_request("undelete_game", {"index": self.selected_game_id})

    def compact_db(self):
        if not self.client.is_running():
            return
        self.client.send_request("compact")

    def save_db(self):
        if not self.client.is_running():
            return
        self.client.send_request("save")

    def import_pgn(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Start backend and open a database first.")
            return
        path, _ = QFileDialog.getOpenFileName(self, "Select PGN File to Import", "", "PGN Files (*.pgn);;All Files (*)")
        if path:
            file_size_mb = os.path.getsize(path) / (1024 * 1024)
            
            params = {"pgn_path": path}
            scid_exe = self.scid_cpp_input.text().strip()
            if self.chk_use_scid_cpp.isChecked() and scid_exe and os.path.exists(scid_exe):
                params["scid_exe"] = scid_exe
                self.status_bar.showMessage(f"Importing {os.path.basename(path)} with SCID C++ engine (~5s)...")
            else:
                self.status_bar.showMessage(f"Importing {os.path.basename(path)} ({file_size_mb:.1f} MB)...")
            
            # Setup Progress Dialog
            from PyQt5.QtWidgets import QProgressDialog
            self.import_progress_dialog = QProgressDialog(
                f"Importing {os.path.basename(path)}...\nStarting ingest engine...",
                "Cancel",
                0,
                100,
                self,
            )
            self.import_progress_dialog.setWindowTitle("Importing PGN Games")
            self.import_progress_dialog.setWindowModality(Qt.WindowModal)
            self.import_progress_dialog.setMinimumDuration(0)
            self.import_progress_dialog.setValue(0)
            self.import_progress_dialog.show()

            self.client.send_request("import_pgn", params)

    def export_pgn(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Start backend and open a database first.")
            return
        path, _ = QFileDialog.getSaveFileName(self, "Export Database to PGN", "export.pgn", "PGN Files (*.pgn);;All Files (*)")
        if path:
            self.status_bar.showMessage(f"Exporting to {path}...")
            from PyQt5.QtWidgets import QProgressDialog
            self.export_progress_dialog = QProgressDialog(
                f"Exporting {os.path.basename(path)}...\nFormatting PGN streams...",
                "Cancel",
                0,
                100,
                self,
            )
            self.export_progress_dialog.setWindowTitle("Exporting PGN Games")
            self.export_progress_dialog.setWindowModality(Qt.WindowModal)
            self.export_progress_dialog.setMinimumDuration(0)
            self.export_progress_dialog.setValue(0)
            self.export_progress_dialog.show()

            self.client.send_request("export_pgn", {"output_path": path})

    def copy_pgn_text(self):
        text = self.pgn_viewer.toPlainText()
        if text:
            QApplication.clipboard().setText(text)
            self.status_bar.showMessage("PGN copied to clipboard.", 3000)

    def clear_logs(self):
        self.log_viewer.clear()

    def on_model_stats_updated(self, total: int, loaded: int):
        self.lbl_vscroll_info.setText(
            f"Matching Games: {total:,} | Cached in Memory: {loaded:,} | ⚡ Scroll Debouncing Active"
        )

    def on_response_received(self, data: dict):
        # Handle async progress notification events
        if data.get("event") == "import_progress":
            prog = data.get("data", {})
            percent = int(prog.get("percent", 0))
            imported = prog.get("imported_games", 0)
            errors = prog.get("errors", 0)
            proc_mb = prog.get("processed_bytes", 0) / (1024 * 1024)
            tot_mb = prog.get("total_bytes", 0) / (1024 * 1024)
            speed = prog.get("speed_gps", 0.0)
            eta = prog.get("eta_seconds", 0)

            msg = (
                f"Importing PGN Games...\n"
                f"Progress: {percent}% ({proc_mb:.1f} / {tot_mb:.1f} MB)\n"
                f"Games Imported: {imported:,} (Errors: {errors})\n"
                f"Speed: {speed:,.0f} games/sec | ETA: {eta}s"
            )
            if hasattr(self, "import_progress_dialog") and self.import_progress_dialog:
                self.import_progress_dialog.setLabelText(msg)
                self.import_progress_dialog.setValue(percent)

            self.status_bar.showMessage(f"Importing: {percent}% | {imported:,} games ({speed:,.0f} g/s, ETA: {eta}s)")
            return

        if data.get("event") == "export_progress":
            prog = data.get("data", {})
            percent = int(prog.get("percent", 0))
            exported = prog.get("exported_games", 0)
            total = prog.get("total_games", 0)
            speed = prog.get("speed_gps", 0.0)
            eta = prog.get("eta_seconds", 0)

            msg = (
                f"Exporting PGN Games...\n"
                f"Progress: {percent}%\n"
                f"Games Exported: {exported:,} / {total:,}\n"
                f"Speed: {speed:,.0f} games/sec | ETA: {eta}s"
            )
            if hasattr(self, "export_progress_dialog") and self.export_progress_dialog:
                self.export_progress_dialog.setLabelText(msg)
                self.export_progress_dialog.setValue(percent)

            self.status_bar.showMessage(f"Exporting: {percent}% | {exported:,} games ({speed:,.0f} g/s, ETA: {eta}s)")
            return

        if data.get("event") == "search_progress":
            prog = data.get("data", {})
            scanned = prog.get("scanned", 0)
            total = prog.get("total", 0)
            matches = prog.get("matches", 0)
            pct = prog.get("percent", 0.0)
            self.status_bar.showMessage(f"🔍 Searching PGN: {scanned:,} / {total:,} games ({pct:.1f}%) — Found {matches:,} matches...")
            return

        if data.get("event") == "build_pos_index_progress":
            prog = data.get("data", {})
            scanned = prog.get("scanned", 0)
            total = prog.get("total", 0)
            positions = prog.get("positions", 0)
            pct = prog.get("percent", 0.0)
            if hasattr(self, "build_pos_dialog") and self.build_pos_dialog and self.build_pos_dialog.isVisible():
                self.build_pos_dialog.update_progress(scanned, total, positions, pct)
            self.status_bar.showMessage(f"⚡ Indexing Positions: {scanned:,} / {total:,} games ({pct:.1f}%) | Unique: {positions:,}")
            return

        # Log to tab
        self.log_viewer.append(json.dumps(data, indent=2))

        status = data.get("status")
        err = data.get("error")
        if status != "ok" and err:
            if hasattr(self, "import_progress_dialog") and self.import_progress_dialog:
                self.import_progress_dialog.close()
            if hasattr(self, "export_progress_dialog") and self.export_progress_dialog:
                self.export_progress_dialog.close()
            self.status_bar.showMessage(f"Error: {err}", 5000)
            return

        resp_data = data.get("data", {})

        # Handle stats updates
        if "stats" in resp_data:
            stats = resp_data["stats"]
            self.current_db_stats = stats
            fmt = stats.get("format", "").upper()
            self.lbl_format.setText(f"Format: {fmt}")
            self.lbl_games_count.setText(f"Total Games: {stats.get('total_games', 0):,}")
            self.lbl_active_count.setText(f"Active: {stats.get('active_games', 0):,}")
            self.lbl_deleted_count.setText(f"Deleted: {stats.get('deleted_games', 0):,}")
            self.lbl_players_count.setText(f"Players: {stats.get('players_count', 0):,}")
            self.lbl_events_count.setText(f"Events: {stats.get('events_count', 0):,}")

            # Position Index status badge update
            pos_status = stats.get("pos_index_status", resp_data.get("pos_index_status", "missing"))
            pos_count = stats.get("pos_index_unique_positions", resp_data.get("pos_index_unique_positions", 0))
            self.update_pos_index_badge(pos_status, pos_count)

            # Reload model
            self.table_model.set_filters(self.table_model.filters)

        # Handle Position Index Status or Complete
        if "pos_index_status" in resp_data:
            self.update_pos_index_badge(resp_data.get("pos_index_status"), resp_data.get("pos_index_unique_positions", 0))

        if "unique_positions" in resp_data and "elapsed_ms" in resp_data and "moves" not in resp_data:
            unique_pos = resp_data.get("unique_positions", 0)
            elapsed = resp_data.get("elapsed_ms", 0.0)
            self.update_pos_index_badge("valid", unique_pos)
            if hasattr(self, "build_pos_dialog") and self.build_pos_dialog and self.build_pos_dialog.isVisible():
                self.build_pos_dialog.on_complete(unique_pos, elapsed)
            self.status_bar.showMessage(f"⚡ Position Index Built in {elapsed:,.1f} ms ({unique_pos:,} positions).", 5000)
            self.opening_tree_widget.refresh_current_position()

        # Handle Opening Tree Report
        if "moves" in resp_data and "white_pct" in resp_data:
            self.opening_tree_widget.on_tree_report(resp_data)

        # Handle PGN response
        if "pgn" in resp_data:
            self.pgn_viewer.setPlainText(resp_data["pgn"])

        # Handle mutations
        if "reclaimed_bytes" in resp_data:
            reclaimed = resp_data["reclaimed_bytes"]
            self.status_bar.showMessage(f"Compaction completed. Reclaimed {reclaimed} bytes.", 4000)
            self.refresh_database_info()

        if "imported" in resp_data:
            if hasattr(self, "import_progress_dialog") and self.import_progress_dialog:
                self.import_progress_dialog.setValue(100)
                self.import_progress_dialog.close()

            imp = resp_data["imported"]
            err_count = resp_data.get("errors", 0)
            QMessageBox.information(
                self, "Import Complete", f"Imported {imp:,} games successfully ({err_count} errors)."
            )
            self.refresh_database_info()

        if "exported" in resp_data:
            if hasattr(self, "export_progress_dialog") and self.export_progress_dialog:
                self.export_progress_dialog.setValue(100)
                self.export_progress_dialog.close()
            exp = resp_data["exported"]
            QMessageBox.information(self, "Export Complete", f"Exported {exp:,} games to PGN successfully.")

        if "deleted" in resp_data or "index" in resp_data and "pgn" not in resp_data:
            self.refresh_database_info()

        # Handle benchmark report
        if "results" in resp_data and "total_time_ms" in resp_data:
            if hasattr(self, "benchmark_dialog") and self.benchmark_dialog:
                self.benchmark_dialog.display_report(resp_data)

    def on_tab_changed(self, index: int):
        if "Opening Tree" in self.tabs.tabText(index):
            self.opening_tree_widget.refresh_current_position()

    def prompt_build_pos_index(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Please start backend and open a database first.")
            return
        self.build_pos_dialog = BuildPosIndexDialog(self.client, parent=self)
        self.build_pos_dialog.show()

    def update_pos_index_badge(self, status: str, count: int = 0):
        if status == "valid":
            self.btn_pos_index.setText(f"🟢 Fast Index: Active ({count:,})")
            self.btn_pos_index.setStyleSheet("font-weight: bold; font-size: 11px; padding: 2px 8px; background-color: #e8f5e9; color: #2e7d32; border: 1px solid #81c784; border-radius: 3px;")
            self.opening_tree_widget.lbl_index_badge.setText(f"🟢 Fast Index: Active ({count:,} pos)")
            self.opening_tree_widget.lbl_index_badge.setStyleSheet("color: #2e7d32; font-weight: bold; font-size: 11px;")
        elif status == "outdated":
            self.btn_pos_index.setText("🟠 Fast Index: Outdated [Rebuild]")
            self.btn_pos_index.setStyleSheet("font-weight: bold; font-size: 11px; padding: 2px 8px; background-color: #fff3e0; color: #e65100; border: 1px solid #ffb74d; border-radius: 3px;")
            self.opening_tree_widget.lbl_index_badge.setText("🟠 Fast Index: Outdated (Rebuild Recommended)")
            self.opening_tree_widget.lbl_index_badge.setStyleSheet("color: #e65100; font-weight: bold; font-size: 11px;")
        else:
            self.btn_pos_index.setText("⚡ Build Fast Index")
            self.btn_pos_index.setStyleSheet("font-weight: bold; font-size: 11px; padding: 2px 8px; border-radius: 3px;")
            self.opening_tree_widget.lbl_index_badge.setText("⚪ Fast Index: Not Built")
            self.opening_tree_widget.lbl_index_badge.setStyleSheet("color: #757575; font-weight: bold; font-size: 11px;")

    def open_benchmark_dialog(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Please start backend and open a database first.")
            return
        self.benchmark_dialog = BenchmarkDialog(self.client, current_stats=self.current_db_stats, parent=self)
        self.benchmark_dialog.show()

    def on_process_error(self, err_msg: str):
        self.log_viewer.append(f"[ERROR] {err_msg}")
        self.status_bar.showMessage(err_msg, 5000)

    def on_process_stopped(self):
        self.update_ui_disconnected()
        self.log_viewer.append("[GUI] Backend process stopped.")

    def closeEvent(self, event):
        self.client.stop()
        event.accept()


