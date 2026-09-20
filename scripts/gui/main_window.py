import os
import sys
from typing import Optional

from PyQt5.QtCore import Qt
from PyQt5.QtWidgets import (
    QMainWindow, QWidget, QVBoxLayout, QSplitter,
    QTabWidget, QStatusBar, QMessageBox, QProgressDialog, QDialog
)

from .backend_client import BackendClient
from .models import VirtualScidTableModel
from .widgets import (
    DatabaseControlWidget, FilterPanelWidget, GameTablePanelWidget,
    GamePreviewPanelWidget, OpeningTreeWidget, CqlSearchWidget, ProtocolLogPanelWidget
)
from .dialogs.add_edit_game_dialog import AddEditGameDialog
from .dialogs.search_progress_dialog import SearchProgressDialog

class MainWindow(QMainWindow):
    """
    Main Application Window for SCID Chess Database Manager.
    Acts as the top-level coordinator wiring backend events to modular UI components.
    """
    def __init__(self):
        super().__init__()
        self.setWindowTitle("SCID Chess Database Manager (chess-scid-rw)")
        self.setMinimumSize(850, 500)
        self.resize(1280, 780)

        # Backend Client & Table Model
        self.client = BackendClient(self)
        self.client.response_received.connect(self.on_response_received)
        self.client.process_error.connect(self.on_process_error)
        self.client.process_stopped.connect(self.on_process_stopped)

        self.table_model = VirtualScidTableModel(self.client, self)

        # State & Dialogs
        self.selected_game_id: Optional[int] = None
        self.import_progress_dialog: Optional[QProgressDialog] = None
        self.export_progress_dialog: Optional[QProgressDialog] = None
        self.search_progress_dialog: Optional[SearchProgressDialog] = None

        self.init_ui()

    def init_ui(self):
        main_widget = QWidget()
        self.setCentralWidget(main_widget)
        root_layout = QVBoxLayout(main_widget)
        root_layout.setContentsMargins(6, 6, 6, 6)
        root_layout.setSpacing(4)

        # 1. Database & Connection Control Bar
        self.db_panel = DatabaseControlWidget(self.client, self)
        self.db_panel.open_database_requested.connect(self.open_database)
        self.db_panel.start_backend_requested.connect(self.start_backend)
        self.db_panel.stop_backend_requested.connect(self.stop_backend)
        self.db_panel.create_database_requested.connect(self.create_database)
        self.db_panel.import_pgn_requested.connect(self.import_pgn)
        self.db_panel.export_pgn_requested.connect(self.export_pgn)
        self.db_panel.compact_requested.connect(self.compact_db)
        self.db_panel.save_requested.connect(self.save_db)
        self.db_panel.refresh_info_requested.connect(self.refresh_database_info)
        root_layout.addWidget(self.db_panel)

        # 2. Search & Filter Bar
        self.filter_panel = FilterPanelWidget(self)
        self.filter_panel.search_applied.connect(self.on_search_applied)
        self.filter_panel.filters_cleared.connect(self.on_filters_cleared)
        root_layout.addWidget(self.filter_panel)

        # 3. Main Splitter: Games Table on Left, Details & Views on Right
        splitter = QSplitter(Qt.Horizontal)

        # Left Container: Virtual Games Table
        self.table_panel = GameTablePanelWidget(self.table_model, self)
        self.table_panel.game_selected.connect(self.on_game_selected)
        splitter.addWidget(self.table_panel)

        # Right Container: Tabs
        self.tabs = QTabWidget()

        # Tab 1: PGN Game Viewer
        self.preview_panel = GamePreviewPanelWidget(self)
        self.preview_panel.add_game_requested.connect(self.add_game_dialog)
        self.preview_panel.edit_game_requested.connect(self.edit_game_dialog)
        self.preview_panel.delete_game_requested.connect(self.delete_selected_game)
        self.preview_panel.undelete_game_requested.connect(self.undelete_selected_game)
        self.tabs.addTab(self.preview_panel, "PGN Game Text")

        # Tab 2: Opening Tree Explorer
        self.opening_tree_widget = OpeningTreeWidget(self.client, self)
        self.tabs.addTab(self.opening_tree_widget, "🌲 Opening Tree")
        self.tabs.currentChanged.connect(self.on_tab_changed)

        # Tab 3: Dedicated CQL / Search Engine Query Panel
        self.cql_search_widget = CqlSearchWidget(self.client, self)
        self.tabs.addTab(self.cql_search_widget, "🔎 CQL Search")

        # Tab 4: Protocol Logs
        self.log_panel = ProtocolLogPanelWidget(self)
        self.tabs.addTab(self.log_panel, "Protocol Logs")

        splitter.addWidget(self.tabs)
        splitter.setSizes([800, 480])
        root_layout.addWidget(splitter, 1)

        # 4. Status Bar
        self.status_bar = QStatusBar()
        self.setStatusBar(self.status_bar)
        self.status_bar.showMessage("Ready. Select SCID database and start backend.")

    # ------------------------------------------------------------------------
    # Backend Lifecycle & Database Actions
    # ------------------------------------------------------------------------

    def start_backend(self, bin_path: str, db_path: str, threads: int):
        try:
            self.client.start(
                bin_path,
                db_path if db_path and os.path.exists(db_path) else None,
                threads=threads if threads > 0 else None,
            )
            self.db_panel.update_ui_connected()
            self.log_panel.append_message(f"[GUI] Spawned backend (Threads: {threads or 'auto'}): {bin_path}")
            if db_path:
                self.client.send_request("open", {"path": db_path})
            else:
                self.refresh_database_info()
        except Exception as e:
            QMessageBox.critical(self, "Startup Error", str(e))

    def stop_backend(self):
        self.client.stop()
        self.update_ui_disconnected()

    def open_database(self, path: str):
        if self.client.is_running():
            self.client.send_request("open", {"path": path})

    def create_database(self, db_path: str, fmt: str):
        if self.client.is_running():
            self.client.send_request("create", {"path": db_path, "format": fmt})
        else:
            bin_path = self.db_panel.binary_input.text().strip()
            if bin_path and os.path.exists(bin_path):
                self.start_backend(bin_path, db_path, 0)
                self.client.send_request("create", {"path": db_path, "format": fmt})

    def refresh_database_info(self):
        if self.client.is_running():
            self.client.send_request("info")

    def compact_db(self):
        if self.client.is_running():
            self.client.send_request("compact")

    def save_db(self):
        if self.client.is_running():
            self.client.send_request("save")

    def import_pgn(self, pgn_path: str, scid_exe: Optional[str] = None):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Start backend and open a database first.")
            return

        file_size_mb = os.path.getsize(pgn_path) / (1024 * 1024)
        params = {"pgn_path": pgn_path}
        if scid_exe:
            params["scid_exe"] = scid_exe
            self.status_bar.showMessage(f"Importing {os.path.basename(pgn_path)} with SCID C++ engine (~5s)...")
        else:
            self.status_bar.showMessage(f"Importing {os.path.basename(pgn_path)} ({file_size_mb:.1f} MB)...")

        self.import_progress_dialog = QProgressDialog(
            f"Importing {os.path.basename(pgn_path)}...\nStarting ingest engine...",
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

    def export_pgn(self, output_path: str):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Start backend and open a database first.")
            return

        self.status_bar.showMessage(f"Exporting to {output_path}...")
        self.export_progress_dialog = QProgressDialog(
            f"Exporting {os.path.basename(output_path)}...\nFormatting PGN streams...",
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

        self.client.send_request("export_pgn", {"output_path": output_path})

    def prompt_build_pos_index(self):
        self.db_panel.prompt_build_pos_index()

    # ------------------------------------------------------------------------
    # Search & Filtering
    # ------------------------------------------------------------------------

    def on_search_applied(self, filters: dict):
        f = dict(filters)
        if self.table_model.sort_col is not None and self.table_model.sort_col in self.table_model.COLUMN_SORT_FIELDS:
            f["sort_by"] = self.table_model.COLUMN_SORT_FIELDS[self.table_model.sort_col]
            f["sort_asc"] = self.table_model.sort_asc
        self.table_model.set_filters(f)

    def on_filters_cleared(self):
        self.table_model.set_filters({})

    # ------------------------------------------------------------------------
    # Game Selection & Details
    # ------------------------------------------------------------------------

    def on_game_selected(self, game_id: int, game_data: dict):
        self.selected_game_id = game_id
        title = f"Selected Game #{game_id}: {game_data.get('white')} vs {game_data.get('black')} ({game_data.get('result')})"
        self.preview_panel.set_selected_game(game_id, title)
        if self.client.is_running():
            self.client.send_request("get_pgn", {"index": game_id})

    def load_game_by_id(self, game_id: int):
        self.selected_game_id = game_id
        self.preview_panel.set_selected_game(game_id)
        if self.client.is_running():
            self.client.send_request("get_pgn", {"index": game_id})
        self.tabs.setCurrentIndex(0)

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
        current_pgn = self.preview_panel.get_pgn_text()
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

    def on_tab_changed(self, index: int):
        if "Opening Tree" in self.tabs.tabText(index):
            self.opening_tree_widget.refresh_current_position()

    def update_ui_disconnected(self):
        self.db_panel.update_ui_disconnected()
        self.table_model.clear()
        self.preview_panel.clear()

    # ------------------------------------------------------------------------
    # Backend Response Dispatcher
    # ------------------------------------------------------------------------

    def on_response_received(self, data: dict):
        # 1. Async Progress Notifications
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
            if self.import_progress_dialog:
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
            if self.export_progress_dialog:
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

            if not self.search_progress_dialog:
                self.search_progress_dialog = SearchProgressDialog("Searching Database Games...", self)

            if not self.search_progress_dialog.isVisible() and pct < 98.0:
                self.search_progress_dialog.show()

            if self.search_progress_dialog.isVisible():
                self.search_progress_dialog.update_progress(scanned, total, matches, pct)

            self.status_bar.showMessage(f"🔍 Searching: {scanned:,} / {total:,} games ({pct:.1f}%) — Found {matches:,} matches...")
            return

        if data.get("event") in ("build_pos_index_progress", "build_tree_progress"):
            event = data.get("event")
            prog = data.get("data", {})
            scanned = prog.get("scanned", 0)
            total = prog.get("total", 0)
            positions = prog.get("positions", 0)
            pct = prog.get("percent", 0.0)
            task_name = "Tree Index (.tree.idx)" if event == "build_tree_progress" else "Position Booster (.pos.idx)"
            if hasattr(self.db_panel, "build_pos_dialog") and self.db_panel.build_pos_dialog and self.db_panel.build_pos_dialog.isVisible():
                self.db_panel.build_pos_dialog.update_progress(scanned, total, positions, pct, task_name)
            self.status_bar.showMessage(f"⚡ Indexing [{task_name}]: {scanned:,} / {total:,} games ({pct:.1f}%) | Unique: {positions:,}")
            return

        # 2. Append sanitized protocol log
        self.log_panel.append_json_payload(data)

        status = data.get("status")
        err = data.get("error")
        if status != "ok" and err:
            if self.import_progress_dialog:
                self.import_progress_dialog.close()
            if self.export_progress_dialog:
                self.export_progress_dialog.close()
            if self.search_progress_dialog:
                self.search_progress_dialog.close()
            self.status_bar.showMessage(f"Error: {err}", 5000)
            return

        resp_data = data.get("data", {})

        # 3. Handle games or dsl_search query response
        if "games" in resp_data or "matches" in resp_data:
            total_matches = resp_data.get("total", resp_data.get("matched_count", 0))
            if self.search_progress_dialog and self.search_progress_dialog.isVisible():
                self.search_progress_dialog.on_finished(total_matches)
            self.status_bar.showMessage(f"Search complete: {total_matches:,} matching games found.", 5000)

        # 4. Handle stats updates
        if "stats" in resp_data:
            stats = resp_data["stats"]
            self.db_panel.update_stats(stats)
            pos_status = stats.get("pos_index_status", resp_data.get("pos_index_status", "missing"))
            pos_count = stats.get("pos_index_unique_positions", resp_data.get("pos_index_unique_positions", 0))
            tree_status = stats.get("tree_index_status", resp_data.get("tree_index_status", "missing"))
            tree_count = stats.get("tree_index_unique_positions", resp_data.get("tree_index_unique_positions", 0))
            self.update_indexes_badge(pos_status, pos_count, tree_status, tree_count)
            self.table_model.set_filters(self.table_model.filters)

        # 5. Handle Position / Tree Index Status
        if "pos_index_status" in resp_data or "tree_index_status" in resp_data:
            p_st = resp_data.get("pos_index_status", self.db_panel.pos_index_status)
            p_cnt = resp_data.get("pos_index_unique_positions", self.db_panel.pos_index_unique_positions)
            t_st = resp_data.get("tree_index_status", self.db_panel.tree_index_status)
            t_cnt = resp_data.get("tree_index_unique_positions", self.db_panel.tree_index_unique_positions)
            self.update_indexes_badge(p_st, p_cnt, t_st, t_cnt)

        if "unique_positions" in resp_data and "elapsed_ms" in resp_data and "moves" not in resp_data:
            self.refresh_database_info()
            if "Opening Tree" in self.tabs.tabText(self.tabs.currentIndex()):
                self.opening_tree_widget.refresh_current_position()

        # 6. Handle Opening Tree Report
        if "moves" in resp_data and "white_pct" in resp_data:
            self.opening_tree_widget.on_tree_report(resp_data)

        # 7. Handle Game Summaries response
        if "game_summaries" in resp_data:
            self.opening_tree_widget.on_game_summaries_received(resp_data["game_summaries"])

        # 8. Handle PGN response
        if "pgn" in resp_data:
            self.preview_panel.set_pgn_text(resp_data["pgn"])

        # 9. Handle mutations
        if "reclaimed_bytes" in resp_data:
            reclaimed = resp_data["reclaimed_bytes"]
            self.status_bar.showMessage(f"Compaction completed. Reclaimed {reclaimed} bytes.", 4000)
            self.refresh_database_info()

        if "imported" in resp_data:
            if self.import_progress_dialog:
                self.import_progress_dialog.setValue(100)
                self.import_progress_dialog.close()
            imp = resp_data["imported"]
            err_count = resp_data.get("errors", 0)
            QMessageBox.information(
                self, "Import Complete", f"Imported {imp:,} games successfully ({err_count} errors)."
            )
            self.refresh_database_info()

        if "exported" in resp_data:
            if self.export_progress_dialog:
                self.export_progress_dialog.setValue(100)
                self.export_progress_dialog.close()
            exp = resp_data["exported"]
            QMessageBox.information(self, "Export Complete", f"Exported {exp:,} games to PGN successfully.")

        if "deleted" in resp_data or ("index" in resp_data and "pgn" not in resp_data):
            self.refresh_database_info()

    def update_indexes_badge(self, pos_status: str, pos_count: int = 0, tree_status: str = "missing", tree_count: int = 0):
        self.opening_tree_widget.update_tree_index_badge(tree_status, tree_count)
        self.db_panel.update_indexes_badge(pos_status, pos_count, tree_status, tree_count)

    def on_process_error(self, err_msg: str):
        self.log_panel.append_message(f"[ERROR] {err_msg}")
        self.status_bar.showMessage(err_msg, 5000)

    def on_process_stopped(self):
        self.update_ui_disconnected()
        self.log_panel.append_message("[GUI] Backend process stopped.")

    def closeEvent(self, event):
        self.client.stop()
        event.accept()
