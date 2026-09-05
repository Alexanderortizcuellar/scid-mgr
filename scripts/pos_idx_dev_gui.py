#!/usr/bin/env python3
"""
Position Index (.pos.idx) Development & Testing Workbench
Features:
- Live Opening Tree & Move GameSet Inspector
- Real-time Roaring Bitmap vs Delta-Varint Adaptive Diagnostics
- Space Savings & Distribution Visualizer
- Filter Simulation Lab (Set Intersections in RAM)
- Rebuilding Lab with Configurable Max Games per Move
"""

import os
import sys
import json
import time

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

from PyQt5.QtCore import Qt, QTimer
from PyQt5.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QLabel, QPushButton, QLineEdit, QFileDialog, QTableWidget,
    QTableWidgetItem, QHeaderView, QGroupBox, QFormLayout, QSpinBox,
    QProgressBar, QTextEdit, QSplitter, QTabWidget, QMessageBox, QFrame
)
from gui.backend_client import BackendClient
from gui.dialogs.build_pos_index_dialog import BuildPosIndexDialog

class PosIdxDevWorkbench(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("⚡ Position Index (.pos.idx) Dev & Test Workbench")
        self.resize(1100, 780)

        self.client = BackendClient(self)
        self.client.response_received.connect(self.on_backend_response)
        self.client.process_error.connect(self.on_backend_error)

        self.current_fen = ""
        self.move_history = []

        self.init_ui()
        self.start_backend()

    def init_ui(self):
        central = QWidget()
        self.setCentralWidget(central)
        main_layout = QVBoxLayout(central)
        main_layout.setSpacing(8)

        # Top Bar: Database Selector & Backend Status
        top_box = QGroupBox("Database & Companion Index Connection")
        top_layout = QHBoxLayout(top_box)

        self.db_input = QLineEdit()
        self.db_input.setPlaceholderText("Path to .si5, .si4, or .pgn database...")
        default_db = r"C:\Users\ASUS\programming\qt_programs\chess\twchess\data\database.si5"
        if os.path.exists(default_db):
            self.db_input.setText(default_db)
        top_layout.addWidget(self.db_input, 4)

        btn_browse = QPushButton("📁 Browse DB...")
        btn_browse.clicked.connect(self.browse_db)
        top_layout.addWidget(btn_browse, 1)

        self.btn_open = QPushButton("⚡ Load / Connect")
        self.btn_open.setStyleSheet("font-weight: bold; background-color: #0288d1; color: white;")
        self.btn_open.clicked.connect(self.open_database)
        top_layout.addWidget(self.btn_open, 1)

        self.btn_build_idx = QPushButton("🔨 Build .pos.idx...")
        self.btn_build_idx.setStyleSheet("font-weight: bold; background-color: #2e7d32; color: white;")
        self.btn_build_idx.clicked.connect(self.open_build_dialog)
        top_layout.addWidget(self.btn_build_idx, 1)

        main_layout.addWidget(top_box)

        # Tabs: Explorer, Diagnostics, Filter Lab
        self.tabs = QTabWidget()

        # Tab 1: Opening Tree & GameSet Move Inspector
        tab_tree = QWidget()
        tree_layout = QVBoxLayout(tab_tree)

        nav_bar = QHBoxLayout()
        self.lbl_fen = QLabel("Position: Starting Board")
        self.lbl_fen.setStyleSheet("font-family: monospace; font-weight: bold;")
        nav_bar.addWidget(self.lbl_fen, 4)

        btn_reset = QPushButton("⏮ Reset Starting Pos")
        btn_reset.clicked.connect(self.reset_board)
        nav_bar.addWidget(btn_reset, 1)
        tree_layout.addLayout(nav_bar)

        self.tree_table = QTableWidget(0, 7)
        self.tree_table.setHorizontalHeaderLabels([
            "Move (SAN)", "UCI", "Games", "White Win %", "Draw %", "Black Win %", "Sample Game IDs"
        ])
        self.tree_table.horizontalHeader().setSectionResizeMode(QHeaderView.Stretch)
        self.tree_table.setSelectionBehavior(QTableWidget.SelectRows)
        self.tree_table.setEditTriggers(QTableWidget.NoEditTriggers)
        self.tree_table.cellDoubleClicked.connect(self.on_move_double_clicked)
        tree_layout.addWidget(self.tree_table)

        self.lbl_query_stats = QLabel("Query latency: — ms | Moves available: 0")
        self.lbl_query_stats.setStyleSheet("color: #555; font-size: 11px;")
        tree_layout.addWidget(self.lbl_query_stats)

        self.tabs.addTab(tab_tree, "🌳 Opening Tree & Move Inspector")

        # Tab 2: Adaptive Roaring vs Delta Diagnostics
        tab_diag = QWidget()
        diag_layout = QVBoxLayout(tab_diag)

        diag_top = QHBoxLayout()
        self.lbl_diag_status = QLabel("Diagnostics: Click 'Scan Index' to analyze adaptive GameSet breakdown.")
        self.lbl_diag_status.setStyleSheet("font-weight: bold;")
        diag_top.addWidget(self.lbl_diag_status, 4)

        btn_scan = QPushButton("🔍 Scan .pos.idx Diagnostics")
        btn_scan.setStyleSheet("font-weight: bold; padding: 6px 12px;")
        btn_scan.clicked.connect(self.scan_diagnostics)
        diag_top.addWidget(btn_scan, 1)
        diag_layout.addLayout(diag_top)

        metrics_group = QGroupBox("Adaptive GameSet Encoding & Space Savings Metrics")
        form = QFormLayout(metrics_group)
        self.lbl_diag_total_sets = QLabel("—")
        form.addRow("Total Move GameSets:", self.lbl_diag_total_sets)

        self.lbl_diag_delta = QLabel("—")
        form.addRow("Delta-Varint Chosen:", self.lbl_diag_delta)

        self.lbl_diag_roaring = QLabel("—")
        form.addRow("Roaring Bitmap Chosen:", self.lbl_diag_roaring)

        self.lbl_diag_size_delta = QLabel("—")
        form.addRow("Total Bytes if 100% Delta-Varint:", self.lbl_diag_size_delta)

        self.lbl_diag_size_roaring = QLabel("—")
        form.addRow("Total Bytes if 100% Roaring:", self.lbl_diag_size_roaring)

        self.lbl_diag_size_adaptive = QLabel("—")
        self.lbl_diag_size_adaptive.setStyleSheet("font-weight: bold; color: #2e7d32;")
        form.addRow("Actual Adaptive File Payload:", self.lbl_diag_size_adaptive)

        self.lbl_diag_savings = QLabel("—")
        self.lbl_diag_savings.setStyleSheet("font-weight: bold; color: #1565c0;")
        form.addRow("Net Space Saved:", self.lbl_diag_savings)
        diag_layout.addWidget(metrics_group)

        dist_group = QGroupBox("GameSet ID Size Distribution")
        dist_layout = QVBoxLayout(dist_group)
        self.table_dist = QTableWidget(6, 2)
        self.table_dist.setHorizontalHeaderLabels(["ID Count Range", "GameSet Count"])
        self.table_dist.horizontalHeader().setSectionResizeMode(QHeaderView.Stretch)
        self.table_dist.verticalHeader().setVisible(False)
        self.table_dist.setEditTriggers(QTableWidget.NoEditTriggers)
        for r, name in enumerate(["1 – 10 games", "11 – 100 games", "101 – 1,000 games", "1,001 – 10,000 games", "10,001 – 100,000 games", "100,001+ games"]):
            self.table_dist.setItem(r, 0, QTableWidgetItem(name))
            self.table_dist.setItem(r, 1, QTableWidgetItem("—"))
        dist_layout.addWidget(self.table_dist)
        diag_layout.addWidget(dist_group)

        self.tabs.addTab(tab_diag, "📊 Dev Diagnostics & Roaring Metrics")

        # Tab 3: Filter Simulation Lab
        tab_filter = QWidget()
        filter_layout = QVBoxLayout(tab_filter)

        filter_info = QLabel(
            "<b>Filter Testing Lab:</b> Test instant Opening Tree set intersections on filtered game subsets.<br>"
            "Simulates querying active search results (e.g. subset of Game IDs) using memory-mapped GameSets in RAM."
        )
        filter_info.setWordWrap(True)
        filter_layout.addWidget(filter_info)

        filt_form = QFormLayout()
        self.spin_filter_count = QSpinBox()
        self.spin_filter_count.setRange(1, 1000000)
        self.spin_filter_count.setValue(500)
        self.spin_filter_count.setSuffix(" matching game IDs (e.g. first N games)")
        filt_form.addRow("Filter Simulation Size:", self.spin_filter_count)

        btn_run_filter_test = QPushButton("⚡ Benchmark Filtered Tree Set Intersection")
        btn_run_filter_test.setStyleSheet("font-weight: bold; background-color: #6a1b9a; color: white; padding: 6px 14px;")
        btn_run_filter_test.clicked.connect(self.run_filter_benchmark)
        filt_form.addRow(btn_run_filter_test)
        filter_layout.addLayout(filt_form)

        self.txt_filter_log = QTextEdit()
        self.txt_filter_log.setReadOnly(True)
        self.txt_filter_log.setStyleSheet("font-family: monospace; font-size: 12px; background-color: #1e1e1e; color: #d4d4d4;")
        filter_layout.addWidget(self.txt_filter_log)

        self.tabs.addTab(tab_filter, "🔬 Filter Set-Intersection Lab")

        main_layout.addWidget(self.tabs)

        # Status Bar
        self.status_bar = QLabel("Ready")
        self.status_bar.setStyleSheet("color: #666; font-size: 11px; padding: 4px;")
        main_layout.addWidget(self.status_bar)

    def start_backend(self):
        backend_bin = os.path.join(SCRIPT_DIR, "..", "target", "release", "scid-mgr.exe")
        if not os.path.exists(backend_bin):
            backend_bin = os.path.join(SCRIPT_DIR, "..", "target", "debug", "scid-mgr.exe")
        try:
            self.client.start(backend_bin)
            self.status_bar.setText(f"Backend started: {backend_bin}")
        except Exception as e:
            self.status_bar.setText(f"Failed to start backend: {e}")

        # If default DB exists, open it after 250ms
        if self.db_input.text() and os.path.exists(self.db_input.text()):
            QTimer.singleShot(250, self.open_database)

    def browse_db(self):
        path, _ = QFileDialog.getOpenFileName(
            self, "Select Chess Database", "",
            "Chess Databases (*.si5 *.si4 *.pgn);;All Files (*)"
        )
        if path:
            self.db_input.setText(path)
            self.open_database()

    def open_database(self):
        path = self.db_input.text().strip()
        if not path:
            return
        self.status_bar.setText(f"Opening {path}...")
        self.client.send_request("open", {"path": path})

    def open_build_dialog(self):
        dlg = BuildPosIndexDialog(self.client, default_ply=24, parent=self)
        dlg.exec_()

    def reset_board(self):
        self.current_fen = ""
        self.move_history.clear()
        self.lbl_fen.setText("Position: Starting Board")
        self.query_opening_tree("")

    def query_opening_tree(self, fen: str, game_ids=None):
        if not self.client.is_running():
            return
        t0 = time.perf_counter()
        req_params = {"fen": fen}
        if game_ids:
            req_params["game_ids"] = game_ids

        def on_tree_resp(resp):
            elapsed_ms = (time.perf_counter() - t0) * 1000.0
            if resp.get("status") != "ok":
                self.status_bar.setText(f"Tree query error: {resp.get('error')}")
                return
            data = resp.get("data", {})
            moves = data.get("moves", [])
            total_g = data.get("total_games", 0)

            self.tree_table.setRowCount(len(moves))
            for r, m in enumerate(moves):
                self.tree_table.setItem(r, 0, QTableWidgetItem(m.get("san", "")))
                self.tree_table.setItem(r, 1, QTableWidgetItem(m.get("uci", "")))
                self.tree_table.setItem(r, 2, QTableWidgetItem(f"{m.get('total_games', 0):,}"))
                self.tree_table.setItem(r, 3, QTableWidgetItem(f"{m.get('white_pct', 0.0):.1f}%"))
                self.tree_table.setItem(r, 4, QTableWidgetItem(f"{m.get('draw_pct', 0.0):.1f}%"))
                self.tree_table.setItem(r, 5, QTableWidgetItem(f"{m.get('black_pct', 0.0):.1f}%"))
                samples = m.get("sample_game_ids", [])
                sample_str = ", ".join(str(gid) for gid in samples[:10]) + ("..." if len(samples) > 10 else "")
                self.tree_table.setItem(r, 6, QTableWidgetItem(sample_str))

            self.lbl_query_stats.setText(
                f"Query latency: {elapsed_ms:.3f} ms | Total games in pos: {total_g:,} | Candidate moves: {len(moves)}"
            )

        self.client.send_request("opening_tree", req_params, callback=on_tree_resp)

    def on_move_double_clicked(self, row, col):
        uci = self.tree_table.item(row, 1).text()
        san = self.tree_table.item(row, 0).text()
        self.move_history.append(san)
        self.lbl_fen.setText(f"Position: {' '.join(self.move_history)}")
        # For simple FEN query, we request with fen of position
        # In a full board we update shakmaty fen, for testing we can query starting moves
        self.status_bar.setText(f"Exploring move: {san} ({uci})")

    def scan_diagnostics(self):
        if not self.client.is_running():
            return
        self.lbl_diag_status.setText("Scanning all GameSets in .pos.idx...")
        t0 = time.perf_counter()

        def on_diag_resp(resp):
            elapsed_ms = (time.perf_counter() - t0) * 1000.0
            if resp.get("status") != "ok":
                self.lbl_diag_status.setText(f"Scan failed: {resp.get('error')}")
                return

            data = resp.get("data", {})
            tot = data.get("total_game_sets", 0)
            delta_cnt = data.get("delta_varint_count", 0)
            roar_cnt = data.get("roaring_count", 0)

            self.lbl_diag_total_sets.setText(f"{tot:,}")
            self.lbl_diag_delta.setText(f"{delta_cnt:,} ({delta_cnt/tot*100:.2f}%)" if tot else "0")
            self.lbl_diag_roaring.setText(f"{roar_cnt:,} ({roar_cnt/tot*100:.2f}%)" if tot else "0")

            b_delta = data.get("bytes_if_all_delta", 0)
            b_roar = data.get("bytes_if_all_roaring", 0)
            b_adapt = data.get("bytes_adaptive", 0)

            self.lbl_diag_size_delta.setText(f"{b_delta:,} bytes ({b_delta / 1048576:.2f} MB)")
            self.lbl_diag_size_roaring.setText(f"{b_roar:,} bytes ({b_roar / 1048576:.2f} MB)")
            self.lbl_diag_size_adaptive.setText(f"{b_adapt:,} bytes ({b_adapt / 1048576:.2f} MB)")

            sav_delta = ((b_delta - b_adapt) / b_delta * 100.0) if b_delta > 0 else 0.0
            sav_roar = ((b_roar - b_adapt) / b_roar * 100.0) if b_roar > 0 else 0.0
            self.lbl_diag_savings.setText(f"{sav_delta:.2f}% saved vs all-Delta | {sav_roar:.2f}% saved vs all-Roaring")

            buckets = [
                data.get("bucket_1_10", 0),
                data.get("bucket_11_100", 0),
                data.get("bucket_101_1k", 0),
                data.get("bucket_1k_10k", 0),
                data.get("bucket_10k_100k", 0),
                data.get("bucket_100k_plus", 0),
            ]
            for r, count in enumerate(buckets):
                pct = (count / tot * 100.0) if tot else 0.0
                self.table_dist.setItem(r, 1, QTableWidgetItem(f"{count:,} ({pct:.2f}%)"))

            self.lbl_diag_status.setText(f"✅ Diagnostics completed in {elapsed_ms:.1f} ms!")

        self.client.send_request("pos_index_diagnostics", {}, callback=on_diag_resp)

    def run_filter_benchmark(self):
        if not self.client.is_running():
            return
        count = self.spin_filter_count.value()
        self.txt_filter_log.append(f"--- Running Filter Simulation with {count:,} Game IDs ---")
        mock_gids = list(range(count))

        t0 = time.perf_counter()
        def on_bench_resp(resp):
            elapsed_ms = (time.perf_counter() - t0) * 1000.0
            if resp.get("status") == "ok":
                data = resp.get("data", {})
                tot_games = data.get("total_games", 0)
                moves = data.get("moves", [])
                self.txt_filter_log.append(
                    f"✅ Filtered Tree Query took: {elapsed_ms:.3f} ms!\n"
                    f"   Matched filtered games: {tot_games:,}\n"
                    f"   Moves in filtered tree: {len(moves)}\n"
                )
                for m in moves[:5]:
                    self.txt_filter_log.append(
                        f"   - {m.get('san'):<5} | Games: {m.get('total_games'):>6} | "
                        f"+{m.get('white_pct'):.1f}% / ={m.get('draw_pct'):.1f}% / -{m.get('black_pct'):.1f}%"
                    )
                self.txt_filter_log.append("--------------------------------------------------\n")
            else:
                self.txt_filter_log.append(f"❌ Query failed: {resp.get('error')}\n")

        self.client.send_request("opening_tree", {"fen": "", "game_ids": mock_gids}, callback=on_bench_resp)

    def on_backend_response(self, resp):
        if not self.client.is_running():
            return
        if resp.get("status") == "ok":
            d = resp.get("data")
            if isinstance(d, dict) and ("format" in d or "db_type" in d or "total_games" in d):
                self.status_bar.setText(f"Opened {d.get('path', 'database')} ({d.get('total_games', 0):,} games)")
                self.query_opening_tree("")
                self.scan_diagnostics()

    def on_backend_error(self, err_msg: str):
        self.status_bar.setText(f"Backend error: {err_msg}")

    def closeEvent(self, event):
        try:
            self.client.response_received.disconnect(self.on_backend_response)
            self.client.process_error.disconnect(self.on_backend_error)
        except Exception:
            pass
        self.client.stop()
        super().closeEvent(event)

def main():
    app = QApplication(sys.argv)
    window = PosIdxDevWorkbench()
    window.show()
    sys.exit(app.exec_())

if __name__ == '__main__':
    main()
