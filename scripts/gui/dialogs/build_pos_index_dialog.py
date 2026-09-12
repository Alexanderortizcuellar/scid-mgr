import os
from PyQt5.QtCore import Qt
from PyQt5.QtWidgets import (
    QDialog, QVBoxLayout, QLabel, QFormLayout, QSpinBox,
    QProgressBar, QHBoxLayout, QPushButton, QMessageBox,
    QCheckBox, QGroupBox
)
from ..backend_client import BackendClient

class BuildPosIndexDialog(QDialog):
    """Unified Dialog for creating / rebuilding .pos.idx (Search Booster) and .tree.idx (Opening Tree Stats)."""
    def __init__(self, client: BackendClient, default_ply: int = 24, parent=None):
        super().__init__(parent)
        self.setWindowTitle("⚡ Build Fast Database Indexes (.pos.idx & .tree.idx)")
        self.resize(540, 390)
        self.client = client
        self.queue = []
        self.current_task = None
        self.results = {}

        layout = QVBoxLayout(self)
        layout.setSpacing(10)

        info = QLabel(
            "<b>Companion Fast Indexes</b> enable sub-millisecond opening explorer statistics and "
            "instant candidate game search acceleration without loading the full database."
        )
        info.setWordWrap(True)
        layout.addWidget(info)

        # Index Selection Group
        grp_targets = QGroupBox("Select Indexes to Build")
        box_targets = QVBoxLayout(grp_targets)

        self.chk_tree = QCheckBox("🌲 Opening Tree Stats Index (.tree.idx) — Instant opening repertoire & move statistics")
        self.chk_tree.setChecked(True)
        self.chk_tree.setStyleSheet("font-weight: bold; color: #1b5e20;")
        box_targets.addWidget(self.chk_tree)

        self.chk_pos = QCheckBox("⚡ Position Search Booster (.pos.idx) — Sub-millisecond position candidate searches")
        self.chk_pos.setChecked(True)
        self.chk_pos.setStyleSheet("font-weight: bold; color: #0d47a1;")
        box_targets.addWidget(self.chk_pos)

        layout.addWidget(grp_targets)

        # Options Form
        form = QFormLayout()
        self.spin_depth = QSpinBox()
        self.spin_depth.setRange(4, 100)
        self.spin_depth.setValue(default_ply)
        self.spin_depth.setSuffix(" plies (half-moves)")
        form.addRow("Indexing Depth:", self.spin_depth)

        self.spin_min_games = QSpinBox()
        self.spin_min_games.setRange(1, 100000)
        self.spin_min_games.setValue(1)
        self.spin_min_games.setSpecialValueText("1 (Include all positions)")
        self.spin_min_games.setSuffix(" occurrences min")
        form.addRow("Min Position Frequency:", self.spin_min_games)

        cpu_count = os.cpu_count() or 4
        self.spin_threads = QSpinBox()
        self.spin_threads.setRange(1, cpu_count)
        self.spin_threads.setValue(max(1, cpu_count))
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

        self.btn_view_diagnostics = QPushButton("📊 View Diagnostics...")
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

    def update_progress(self, scanned: int, total: int, positions: int, pct: float, task_name: str = ""):
        if not task_name:
            task_name = "Opening Tree (.tree.idx)" if self.current_task == "build_tree" else "Position Booster (.pos.idx)"
        self.progress_bar.setValue(int(pct))
        self.lbl_progress.setText(
            f"[{task_name}] Indexed: {scanned:,} / {total:,} games ({pct:.1f}%) | Unique positions: {positions:,}"
        )

    def _on_backend_message(self, data: dict):
        if not self.isVisible():
            return
        event = data.get("event")
        if event in ("build_pos_index_progress", "build_tree_progress"):
            prog = data.get("data", {})
            scanned = prog.get("scanned", 0)
            total = prog.get("total", 0)
            positions = prog.get("positions", 0)
            pct = prog.get("percent", 0.0)
            task_name = "Opening Tree (.tree.idx)" if event == "build_tree_progress" else "Position Booster (.pos.idx)"
            self.update_progress(scanned, total, positions, pct, task_name)
        elif data.get("status") == "ok":
            res_data = data.get("data", {})
            if "unique_positions" in res_data and "elapsed_ms" in res_data and "moves" not in res_data:
                self._handle_task_complete(res_data)

    def start_build(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Offline", "Backend is not running.")
            return

        self.queue = []
        if self.chk_tree.isChecked():
            self.queue.append("build_tree")
        if self.chk_pos.isChecked():
            self.queue.append("build_pos_index")

        if not self.queue:
            QMessageBox.warning(self, "No Index Selected", "Please select at least one index to build.")
            return

        self.btn_build.setEnabled(False)
        self.btn_view_diagnostics.setVisible(False)
        self.chk_tree.setEnabled(False)
        self.chk_pos.setEnabled(False)
        self.spin_depth.setEnabled(False)
        self.spin_min_games.setEnabled(False)
        self.spin_threads.setEnabled(False)
        self.results = {}
        self._run_next_task()

    def _run_next_task(self):
        if not self.queue:
            self._all_tasks_completed()
            return

        self.current_task = self.queue.pop(0)
        threads = self.spin_threads.value()
        depth = self.spin_depth.value()
        min_g = self.spin_min_games.value()
        task_label = "Opening Tree (.tree.idx)" if self.current_task == "build_tree" else "Position Booster (.pos.idx)"
        self.progress_bar.setValue(0)
        self.lbl_progress.setText(f"Starting {task_label} build using {threads} worker threads...")

        self.client.send_request(self.current_task, {
            "max_ply": depth,
            "min_games": min_g,
            "threads": threads,
        })

    def _handle_task_complete(self, res_data: dict):
        if not self.current_task:
            return
        self.results[self.current_task] = res_data
        if self.current_task == "build_pos_index" and "diagnostics" in res_data:
            self.last_diagnostics = res_data.get("diagnostics")

        # Run next in queue if available
        self._run_next_task()

    def _all_tasks_completed(self):
        self.progress_bar.setValue(100)
        summary_lines = []
        for task, res in self.results.items():
            name = "Tree Index (.tree.idx)" if task == "build_tree" else "Position Booster (.pos.idx)"
            pos_cnt = res.get("unique_positions", 0)
            elapsed = res.get("elapsed_ms", 0.0)
            summary_lines.append(f"✅ {name}: {pos_cnt:,} positions in {elapsed:,.0f} ms")

        self.lbl_progress.setText(" | ".join(summary_lines))
        self.btn_build.setEnabled(True)
        self.chk_tree.setEnabled(True)
        self.chk_pos.setEnabled(True)
        self.spin_depth.setEnabled(True)
        self.spin_min_games.setEnabled(True)
        self.spin_threads.setEnabled(True)
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
