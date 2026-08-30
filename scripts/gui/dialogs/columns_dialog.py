from PyQt5.QtCore import Qt
from PyQt5.QtWidgets import (
    QDialog, QVBoxLayout, QLabel, QScrollArea, QWidget,
    QCheckBox, QHBoxLayout, QPushButton, QTableView
)

class ColumnsConfigDialog(QDialog):
    """Dialog allowing user to check/uncheck columns to display in the database table."""
    def __init__(self, table_view: QTableView, headers: list, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Configure Column Visibility")
        self.resize(360, 420)
        self.table_view = table_view
        self.headers = headers
        self.checkboxes = []

        layout = QVBoxLayout(self)

        info_lbl = QLabel("Check columns to display in the database table:")
        info_lbl.setStyleSheet("font-weight: bold; margin-bottom: 6px;")
        layout.addWidget(info_lbl)

        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll_widget = QWidget()
        scroll_layout = QVBoxLayout(scroll_widget)

        for col, name in enumerate(self.headers):
            cb = QCheckBox(f"{col + 1}. {name}")
            cb.setChecked(not self.table_view.isColumnHidden(col))
            self.checkboxes.append((col, cb))
            scroll_layout.addWidget(cb)

        scroll_layout.addStretch()
        scroll.setWidget(scroll_widget)
        layout.addWidget(scroll)

        btn_row1 = QHBoxLayout()
        btn_all = QPushButton("Select All")
        btn_all.clicked.connect(self.select_all)
        btn_row1.addWidget(btn_all)

        btn_reset = QPushButton("Reset Defaults")
        btn_reset.clicked.connect(self.reset_defaults)
        btn_row1.addWidget(btn_reset)
        layout.addLayout(btn_row1)

        btn_row2 = QHBoxLayout()
        btn_apply = QPushButton("Apply")
        btn_apply.setStyleSheet("font-weight: bold; background-color: #0288d1; color: white;")
        btn_apply.clicked.connect(self.apply_changes)
        btn_row2.addWidget(btn_apply)

        btn_close = QPushButton("Close")
        btn_close.clicked.connect(self.accept)
        btn_row2.addWidget(btn_close)
        layout.addLayout(btn_row2)

    def select_all(self):
        for _, cb in self.checkboxes:
            cb.setChecked(True)

    def reset_defaults(self):
        for _, cb in self.checkboxes:
            cb.setChecked(True)

    def apply_changes(self):
        for col, cb in self.checkboxes:
            self.table_view.setColumnHidden(col, not cb.isChecked())
        if self.parent() and hasattr(self.parent(), "save_column_settings"):
            self.parent().save_column_settings()


