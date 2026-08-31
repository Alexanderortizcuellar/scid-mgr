import os
from typing import Optional
from PyQt5.QtCore import Qt, QSettings
from PyQt5.QtWidgets import (
    QDialog, QVBoxLayout, QHBoxLayout, QGridLayout, QLabel,
    QSlider, QSpinBox, QPushButton, QGroupBox, QMessageBox
)

class SettingsDialog(QDialog):
    """
    Settings / Preferences Dialog allowing users to configure CPU thread limits,
    indexing parameters, and backend performance profiles.
    """
    def __init__(self, backend_client=None, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Application & Search Settings")
        self.resize(480, 260)
        self.setWindowFlags(self.windowFlags() & ~Qt.WindowContextHelpButtonHint)

        self.client = backend_client
        self.settings = QSettings("chess-scid-rw", "ScidDatabaseManager")
        self.max_system_cpus = os.cpu_count() or 4

        self.init_ui()
        self.load_settings()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setSpacing(12)

        # Performance & CPU Threading Group
        cpu_group = QGroupBox("⚡ Parallel Processing & CPU Resource Limit")
        cpu_layout = QGridLayout(cpu_group)
        cpu_layout.setSpacing(10)

        cpu_layout.addWidget(QLabel("<b>Worker Threads for Searches & Indexing:</b>"), 0, 0, 1, 2)

        slider_row = QHBoxLayout()
        self.slider_threads = QSlider(Qt.Horizontal)
        self.slider_threads.setRange(1, self.max_system_cpus)
        self.slider_threads.setTickPosition(QSlider.TicksBelow)
        self.slider_threads.setTickInterval(1)
        slider_row.addWidget(self.slider_threads)

        self.spin_threads = QSpinBox()
        self.spin_threads.setRange(1, self.max_system_cpus)
        slider_row.addWidget(self.spin_threads)
        cpu_layout.addLayout(slider_row, 1, 0, 1, 2)

        self.slider_threads.valueChanged.connect(self.spin_threads.setValue)
        self.spin_threads.valueChanged.connect(self.slider_threads.setValue)
        self.spin_threads.valueChanged.connect(self.update_cpu_hint)

        self.lbl_cpu_info = QLabel()
        self.lbl_cpu_info.setStyleSheet("color: #555; font-size: 11px;")
        cpu_layout.addWidget(self.lbl_cpu_info, 2, 0, 1, 2)

        layout.addWidget(cpu_group)

        # Action Buttons
        btn_box = QHBoxLayout()
        btn_box.addStretch()

        self.btn_cancel = QPushButton("Cancel")
        self.btn_cancel.clicked.connect(self.reject)
        btn_box.addWidget(self.btn_cancel)

        self.btn_save = QPushButton("Save & Apply")
        self.btn_save.setStyleSheet("font-weight: bold; background-color: #1976d2; color: white; padding: 6px 14px;")
        self.btn_save.clicked.connect(self.save_settings)
        btn_box.addWidget(self.btn_save)

        layout.addLayout(btn_box)

    def update_cpu_hint(self, threads: int):
        pct = (threads / self.max_system_cpus) * 100.0
        if threads == self.max_system_cpus:
            rec = "🔥 <b>Maximum Speed</b> — Will utilize 100% of all logical CPU cores during searches."
        elif threads >= self.max_system_cpus // 2:
            rec = "⚡ <b>Balanced (Recommended)</b> — Fast searches while leaving CPU headroom for system responsiveness."
        else:
            rec = "🍃 <b>Low CPU Usage</b> — Minimizes system load and background resource consumption."
        
        self.lbl_cpu_info.setText(
            f"Configured: <b>{threads} / {self.max_system_cpus} threads</b> (~{pct:.0f}% CPU capacity)<br>{rec}"
        )

    def load_settings(self):
        # Default to max - 1 or at least half of CPUs to avoid 100% saturation spikes
        recommended_threads = max(1, self.max_system_cpus - 1) if self.max_system_cpus > 2 else self.max_system_cpus
        saved_threads = int(self.settings.value("worker_threads", recommended_threads))
        saved_threads = max(1, min(self.max_system_cpus, saved_threads))
        self.spin_threads.setValue(saved_threads)
        self.update_cpu_hint(saved_threads)

    def save_settings(self):
        threads = self.spin_threads.value()
        self.settings.setValue("worker_threads", threads)

        if self.client and self.client.is_running():
            self.client.send_request("set_threads", {"threads": threads})

        self.accept()
