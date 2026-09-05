import os
from PyQt5.QtCore import Qt
from PyQt5.QtWidgets import (
    QDialog, QVBoxLayout, QLabel, QFormLayout, QSpinBox,
    QProgressBar, QHBoxLayout, QPushButton, QMessageBox
)
from ..backend_client import BackendClient

class BuildPosIndexDialog(QDialog):
    """Dialog for creating / rebuilding .pos.idx companion file with streaming progress."""
    def __init__(self, client: BackendClient, default_ply: int = 16, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Build Fast Position Index (.pos.idx)")
        self.resize(500, 290)
        self.client = client

        layout = QVBoxLayout(self)
        layout.setSpacing(10)

        info = QLabel(
            "<b>Companion Position Index (.pos.idx)</b> enables sub-millisecond position searches\n"
            "and instant Lichess/ChessBase style opening trees with win/draw/loss statistics."
        )
        info.setWordWrap(True)
        layout.addWidget(info)

        form = QFormLayout()
        self.spin_depth = QSpinBox()
        self.spin_depth.setRange(4, 100)
        self.spin_depth.setValue(default_ply)
        self.spin_depth.setSuffix(" plies (half-moves)")
        form.addRow("Indexing Depth:", self.spin_depth)

        self.spin_max_games = QSpinBox()
        self.spin_max_games.setRange(0, 100000)
        self.spin_max_games.setValue(0)
        self.spin_max_games.setSpecialValueText("All games (Complete Inverted Index)")
        self.spin_max_games.setSuffix(" games per move")
        form.addRow("Max Games / IDs per Move:", self.spin_max_games)

        cpu_count = os.cpu_count() or 4
        self.spin_threads = QSpinBox()
        self.spin_threads.setRange(1, cpu_count)
        self.spin_threads.setValue(max(1, cpu_count // 2))
        self.spin_threads.setSuffix(f" threads (of {cpu_count} CPU cores)")
        form.addRow("CPU Worker Threads:", self.spin_threads)
        layout.addLayout(form)

        self.progress_bar = QProgressBar()
        self.progress_bar.setRange(0, 100)
        self.progress_bar.setValue(0)
        layout.addWidget(self.progress_bar)

        self.lbl_progress = QLabel("Status: Ready to build")
        self.lbl_progress.setStyleSheet("color: #555; font-size: 11px;")
        layout.addWidget(self.lbl_progress)

        btn_box = QHBoxLayout()
        self.btn_build = QPushButton("⚡ Start Indexing")
        self.btn_build.setStyleSheet("font-weight: bold; background-color: #2e7d32; color: white; padding: 6px 16px;")
        self.btn_build.clicked.connect(self.start_build)
        btn_box.addWidget(self.btn_build)

        self.btn_view_diagnostics = QPushButton("📊 View Diagnostics & Benchmark...")
        self.btn_view_diagnostics.setStyleSheet("font-weight: bold; background-color: #1565c0; color: white; padding: 6px 14px;")
        self.btn_view_diagnostics.setVisible(False)
        self.btn_view_diagnostics.clicked.connect(self.open_diagnostics)
        btn_box.addWidget(self.btn_view_diagnostics)

        btn_close = QPushButton("Close")
        btn_close.clicked.connect(self.accept)
        btn_box.addWidget(btn_close)
        layout.addLayout(btn_box)

        self.last_diagnostics = None
        self.client.response_received.connect(self._on_backend_message)

    def _on_backend_message(self, data: dict):
        if not self.isVisible():
            return
        if data.get("event") == "build_pos_index_progress":
            prog = data.get("data", {})
            self.update_progress(
                prog.get("scanned", 0),
                prog.get("total", 0),
                prog.get("positions", 0),
                prog.get("percent", 0.0),
            )
        elif data.get("status") == "ok":
            res_data = data.get("data", {})
            if "unique_positions" in res_data and "elapsed_ms" in res_data and "moves" not in res_data:
                self.on_complete(
                    res_data.get("unique_positions", 0),
                    res_data.get("elapsed_ms", 0.0),
                    res_data.get("diagnostics"),
                )

    def start_build(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Offline", "Backend is not running.")
            return
        self.btn_build.setEnabled(False)
        self.btn_view_diagnostics.setVisible(False)
        self.spin_depth.setEnabled(False)
        self.spin_max_games.setEnabled(False)
        self.spin_threads.setEnabled(False)
        self.progress_bar.setValue(0)
        threads = self.spin_threads.value()
        self.lbl_progress.setText(f"Building index using {threads} worker threads...")
        self.client.send_request("build_pos_index", {
            "max_ply": self.spin_depth.value(),
            "max_games": self.spin_max_games.value(),
            "threads": threads,
        })

    def update_progress(self, scanned: int, total: int, positions: int, percent: float):
        self.progress_bar.setValue(int(percent))
        self.lbl_progress.setText(f"Indexed: {scanned:,} / {total:,} games ({percent:.1f}%) | Unique positions: {positions:,}")

    def on_complete(self, unique_positions: int, elapsed_ms: float, diagnostics: dict = None):
        self.progress_bar.setValue(100)
        self.lbl_progress.setText(f"✅ Finished in {elapsed_ms:,.1f} ms! Indexed {unique_positions:,} unique positions.")
        self.btn_build.setEnabled(True)
        self.spin_depth.setEnabled(True)
        self.spin_max_games.setEnabled(True)
        self.spin_threads.setEnabled(True)
        self.last_diagnostics = diagnostics
        self.btn_view_diagnostics.setVisible(True)

    def open_diagnostics(self):
        from .pos_idx_diagnostics_dialog import PosIdxDiagnosticsDialog
        diag = PosIdxDiagnosticsDialog(self.client, initial_data=self.last_diagnostics, parent=self)
        diag.exec_()

    def closeEvent(self, event):
        try:
            self.client.response_received.disconnect(self._on_backend_message)
        except Exception:
            pass
        super().closeEvent(event)



