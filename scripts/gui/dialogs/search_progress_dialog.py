import time
from PyQt5.QtCore import Qt, QTimer
from PyQt5.QtWidgets import (
    QDialog, QVBoxLayout, QLabel, QProgressBar, QHBoxLayout, QPushButton
)

class SearchProgressDialog(QDialog):
    """
    Non-blocking / live progress dialog displayed during long searches across millions of games.
    Automatically updates with scanned count, matches found, and scanning speed.
    """
    def __init__(self, title: str = "Searching Games...", parent=None):
        super().__init__(parent)
        self.setWindowTitle(title)
        self.resize(460, 180)
        self.setWindowFlags(self.windowFlags() & ~Qt.WindowContextHelpButtonHint)

        layout = QVBoxLayout(self)
        layout.setSpacing(10)

        self.lbl_title = QLabel(f"<b>🔍 {title}</b>")
        self.lbl_title.setStyleSheet("font-size: 13px;")
        layout.addWidget(self.lbl_title)

        self.progress_bar = QProgressBar()
        self.progress_bar.setRange(0, 100)
        self.progress_bar.setValue(0)
        self.progress_bar.setTextVisible(True)
        self.progress_bar.setStyleSheet("""
            QProgressBar {
                border: 1px solid #bbb;
                border-radius: 4px;
                text-align: center;
                height: 22px;
                font-weight: bold;
            }
            QProgressBar::chunk {
                background-color: #1976d2;
                border-radius: 3px;
            }
        """)
        layout.addWidget(self.progress_bar)

        self.lbl_scanned = QLabel("Scanned: 0 / 0 games (0.0%)")
        self.lbl_scanned.setStyleSheet("color: #333; font-size: 11px;")
        layout.addWidget(self.lbl_scanned)

        self.lbl_matches = QLabel("🎯 Matches found: 0")
        self.lbl_matches.setStyleSheet("color: #2e7d32; font-weight: bold; font-size: 11px;")
        layout.addWidget(self.lbl_matches)

        self.lbl_speed = QLabel("⚡ Initializing scanner...")
        self.lbl_speed.setStyleSheet("color: #666; font-size: 10px;")
        layout.addWidget(self.lbl_speed)

        btn_box = QHBoxLayout()
        btn_box.addStretch()
        self.btn_cancel = QPushButton("Cancel")
        self.btn_cancel.clicked.connect(self.reject)
        btn_box.addWidget(self.btn_cancel)
        layout.addLayout(btn_box)

        self.start_time = time.time()
        self.last_scanned = 0

    def update_progress(self, scanned: int, total: int, matches: int, percent: float):
        self.progress_bar.setValue(min(100, int(percent)))
        self.lbl_scanned.setText(f"Scanned: {scanned:,} / {total:,} games ({percent:.1f}%)")
        self.lbl_matches.setText(f"🎯 Matches found: {matches:,}")

        elapsed = time.time() - self.start_time
        if elapsed > 0.3 and scanned > 0:
            speed = scanned / elapsed
            self.lbl_speed.setText(f"⚡ Scanning Speed: ~{speed:,.0f} games/sec (Elapsed: {elapsed:.1f}s)")

    def on_finished(self, total_matches: int = 0):
        self.progress_bar.setValue(100)
        elapsed = time.time() - self.start_time
        self.lbl_matches.setText(f"✅ Search complete! Found {total_matches:,} matching games in {elapsed:.2f}s")
        QTimer.singleShot(400, self.accept)
