from typing import Optional, Dict, Any
from PyQt5.QtCore import Qt
from PyQt5.QtGui import QFont, QColor
from PyQt5.QtWidgets import (
    QDialog, QVBoxLayout, QHBoxLayout, QLabel, QPushButton, QTableWidget,
    QTableWidgetItem, QHeaderView, QCheckBox, QProgressBar, QMessageBox
)
from ..backend_client import BackendClient

class BenchmarkDialog(QDialog):
    def __init__(self, client: BackendClient, current_stats: Optional[dict] = None, parent=None):
        super().__init__(parent)
        self.setWindowTitle("📊 Database Performance Benchmark & Metrics")
        self.resize(920, 620)
        self.client = client
        self.current_stats = current_stats
        self.report_data = None
        self.init_ui()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(12, 12, 12, 12)
        layout.setSpacing(10)

        # Header info card
        info_frame = QFrame()
        info_frame.setFrameShape(QFrame.StyledPanel)
        info_frame.setStyleSheet("background-color: #f8f9fa; border: 1px solid #dee2e6; border-radius: 6px; padding: 6px;")
        info_layout = QHBoxLayout(info_frame)

        db_name = "None"
        fmt = "-"
        total = 0
        if self.current_stats:
            path = self.current_stats.get("path") or self.current_stats.get("index_path", "")
            db_name = os.path.basename(path)
            fmt = str(self.current_stats.get("format", "")).upper()
            total = self.current_stats.get("total_games", 0)

        self.lbl_db_info = QLabel(f"Database: {db_name} ({fmt}) | Total Games: {total:,}")
        self.lbl_db_info.setStyleSheet("font-weight: bold; font-size: 13px; color: #212529;")
        info_layout.addWidget(self.lbl_db_info)
        info_layout.addStretch()

        self.chk_heavy = QCheckBox("Deep Position Search")
        self.chk_heavy.setToolTip("Performs full position search across opening plies (recommended for databases < 500k games)")
        info_layout.addWidget(self.chk_heavy)

        self.btn_run = QPushButton("▶ Run Full Benchmark")
        self.btn_run.setStyleSheet("font-weight: bold; background-color: #2e7d32; color: white; padding: 6px 14px; border-radius: 4px;")
        self.btn_run.clicked.connect(self.run_benchmark)
        info_layout.addWidget(self.btn_run)

        layout.addWidget(info_frame)

        # Progress Bar
        self.progress_bar = QProgressBar()
        self.progress_bar.setRange(0, 0)
        self.progress_bar.setVisible(False)
        layout.addWidget(self.progress_bar)

        # Results Table
        self.table = QTableWidget()
        self.table.setColumnCount(5)
        self.table.setHorizontalHeaderLabels(["Category", "Benchmark Operation", "Time (ms)", "Count / Matches", "Details & Throughput"])
        self.table.horizontalHeader().setSectionResizeMode(0, QHeaderView.ResizeToContents)
        self.table.horizontalHeader().setSectionResizeMode(1, QHeaderView.Stretch)
        self.table.horizontalHeader().setSectionResizeMode(2, QHeaderView.ResizeToContents)
        self.table.horizontalHeader().setSectionResizeMode(3, QHeaderView.ResizeToContents)
        self.table.horizontalHeader().setSectionResizeMode(4, QHeaderView.Stretch)
        self.table.setAlternatingRowColors(True)
        self.table.setStyleSheet("QTableWidget { font-size: 11px; } QHeaderView::section { font-weight: bold; }")
        layout.addWidget(self.table)

        # Summary footer
        self.lbl_summary = QLabel("Ready to benchmark. Click 'Run Full Benchmark' to measure performance across all operations.")
        self.lbl_summary.setStyleSheet("font-style: italic; color: #555; font-size: 11px;")
        layout.addWidget(self.lbl_summary)

        # Actions row
        btn_box = QHBoxLayout()
        self.btn_copy = QPushButton("📋 Copy Report")
        self.btn_copy.clicked.connect(self.copy_report)
        self.btn_copy.setEnabled(False)
        btn_box.addWidget(self.btn_copy)
        btn_box.addStretch()

        btn_close = QPushButton("Close")
        btn_close.clicked.connect(self.accept)
        btn_box.addWidget(btn_close)
        layout.addLayout(btn_box)

    def run_benchmark(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Not Connected", "Please open a database first.")
            return
        self.btn_run.setEnabled(False)
        self.progress_bar.setVisible(True)
        self.lbl_summary.setText("Running comprehensive multi-threaded benchmarks... Please wait...")
        self.table.setRowCount(0)
        self.client.send_request("benchmark", {"heavy": self.chk_heavy.isChecked()})

    def display_report(self, report: dict):
        self.report_data = report
        self.btn_run.setEnabled(True)
        self.progress_bar.setVisible(False)
        self.btn_copy.setEnabled(True)

        total_games = report.get("total_games", 0)
        fmt = report.get("format", "")
        size_mb = report.get("file_size_mb", 0.0)
        db_path = report.get("db_path", "")
        db_name = os.path.basename(db_path)

        self.lbl_db_info.setText(f"Database: {db_name} ({fmt.upper()}) | Total Games: {total_games:,} | Size: {size_mb:.2f} MB")

        results = report.get("results", [])
        self.table.setRowCount(len(results))
        for row, item in enumerate(results):
            cat_text = item.get("category", "")
            cat = QTableWidgetItem(cat_text)
            name = QTableWidgetItem(item.get("name", ""))
            ms = item.get("elapsed_ms", 0.0)
            ms_item = QTableWidgetItem(f"{ms:,.2f} ms")
            ms_item.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)

            count = item.get("count", 0)
            count_item = QTableWidgetItem(f"{count:,}")
            count_item.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)

            notes = QTableWidgetItem(item.get("notes", ""))

            # Color code performance
            if "Sort" in cat_text or "Filter" in cat_text or "Index" in cat_text:
                if ms < 500:
                    ms_item.setForeground(QColor("#2e7d32")) # Green
                elif ms < 2000:
                    ms_item.setForeground(QColor("#e65100")) # Orange
                else:
                    ms_item.setForeground(QColor("#c62828")) # Red

            self.table.setItem(row, 0, cat)
            self.table.setItem(row, 1, name)
            self.table.setItem(row, 2, ms_item)
            self.table.setItem(row, 3, count_item)
            self.table.setItem(row, 4, notes)

        tot_ms = report.get("total_time_ms", 0.0)
        self.lbl_summary.setText(f"Completed {len(results)} benchmark operations in {tot_ms:,.2f} ms ({tot_ms/1000.0:.2f} s).")

    def copy_report(self):
        if not self.report_data:
            return
        lines = []
        lines.append("DATABASE PERFORMANCE BENCHMARK REPORT")
        lines.append("=" * 80)
        lines.append(f"Database:    {self.report_data.get('db_path')}")
        lines.append(f"Format:      {self.report_data.get('format')}")
        lines.append(f"Total Games: {self.report_data.get('total_games', 0):,}")
        lines.append(f"Disk Size:   {self.report_data.get('file_size_mb', 0.0):.2f} MB")
        lines.append("-" * 80)
        lines.append(f"{'Category':<22} | {'Operation':<42} | {'Time (ms)':>10} | {'Details'}")
        lines.append("-" * 80)
        for it in self.report_data.get("results", []):
            lines.append(f"{it.get('category',''):<22} | {it.get('name',''):<42} | {it.get('elapsed_ms',0.0):>10.2f} | {it.get('notes','')}")
        lines.append("=" * 80)
        lines.append(f"Total Benchmark Time: {self.report_data.get('total_time_ms',0.0):.2f} ms\n")
        QApplication.clipboard().setText("\n".join(lines))
        QMessageBox.information(self, "Copied", "Benchmark report copied to clipboard!")


