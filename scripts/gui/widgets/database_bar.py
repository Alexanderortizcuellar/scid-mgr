import os
import sys
from typing import Optional

from PyQt5.QtCore import Qt, pyqtSignal, QSettings
from PyQt5.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QGridLayout, QLabel,
    QLineEdit, QPushButton, QGroupBox, QFrame, QCheckBox, QFileDialog, QMessageBox
)

from ..dialogs.new_db_dialog import NewDatabaseDialog
from ..dialogs.settings_dialog import SettingsDialog
from ..dialogs.pos_idx_diagnostics_dialog import PosIdxDiagnosticsDialog
from ..dialogs.build_pos_index_dialog import BuildPosIndexDialog
from ..dialogs.benchmark_dialog import BenchmarkDialog

class DatabaseControlWidget(QWidget):
    """
    Manages backend binary connection, database file selection,
    high-level database operations (new, import, export, compact, save),
    and database summary statistics / fast index status badge.
    """
    open_database_requested = pyqtSignal(str)
    start_backend_requested = pyqtSignal(str, str, int)  # bin_path, db_path, threads
    stop_backend_requested = pyqtSignal()
    create_database_requested = pyqtSignal(str, str)     # db_path, format
    import_pgn_requested = pyqtSignal(str, object)      # pgn_path, scid_exe (Optional[str])
    export_pgn_requested = pyqtSignal(str)               # output_path
    compact_requested = pyqtSignal()
    save_requested = pyqtSignal()
    refresh_info_requested = pyqtSignal()

    def __init__(self, client, parent=None):
        super().__init__(parent)
        self.client = client
        self.current_db_stats: Optional[dict] = None
        self.pos_index_status = "missing"
        self.pos_index_unique_positions = 0
        self.tree_index_status = "missing"
        self.tree_index_unique_positions = 0

        self.init_ui()
        self.auto_detect_defaults()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(6)

        # 1. Connection & Database Management Group
        conn_group = QGroupBox("Backend Connection & SCID Database")
        conn_layout = QGridLayout(conn_group)
        conn_layout.setContentsMargins(8, 4, 8, 4)
        conn_layout.setSpacing(4)

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
        scid_cpp_row.setContentsMargins(0, 0, 0, 0)
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
        db_actions_layout.setContentsMargins(0, 0, 0, 0)
        db_actions_layout.setSpacing(4)
        self.btn_connect = QPushButton("Start Backend")
        self.btn_connect.setStyleSheet("font-weight: bold; background-color: #2e7d32; color: white; padding: 4px 10px;")
        self.btn_connect.clicked.connect(self.toggle_backend)
        db_actions_layout.addWidget(self.btn_connect)

        self.btn_new_db = QPushButton("New DB...")
        self.btn_new_db.clicked.connect(self.create_new_db)
        db_actions_layout.addWidget(self.btn_new_db)

        self.btn_import_pgn = QPushButton("Import PGN...")
        self.btn_import_pgn.clicked.connect(self.on_import_pgn)
        db_actions_layout.addWidget(self.btn_import_pgn)

        self.btn_export_pgn = QPushButton("Export PGN...")
        self.btn_export_pgn.clicked.connect(self.on_export_pgn)
        db_actions_layout.addWidget(self.btn_export_pgn)

        self.btn_compact = QPushButton("Compact DB")
        self.btn_compact.clicked.connect(lambda: self.compact_requested.emit())
        db_actions_layout.addWidget(self.btn_compact)

        self.btn_save = QPushButton("Save DB")
        self.btn_save.setStyleSheet("font-weight: bold; background-color: #0288d1; color: white; padding: 4px 10px;")
        self.btn_save.clicked.connect(lambda: self.save_requested.emit())
        db_actions_layout.addWidget(self.btn_save)

        self.btn_benchmark = QPushButton("📊 Metrics...")
        self.btn_benchmark.setStyleSheet("font-weight: bold; padding: 4px 8px;")
        self.btn_benchmark.clicked.connect(self.open_benchmark_dialog)
        db_actions_layout.addWidget(self.btn_benchmark)

        self.btn_pos_idx_diag = QPushButton("🔬 Pos.idx...")
        self.btn_pos_idx_diag.setStyleSheet("font-weight: bold; padding: 4px 8px;")
        self.btn_pos_idx_diag.clicked.connect(self.open_pos_idx_diagnostics_dialog)
        db_actions_layout.addWidget(self.btn_pos_idx_diag)

        self.btn_settings = QPushButton("⚙️ Settings...")
        self.btn_settings.setStyleSheet("font-weight: bold; padding: 4px 8px;")
        self.btn_settings.clicked.connect(self.open_settings_dialog)
        db_actions_layout.addWidget(self.btn_settings)

        conn_layout.addLayout(db_actions_layout, 3, 0, 1, 3)
        layout.addWidget(conn_group)

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
        self.btn_pos_index.setToolTip("Companion Indexes (.tree.idx & .pos.idx) for sub-millisecond tree & position search")
        self.btn_pos_index.clicked.connect(self.prompt_build_pos_index)
        stats_layout.addWidget(self.btn_pos_index)

        stats_layout.addStretch()
        btn_refresh_info = QPushButton("Refresh Info")
        btn_refresh_info.clicked.connect(lambda: self.refresh_info_requested.emit())
        stats_layout.addWidget(btn_refresh_info)

        layout.addWidget(self.stats_bar)

    def auto_detect_defaults(self):
        project_root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
        release_bin = os.path.join(project_root, "target", "release", "scid-mgr.exe" if sys.platform == "win32" else "scid-mgr")
        
        self.binary_input.setText(os.path.abspath(release_bin))

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
            settings = QSettings("ChessScidMgr", "ScidGui")
            settings.setValue("binary_path", path)

    def browse_db(self):
        path, _ = QFileDialog.getOpenFileName(
            self,
            "Select Chess Database or PGN File",
            "",
            "Chess Databases (*.si5 *.si4 *.pgn);;SCID Databases (*.si5 *.si4 *.sn5 *.sn4 *.sg5 *.sg4);;PGN Files (*.pgn);;All Files (*)",
        )
        if path:
            self.db_input.setText(path)
            self.open_database_requested.emit(path)

    def create_new_db(self):
        dialog = NewDatabaseDialog(self)
        if dialog.exec_() == NewDatabaseDialog.Accepted:
            db_path, fmt = dialog.get_data()
            if not db_path:
                return
            self.db_input.setText(db_path)
            self.create_database_requested.emit(db_path, fmt)

    def toggle_backend(self):
        if self.client.is_running():
            self.stop_backend_requested.emit()
        else:
            bin_path = self.binary_input.text().strip()
            db_path = self.db_input.text().strip()
            if not bin_path or not os.path.exists(bin_path):
                QMessageBox.warning(
                    self,
                    "Release Binary Missing",
                    f"Cannot find release binary at:\n{bin_path}\n\nPlease compile it using: cargo build --release",
                )
                return

            settings = QSettings("chess-scid-rw", "ScidDatabaseManager")
            threads = int(settings.value("worker_threads", 0))
            self.start_backend_requested.emit(bin_path, db_path, threads)

    def on_import_pgn(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Start backend and open a database first.")
            return
        path, _ = QFileDialog.getOpenFileName(self, "Select PGN File to Import", "", "PGN Files (*.pgn);;All Files (*)")
        if path:
            scid_exe = self.scid_cpp_input.text().strip()
            use_scid = self.chk_use_scid_cpp.isChecked() and scid_exe and os.path.exists(scid_exe)
            self.import_pgn_requested.emit(path, scid_exe if use_scid else None)

    def on_export_pgn(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Start backend and open a database first.")
            return
        path, _ = QFileDialog.getSaveFileName(self, "Export Database to PGN", "export.pgn", "PGN Files (*.pgn);;All Files (*)")
        if path:
            self.export_pgn_requested.emit(path)

    def open_settings_dialog(self):
        dialog = SettingsDialog(self.client, self)
        dialog.exec_()

    def open_pos_idx_diagnostics_dialog(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Offline", "Backend is not running. Please open a database first.")
            return
        dlg = PosIdxDiagnosticsDialog(self.client, self)
        dlg.exec_()

    def open_benchmark_dialog(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Please start backend and open a database first.")
            return
        dlg = BenchmarkDialog(self.client, current_stats=self.current_db_stats, parent=self)
        dlg.show()

    def prompt_build_pos_index(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Please start backend and open a database first.")
            return
        dlg = BuildPosIndexDialog(self.client, parent=self)
        dlg.show()

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
        self.lbl_format.setText("Format: -")
        self.lbl_games_count.setText("Total Games: -")
        self.lbl_active_count.setText("Active: -")
        self.lbl_deleted_count.setText("Deleted: -")
        self.lbl_players_count.setText("Players: -")
        self.lbl_events_count.setText("Events: -")
        self.btn_pos_index.setText("⚡ Build Fast Index")
        self.btn_pos_index.setStyleSheet("font-weight: bold; font-size: 11px; padding: 2px 8px; border-radius: 3px;")

    def update_stats(self, stats: dict):
        self.current_db_stats = stats
        fmt = stats.get("format", "").upper()
        self.lbl_format.setText(f"Format: {fmt}")
        self.lbl_games_count.setText(f"Total Games: {stats.get('total_games', 0):,}")
        self.lbl_active_count.setText(f"Active: {stats.get('active_games', 0):,}")
        self.lbl_deleted_count.setText(f"Deleted: {stats.get('deleted_games', 0):,}")
        self.lbl_players_count.setText(f"Players: {stats.get('players_count', 0):,}")
        self.lbl_events_count.setText(f"Events: {stats.get('events_count', 0):,}")

    def update_indexes_badge(self, pos_status: str, pos_count: int = 0, tree_status: str = "missing", tree_count: int = 0):
        self.pos_index_status = pos_status
        self.pos_index_unique_positions = pos_count
        self.tree_index_status = tree_status
        self.tree_index_unique_positions = tree_count

        if pos_status == "valid" and tree_status == "valid":
            self.btn_pos_index.setText(f"🟢 Fast Indexes: Active (Tree: {tree_count:,} | Pos: {pos_count:,})")
            self.btn_pos_index.setStyleSheet("font-weight: bold; font-size: 11px; padding: 2px 8px; background-color: #e8f5e9; color: #2e7d32; border: 1px solid #81c784; border-radius: 3px;")
        elif tree_status == "valid":
            self.btn_pos_index.setText(f"🟢 Tree Idx: Active ({tree_count:,}) | ⚪ Pos Idx")
            self.btn_pos_index.setStyleSheet("font-weight: bold; font-size: 11px; padding: 2px 8px; background-color: #e8f5e9; color: #2e7d32; border: 1px solid #81c784; border-radius: 3px;")
        elif pos_status == "valid":
            self.btn_pos_index.setText(f"🟢 Pos Idx: Active ({pos_count:,}) | ⚪ Tree Idx")
            self.btn_pos_index.setStyleSheet("font-weight: bold; font-size: 11px; padding: 2px 8px; background-color: #e8f5e9; color: #2e7d32; border: 1px solid #81c784; border-radius: 3px;")
        elif pos_status == "outdated" or tree_status == "outdated":
            self.btn_pos_index.setText("🟠 Fast Indexes: Outdated [Rebuild]")
            self.btn_pos_index.setStyleSheet("font-weight: bold; font-size: 11px; padding: 2px 8px; background-color: #fff3e0; color: #e65100; border: 1px solid #ffb74d; border-radius: 3px;")
        else:
            self.btn_pos_index.setText("⚡ Build Fast Indexes")
            self.btn_pos_index.setStyleSheet("font-weight: bold; font-size: 11px; padding: 2px 8px; border-radius: 3px;")
