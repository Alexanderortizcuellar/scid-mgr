import sys
from typing import Optional
from PyQt5.QtWidgets import QDialog, QVBoxLayout, QLabel, QTextEdit, QDialogButtonBox
from PyQt5.QtGui import QFont

class AddEditGameDialog(QDialog):
    def __init__(self, title="Add Game to Database", initial_pgn="", parent=None):
        super().__init__(parent)
        self.setWindowTitle(title)
        self.resize(650, 480)
        layout = QVBoxLayout(self)

        lbl = QLabel("Enter or Paste Standard PGN text (Tags + Moves):")
        lbl.setStyleSheet("font-weight: bold;")
        layout.addWidget(lbl)

        self.pgn_edit = QTextEdit()
        mono_font = QFont("Consolas" if sys.platform == "win32" else "Monospace", 10)
        self.pgn_edit.setFont(mono_font)
        if initial_pgn:
            self.pgn_edit.setPlainText(initial_pgn)
        else:
            sample = (
                '[Event "Casual Game"]\n'
                '[Site "Local"]\n'
                '[Date "2026.01.01"]\n'
                '[Round "1"]\n'
                '[White "Player 1"]\n'
                '[Black "Player 2"]\n'
                '[Result "1-0"]\n'
                '[ECO "C50"]\n\n'
                '1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. O-O Nf6 5. d3 d6 1-0\n'
            )
            self.pgn_edit.setPlainText(sample)
        layout.addWidget(self.pgn_edit)

        buttons = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel)
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addWidget(buttons)

    def get_pgn(self) -> str:
        return self.pgn_edit.toPlainText().strip()


