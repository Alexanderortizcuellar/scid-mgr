import os
from PyQt5.QtCore import Qt
from PyQt5.QtWidgets import (
    QDialog, QVBoxLayout, QLabel, QFormLayout, QTableWidget,
    QTableWidgetItem, QHeaderView, QHBoxLayout, QPushButton,
    QMessageBox, QProgressBar, QGroupBox, QFrame
)
from ..backend_client import BackendClient

class PosIdxDiagnosticsDialog(QDialog):
    """Dialog for inspecting .pos.idx GameSet encoding diagnostics, adaptive metrics, and space savings."""
    def __init__(self, client: BackendClient, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Position Index (.pos.idx) Diagnostics & Dev Metrics")
        self.resize(680, 560)
        self.client = client

        layout = QVBoxLayout(self)
        layout.setSpacing(12)

        # Header Info
        header_box = QGroupBox("Position Index Metadata")
        header_layout = QFormLayout(header_box)
        header_layout.setSpacing(6)

        self.lbl_status = QLabel("Checking status...")
        header_layout.addRow("Index Status:", self.lbl_status)

        self.lbl_unique_positions = QLabel("—")
        header_layout.addRow("Unique Positions:", self.lbl_unique_positions)

        self.lbl_file_size = QLabel("—")
        header_layout.addRow("Index File Size:", self.lbl_file_size)

        self.lbl_games_indexed = QLabel("—")
        header_layout.addRow("Indexed DB Games:", self.lbl_games_indexed)

        layout.addWidget(header_box)

        # Adaptive GameSet Metrics Box
        metrics_box = QGroupBox("Adaptive GameSet Encoding Metrics (Delta-Varint vs Roaring Bitmap)")
        metrics_layout = QFormLayout(metrics_box)
        metrics_layout.setSpacing(6)

        self.lbl_total_gamesets = QLabel("—")
        metrics_layout.addRow("Total Move GameSets:", self.lbl_total_gamesets)

        self.lbl_delta_chosen = QLabel("—")
        metrics_layout.addRow("DeltaVarint Chosen:", self.lbl_delta_chosen)

        self.lbl_roaring_chosen = QLabel("—")
        metrics_layout.addRow("Roaring Bitmap Chosen:", self.lbl_roaring_chosen)

        self.lbl_bytes_delta = QLabel("—")
        metrics_layout.addRow("Size if 100% DeltaVarint:", self.lbl_bytes_delta)

        self.lbl_bytes_roaring = QLabel("—")
        metrics_layout.addRow("Size if 100% Roaring:", self.lbl_bytes_roaring)

        self.lbl_bytes_adaptive = QLabel("—")
        self.lbl_bytes_adaptive.setStyleSheet("font-weight: bold; color: #2e7d32;")
        metrics_layout.addRow("Actual Adaptive Size:", self.lbl_bytes_adaptive)

        self.lbl_savings = QLabel("—")
        self.lbl_savings.setStyleSheet("font-weight: bold; color: #1565c0;")
        metrics_layout.addRow("Adaptive Space Saved:", self.lbl_savings)

        layout.addWidget(metrics_box)

        # Size Distribution Table
        dist_box = QGroupBox("GameSet ID Count Distribution")
        dist_layout = QVBoxLayout(dist_box)

        self.table_dist = QTableWidget(6, 2)
        self.table_dist.setHorizontalHeaderLabels(["GameSet ID Range", "Number of Move GameSets"])
        self.table_dist.horizontalHeader().setSectionResizeMode(QHeaderView.Stretch)
        self.table_dist.verticalHeader().setVisible(False)
        self.table_dist.setEditTriggers(QTableWidget.NoEditTriggers)

        ranges = ["1 – 10 games", "11 – 100 games", "101 – 1,000 games", "1,001 – 10,000 games", "10,001 – 100,000 games", "100,001+ games"]
        for r, label in enumerate(ranges):
            self.table_dist.setItem(r, 0, QTableWidgetItem(label))
            self.table_dist.setItem(r, 1, QTableWidgetItem("—"))

        dist_layout.addWidget(self.table_dist)
        layout.addWidget(dist_box)

        # Actions
        btn_box = QHBoxLayout()
        self.btn_refresh = QPushButton("🔄 Refresh Diagnostics")
        self.btn_refresh.setStyleSheet("font-weight: bold; padding: 6px 16px;")
        self.btn_refresh.clicked.connect(self.load_diagnostics)
        btn_box.addWidget(self.btn_refresh)

        btn_close = QPushButton("Close")
        btn_close.clicked.connect(self.accept)
        btn_box.addWidget(btn_close)
        layout.addLayout(btn_box)

        # Auto-load on open
        self.load_diagnostics()

    def load_diagnostics(self):
        if not self.client.is_running():
            self.lbl_status.setText("Backend offline")
            return

        self.lbl_status.setText("Scanning position index in background...")
        self.btn_refresh.setEnabled(False)

        # 1. Fetch pos index status
        self.client.send_request("pos_index_status", {}, callback=self.on_status_received)

    def on_status_received(self, resp):
        if not resp.get("status") == "ok":
            self.lbl_status.setText("Failed to get status")
            self.btn_refresh.setEnabled(True)
            return

        data = resp.get("data", {})
        status = data.get("status", "missing")
        header = data.get("header") or {}

        if status == "valid":
            self.lbl_status.setText("<span style='color: green; font-weight: bold;'>VALID (Companion .pos.idx up-to-date)</span>")
        elif status == "outdated":
            self.lbl_status.setText("<span style='color: orange; font-weight: bold;'>OUTDATED (Database modified since index built)</span>")
        else:
            self.lbl_status.setText("<span style='color: red; font-weight: bold;'>MISSING (No .pos.idx built)</span>")
            self.btn_refresh.setEnabled(True)
            return

        unique_pos = header.get("unique_positions", 0)
        self.lbl_unique_positions.setText(f"{unique_pos:,}")
        game_cnt = header.get("db_game_count", 0)
        self.lbl_games_indexed.setText(f"{game_cnt:,} games")

        # 2. Fetch full diagnostic statistics
        self.client.send_request("pos_index_diagnostics", {}, callback=self.on_diagnostics_received)

    def on_diagnostics_received(self, resp):
        self.btn_refresh.setEnabled(True)
        if not resp.get("status") == "ok":
            self.lbl_status.setText(f"Diagnostics error: {resp.get('error', 'Unknown')}")
            return

        data = resp.get("data", {})
        tot_sets = data.get("total_game_sets", 0)
        delta_cnt = data.get("delta_varint_count", 0)
        roar_cnt = data.get("roaring_count", 0)

        self.lbl_total_gamesets.setText(f"{tot_sets:,}")
        pct_delta = (delta_cnt / tot_sets * 100.0) if tot_sets > 0 else 0.0
        pct_roar = (roar_cnt / tot_sets * 100.0) if tot_sets > 0 else 0.0

        self.lbl_delta_chosen.setText(f"{delta_cnt:,} ({pct_delta:.2f}%)")
        self.lbl_roaring_chosen.setText(f"{roar_cnt:,} ({pct_roar:.2f}%)")

        bytes_delta = data.get("bytes_if_all_delta", 0)
        bytes_roar = data.get("bytes_if_all_roaring", 0)
        bytes_adapt = data.get("bytes_adaptive", 0)

        self.lbl_bytes_delta.setText(f"{bytes_delta:,} bytes ({bytes_delta / (1024*1024):.2f} MB)")
        self.lbl_bytes_roaring.setText(f"{bytes_roar:,} bytes ({bytes_roar / (1024*1024):.2f} MB)")
        self.lbl_bytes_adaptive.setText(f"{bytes_adapt:,} bytes ({bytes_adapt / (1024*1024):.2f} MB)")

        savings_delta = ((bytes_delta - bytes_adapt) / bytes_delta * 100.0) if bytes_delta > 0 else 0.0
        savings_roar = ((bytes_roar - bytes_adapt) / bytes_roar * 100.0) if bytes_roar > 0 else 0.0

        self.lbl_savings.setText(f"{savings_delta:.2f}% vs pure DeltaVarint | {savings_roar:.2f}% vs pure Roaring")

        # Distribution Table
        buckets = [
            data.get("bucket_1_10", 0),
            data.get("bucket_11_100", 0),
            data.get("bucket_101_1k", 0),
            data.get("bucket_1k_10k", 0),
            data.get("bucket_10k_100k", 0),
            data.get("bucket_100k_plus", 0),
        ]
        for r, count in enumerate(buckets):
            pct = (count / tot_sets * 100.0) if tot_sets > 0 else 0.0
            self.table_dist.setItem(r, 1, QTableWidgetItem(f"{count:,} ({pct:.2f}%)"))
