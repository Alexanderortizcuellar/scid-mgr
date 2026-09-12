import os
from PyQt5.QtCore import Qt
from PyQt5.QtWidgets import (
    QDialog, QVBoxLayout, QLabel, QFormLayout, QTableWidget,
    QTableWidgetItem, QHeaderView, QHBoxLayout, QPushButton,
    QMessageBox, QProgressBar, QGroupBox, QFrame
)
from PyQt5.QtGui import QColor, QFont
from ..backend_client import BackendClient

class PosIdxDiagnosticsDialog(QDialog):
    """Dialog displaying the Position Index (.pos.idx) Inlined Singletons and Postings Diagnostics Report."""
    def __init__(self, client: BackendClient, initial_data: dict = None, parent=None):
        super().__init__(parent)
        self.setWindowTitle("📊 Position Search Booster (.pos.idx) Diagnostics Report")
        self.resize(840, 620)
        self.client = client

        layout = QVBoxLayout(self)
        layout.setSpacing(12)

        # Header Title
        title = QLabel(
            "<h3 style='margin:0; color:#1565c0;'>⚡ Position Search Booster: Inlined Singletons & Inverted Index Report</h3>"
            "<span style='color:#555; font-size:11px;'>Analyzes 64-bit Zobrist keys, zero-byte inlined singletons, and Delta-Varint compressed postings.</span>"
        )
        title.setWordWrap(True)
        layout.addWidget(title)

        # 1. Summary Overview Table
        comp_group = QGroupBox("Index Architecture & Storage Overview")
        comp_layout = QVBoxLayout(comp_group)

        self.table_comp = QTableWidget(5, 2)
        self.table_comp.setHorizontalHeaderLabels(["Metric", "Value / Performance"])
        self.table_comp.horizontalHeader().setSectionResizeMode(0, QHeaderView.ResizeToContents)
        self.table_comp.horizontalHeader().setSectionResizeMode(1, QHeaderView.Stretch)
        self.table_comp.verticalHeader().setVisible(False)
        self.table_comp.setEditTriggers(QTableWidget.NoEditTriggers)

        metrics = [
            ("Total Unique Positions", "—"),
            ("Inlined Singletons (0 Bytes Payload)", "—"),
            ("Multi-Game Delta-Varint Postings", "—"),
            ("Compressed Data Payload Size", "—"),
            ("Search Candidate Filter Speed", "< 0.01 ms (Direct Mmap Binary Search)"),
        ]
        for r, (m, v) in enumerate(metrics):
            self.table_comp.setItem(r, 0, QTableWidgetItem(m))
            self.table_comp.setItem(r, 1, QTableWidgetItem(v))
            if r == 1:
                item_v = self.table_comp.item(r, 1)
                item_v.setBackground(QColor("#e8f5e9"))
                font = item_v.font()
                font.setBold(True)
                item_v.setFont(font)

        comp_layout.addWidget(self.table_comp)
        layout.addWidget(comp_group)

        # 2. Size Distribution Table
        dist_group = QGroupBox("Position Frequency Distribution Across Database")
        dist_layout = QVBoxLayout(dist_group)

        self.table_dist = QTableWidget(6, 3)
        self.table_dist.setHorizontalHeaderLabels(["Game Occurrence Range", "Positions Count", "Storage Strategy"])
        self.table_dist.horizontalHeader().setSectionResizeMode(QHeaderView.Stretch)
        self.table_dist.verticalHeader().setVisible(False)
        self.table_dist.setEditTriggers(QTableWidget.NoEditTriggers)

        ranges = [
            ("1 game (Singletons)", "—", "Inlined into Directory Table (0 bytes payload)"),
            ("2 – 10 games", "—", "Delta-Varint Compressed"),
            ("11 – 100 games", "—", "Delta-Varint Compressed"),
            ("101 – 1,000 games", "—", "Delta-Varint Compressed"),
            ("1,001 – 10,000 games", "—", "Delta-Varint Compressed"),
            ("10,001+ games (Main Lines)", "—", "Delta-Varint Compressed"),
        ]
        for r, (label, count, reason) in enumerate(ranges):
            self.table_dist.setItem(r, 0, QTableWidgetItem(label))
            self.table_dist.setItem(r, 1, QTableWidgetItem(count))
            self.table_dist.setItem(r, 2, QTableWidgetItem(reason))

        dist_layout.addWidget(self.table_dist)
        layout.addWidget(dist_group)

        # Status Bar / Actions
        self.lbl_status = QLabel("Ready")
        self.lbl_status.setStyleSheet("color: #666; font-size: 11px;")
        layout.addWidget(self.lbl_status)

        btn_box = QHBoxLayout()
        self.btn_refresh = QPushButton("🔄 Refresh Diagnostics")
        self.btn_refresh.setStyleSheet("font-weight: bold; padding: 6px 16px;")
        self.btn_refresh.clicked.connect(self.load_diagnostics)
        btn_box.addWidget(self.btn_refresh)

        btn_close = QPushButton("Close")
        btn_close.clicked.connect(self.accept)
        btn_box.addWidget(btn_close)
        layout.addLayout(btn_box)

        if initial_data:
            self.populate_data(initial_data)
        else:
            self.load_diagnostics()

    def load_diagnostics(self):
        if not self.client.is_running():
            self.lbl_status.setText("Backend offline")
            return
        self.lbl_status.setText("Scanning position index diagnostics in background...")
        self.btn_refresh.setEnabled(False)
        self.client.send_request("pos_index_diagnostics", {}, callback=self.on_diagnostics_received)

    def on_diagnostics_received(self, resp):
        self.btn_refresh.setEnabled(True)
        if resp.get("status") != "ok":
            self.lbl_status.setText(f"Diagnostics error: {resp.get('error', 'Unknown')}")
            return
        self.populate_data(resp.get("data", {}))

    def populate_data(self, data: dict):
        tot_pos = data.get("total_positions", 0)
        tot_postings = data.get("total_postings", 0)
        inlined = data.get("inlined_singletons", 0)
        payload_bytes = data.get("bytes_payload", 0)
        payload_mb = payload_bytes / 1048576.0

        pct_inlined = (inlined / tot_pos * 100.0) if tot_pos > 0 else 0.0

        # Row 0: Total Positions
        self.table_comp.setItem(0, 1, QTableWidgetItem(f"{tot_pos:,} unique Zobrist keys"))
        # Row 1: Inlined Singletons
        self.table_comp.setItem(1, 1, QTableWidgetItem(f"{inlined:,} positions ({pct_inlined:.1f}%) — 0 Bytes Payload!"))
        # Row 2: Multi-game postings
        self.table_comp.setItem(2, 1, QTableWidgetItem(f"{tot_postings:,} total occurrences"))
        # Row 3: Payload size
        self.table_comp.setItem(3, 1, QTableWidgetItem(f"{payload_mb:.2f} MB ({payload_bytes:,} bytes)"))

        # Distribution Table
        buckets = [
            inlined,
            data.get("bucket_1_10", 0) - inlined if data.get("bucket_1_10", 0) >= inlined else data.get("bucket_1_10", 0),
            data.get("bucket_11_100", 0),
            data.get("bucket_101_1k", 0),
            data.get("bucket_1k_10k", 0),
            data.get("bucket_10k_100k", 0) + data.get("bucket_100k_plus", 0),
        ]
        for r, count in enumerate(buckets):
            pct = (count / tot_pos * 100.0) if tot_pos > 0 else 0.0
            self.table_dist.setItem(r, 1, QTableWidgetItem(f"{count:,} ({pct:.2f}%)"))

        self.lbl_status.setText(f"✅ Analyzed {tot_pos:,} positions ({inlined:,} inlined singletons saved ~{inlined * 5 / 1048576.0:.1f} MB payload).")
