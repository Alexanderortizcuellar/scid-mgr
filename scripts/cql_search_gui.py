#!/usr/bin/env python3
import os
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

from PyQt5.QtWidgets import QApplication, QMainWindow, QVBoxLayout, QWidget, QAction, QMessageBox
from PyQt5.QtGui import QKeySequence
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

        self.init_menus()

    def init_menus(self):
        menubar = self.menuBar()

        # File Menu
        file_menu = menubar.addMenu("&File")

        act_open_db = QAction("📂 &Open Database (.si5, .si4, .pgn)...", self)
        act_open_db.setShortcut(QKeySequence.Open)  # Ctrl+O
        act_open_db.triggered.connect(self.search_widget.open_database_dialog)
        file_menu.addAction(act_open_db)

        act_browse_pgn = QAction("📄 Select Custom &PGN File...", self)
        act_browse_pgn.setShortcut("Ctrl+Shift+O")
        act_browse_pgn.triggered.connect(lambda: (
            self.search_widget.radio_custom_pgn.setChecked(True),
            self.search_widget.browse_pgn_file()
        ))
        file_menu.addAction(act_browse_pgn)

        file_menu.addSeparator()

        act_exit = QAction("E&xit", self)
        act_exit.setShortcut("Ctrl+Q")
        act_exit.triggered.connect(self.close)
        file_menu.addAction(act_exit)

        # Query Menu
        query_menu = menubar.addMenu("&Query")

        act_run = QAction("▶ &Run Search", self)
        act_run.setShortcut("F5")
        act_run.triggered.connect(self.search_widget.execute_search)
        query_menu.addAction(act_run)

        act_explain = QAction("💡 &Explain Query (Plan & Symmetries)", self)
        act_explain.setShortcut("Ctrl+E")
        act_explain.triggered.connect(self.search_widget.explain_current_query)
        query_menu.addAction(act_explain)

        act_validate = QAction("✓ &Validate Syntax", self)
        act_validate.setShortcut("Ctrl+Shift+V")
        act_validate.triggered.connect(self.search_widget.validate_current_query)
        query_menu.addAction(act_validate)

        # Help Menu
        help_menu = menubar.addMenu("&Help")
        act_about = QAction("&About CQL Workbench", self)
        act_about.triggered.connect(self.show_about)
        help_menu.addAction(act_about)

    def show_about(self):
        QMessageBox.information(
            self,
            "About CQL Workbench",
            "<h3>CQL / Chess Search Engine Workbench</h3>"
            "<p>Interactive environment for testing, validating, and explaining high-performance "
            "Chess Query Language (CQL) patterns and queries across SCID databases (.si5, .si4) "
            "and PGN collections.</p>"
            "<p><b>Features:</b>"
            "<ul>"
            "<li>AST inspection and canonical query generation (<b>Explain Query</b>)</li>"
            "<li>Automatic symmetry expansion (flipcolor, flipvertical, etc.)</li>"
            "<li>High-speed multi-threaded scanning</li>"
            "<li>Direct board position preview and move navigation</li>"
            "</ul></p>"
        )

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
