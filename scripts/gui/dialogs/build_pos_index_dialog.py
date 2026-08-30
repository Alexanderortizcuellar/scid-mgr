from PyQt5.QtCore import Qt
from PyQt5.QtWidgets import (
    QDialog, QVBoxLayout, QLabel, QFormLayout, QSpinBox,
    QProgressBar, QHBoxLayout, QPushButton, QMessageBox
)
from ..backend_client import BackendClient

class BuildPosIndexDialog(QDialog):
    """Dialog for creating / rebuilding .pos.idx companion file with streaming progress."""
    def __init__(self, client: BackendClient, default_ply: int = 24, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Build Fast Position Index (.pos.idx)")
        self.resize(480, 260)
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

        btn_close = QPushButton("Close")
        btn_close.clicked.connect(self.accept)
        btn_box.addWidget(btn_close)
        layout.addLayout(btn_box)

    def start_build(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Offline", "Backend is not running.")
            return
        self.btn_build.setEnabled(False)
        self.spin_depth.setEnabled(False)
        self.progress_bar.setValue(0)
        self.lbl_progress.setText("Building index across CPU cores...")
        self.client.send_request("build_pos_index", {"max_ply": self.spin_depth.value()})

    def update_progress(self, scanned: int, total: int, positions: int, percent: float):
        self.progress_bar.setValue(int(percent))
        self.lbl_progress.setText(f"Indexed: {scanned:,} / {total:,} games ({percent:.1f}%) | Unique positions: {positions:,}")

    def on_complete(self, unique_positions: int, elapsed_ms: float):
        self.progress_bar.setValue(100)
        self.lbl_progress.setText(f"✅ Finished in {elapsed_ms:,.1f} ms! Indexed {unique_positions:,} unique positions.")
        self.btn_build.setEnabled(True)
        self.spin_depth.setEnabled(True)


