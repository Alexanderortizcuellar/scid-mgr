from typing import Optional
from PyQt5.QtWidgets import (
    QDialog, QVBoxLayout, QFormLayout, QLineEdit, QPushButton,
    QHBoxLayout, QRadioButton, QButtonGroup, QDialogButtonBox, QFileDialog
)

class NewDatabaseDialog(QDialog):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Create New SCID Database")
        self.resize(460, 180)
        layout = QVBoxLayout(self)

        form = QFormLayout()
        self.path_input = QLineEdit()
        btn_browse = QPushButton("Browse...")
        btn_browse.clicked.connect(self.browse_path)

        path_row = QHBoxLayout()
        path_row.addWidget(self.path_input)
        path_row.addWidget(btn_browse)
        form.addRow("Database Path:", path_row)

        self.rb_si5 = QRadioButton("SCID 5 format (.si5) - Modern 64-bit / 140 TB capacity (Recommended)")
        self.rb_si5.setChecked(True)
        self.rb_si4 = QRadioButton("SCID 4 format (.si4) - Legacy 32-bit format")

        self.format_group = QButtonGroup(self)
        self.format_group.addButton(self.rb_si5, 5)
        self.format_group.addButton(self.rb_si4, 4)

        fmt_box = QVBoxLayout()
        fmt_box.addWidget(self.rb_si5)
        fmt_box.addWidget(self.rb_si4)
        form.addRow("Format:", fmt_box)

        layout.addLayout(form)

        buttons = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel)
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addWidget(buttons)

    def browse_path(self):
        ext = ".si5" if self.rb_si5.isChecked() else ".si4"
        path, _ = QFileDialog.getSaveFileName(
            self,
            "Create Database",
            f"database{ext}",
            "SCID 5 Files (*.si5);;SCID 4 Files (*.si4);;All Files (*)",
        )
        if path:
            self.path_input.setText(path)

    def get_data(self):
        return (
            self.path_input.text().strip(),
            "si5" if self.rb_si5.isChecked() else "si4",
        )


