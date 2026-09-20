#!/usr/bin/env python3
import os
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

from PyQt5.QtWidgets import QApplication, QMainWindow, QVBoxLayout, QWidget
from gui.backend_client import BackendClient
from gui.widgets.cql_search_widget import CqlSearchWidget


class CqlSearchStandaloneWindow(QMainWindow):
    """
    Standalone GUI Window for Testing & Developing CQL / Search DSL queries.
    """

    def __init__(self):
        super().__init__()
        self.setWindowTitle("CQL / Chess Search Engine Workbench")
        self.resize(1200, 780)

        self.client = BackendClient(self)
        self.init_backend()

        central = QWidget()
        self.setCentralWidget(central)
        layout = QVBoxLayout(central)
        layout.setContentsMargins(6, 6, 6, 6)

        self.search_widget = CqlSearchWidget(self.client, self)
        layout.addWidget(self.search_widget)

    def init_backend(self):
        root_dir = os.path.dirname(SCRIPT_DIR)
        binary_path = os.path.join(root_dir, "target", "release", "scid-mgr.exe" if sys.platform == "win32" else "scid-mgr")
        if not os.path.exists(binary_path):
            print(f"ERROR: Release binary not found at: {binary_path}")
            print("Please compile the optimized engine using: cargo build --release")
            return

        print(f"Loaded scid-mgr release binary: {binary_path}")
        self.client.start(binary_path)

    def closeEvent(self, event):
        self.client.stop()
        super().closeEvent(event)


def main():
    app = QApplication(sys.argv)
    window = CqlSearchStandaloneWindow()
    window.show()
    sys.exit(app.exec_())


if __name__ == "__main__":
    main()
