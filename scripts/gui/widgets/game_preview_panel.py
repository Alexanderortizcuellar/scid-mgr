import sys
from typing import Optional

from PyQt5.QtCore import pyqtSignal
from PyQt5.QtGui import QFont
from PyQt5.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QLabel,
    QPushButton, QTextEdit, QApplication
)

class GamePreviewPanelWidget(QWidget):
    """
    Panel for viewing selected game PGN text, copy to clipboard,
    and game mutation actions (add, edit, delete, undelete).
    """
    add_game_requested = pyqtSignal()
    edit_game_requested = pyqtSignal()
    delete_game_requested = pyqtSignal()
    undelete_game_requested = pyqtSignal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.selected_game_id: Optional[int] = None
        self.init_ui()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(6, 6, 6, 6)
        layout.setSpacing(6)

        header_layout = QHBoxLayout()
        self.lbl_selected_game = QLabel("Selected Game: None")
        self.lbl_selected_game.setStyleSheet("font-weight: bold; font-size: 13px;")
        header_layout.addWidget(self.lbl_selected_game)
        header_layout.addStretch()

        self.btn_add_game = QPushButton("Add Game...")
        self.btn_add_game.clicked.connect(lambda: self.add_game_requested.emit())
        header_layout.addWidget(self.btn_add_game)

        self.btn_edit_game = QPushButton("Edit Game...")
        self.btn_edit_game.clicked.connect(lambda: self.edit_game_requested.emit())
        header_layout.addWidget(self.btn_edit_game)

        self.btn_del_game = QPushButton("Delete")
        self.btn_del_game.setStyleSheet("color: #d32f2f;")
        self.btn_del_game.clicked.connect(lambda: self.delete_game_requested.emit())
        header_layout.addWidget(self.btn_del_game)

        self.btn_undel_game = QPushButton("Undelete")
        self.btn_undel_game.clicked.connect(lambda: self.undelete_game_requested.emit())
        header_layout.addWidget(self.btn_undel_game)

        btn_copy_pgn = QPushButton("Copy PGN")
        btn_copy_pgn.clicked.connect(self.copy_pgn_text)
        header_layout.addWidget(btn_copy_pgn)

        layout.addLayout(header_layout)

        self.pgn_viewer = QTextEdit()
        self.pgn_viewer.setReadOnly(True)
        mono_font = QFont("Consolas" if sys.platform == "win32" else "Monospace", 10)
        self.pgn_viewer.setFont(mono_font)
        layout.addWidget(self.pgn_viewer)

    def set_selected_game(self, game_id: Optional[int], title: Optional[str] = None):
        self.selected_game_id = game_id
        if game_id is None:
            self.lbl_selected_game.setText("Selected Game: None")
            self.pgn_viewer.clear()
        else:
            if title:
                self.lbl_selected_game.setText(title)
            else:
                self.lbl_selected_game.setText(f"Selected Game #{game_id}")

    def set_pgn_text(self, pgn: str):
        self.pgn_viewer.setPlainText(pgn)

    def get_pgn_text(self) -> str:
        return self.pgn_viewer.toPlainText()

    def copy_pgn_text(self):
        text = self.pgn_viewer.toPlainText()
        if text:
            QApplication.clipboard().setText(text)

    def clear(self):
        self.selected_game_id = None
        self.lbl_selected_game.setText("Selected Game: None")
        self.pgn_viewer.clear()
