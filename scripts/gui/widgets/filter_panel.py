from typing import Optional, Dict, Any

from PyQt5.QtCore import pyqtSignal
from PyQt5.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QGridLayout, QLabel,
    QLineEdit, QPushButton, QComboBox, QGroupBox, QCheckBox, QDialog
)

from ..dialogs.advanced_search_dialog import AdvancedSearchDialog

class FilterPanelWidget(QWidget):
    """
    Search and filter controls for querying games in the database.
    Supports quick header filters, FEN position search, and Advanced Search Dialog.
    """
    search_applied = pyqtSignal(dict)
    filters_cleared = pyqtSignal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.current_material_filter: Optional[dict] = None
        self.current_cql_filter: Optional[str] = None
        self.init_ui()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)

        filters_group = QGroupBox("Search & Filters")
        filters_layout = QGridLayout(filters_group)
        filters_layout.setContentsMargins(10, 8, 10, 8)
        filters_layout.setSpacing(6)

        # Player search
        filters_layout.addWidget(QLabel("Player (Any):"), 0, 0)
        self.filter_player = QLineEdit()
        self.filter_player.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_player, 0, 1)

        filters_layout.addWidget(QLabel("White:"), 0, 2)
        self.filter_white = QLineEdit()
        self.filter_white.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_white, 0, 3)

        filters_layout.addWidget(QLabel("Black:"), 0, 4)
        self.filter_black = QLineEdit()
        self.filter_black.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_black, 0, 5)

        # Result & ECO & Date
        filters_layout.addWidget(QLabel("Result:"), 1, 0)
        self.filter_result = QComboBox()
        self.filter_result.addItems(["All", "1-0", "0-1", "1/2-1/2", "*"])
        filters_layout.addWidget(self.filter_result, 1, 1)

        filters_layout.addWidget(QLabel("ECO Code:"), 1, 2)
        self.filter_eco = QLineEdit()
        self.filter_eco.setPlaceholderText("e.g. B85 or C")
        self.filter_eco.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_eco, 1, 3)

        filters_layout.addWidget(QLabel("Date:"), 1, 4)
        self.filter_date = QLineEdit()
        self.filter_date.setPlaceholderText("YYYY.MM.DD")
        self.filter_date.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_date, 1, 5)

        # Event & Site & Deleted flags
        filters_layout.addWidget(QLabel("Event:"), 2, 0)
        self.filter_event = QLineEdit()
        self.filter_event.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_event, 2, 1)

        filters_layout.addWidget(QLabel("Site:"), 2, 2)
        self.filter_site = QLineEdit()
        self.filter_site.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_site, 2, 3)

        flags_layout = QHBoxLayout()
        self.chk_include_deleted = QCheckBox("Include Deleted")
        self.chk_include_deleted.setChecked(True)
        flags_layout.addWidget(self.chk_include_deleted)

        self.chk_only_deleted = QCheckBox("Only Deleted")
        flags_layout.addWidget(self.chk_only_deleted)
        filters_layout.addLayout(flags_layout, 2, 4, 1, 2)

        # Position (FEN) Search Row
        filters_layout.addWidget(QLabel("Position (FEN):"), 3, 0)
        self.filter_fen = QLineEdit()
        self.filter_fen.setPlaceholderText("e.g. rnbqkb1r/1p2pppp/p2p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6")
        self.filter_fen.returnPressed.connect(self.on_search_clicked)
        filters_layout.addWidget(self.filter_fen, 3, 1, 1, 3)

        # Action buttons
        btn_search_layout = QHBoxLayout()
        self.btn_search = QPushButton("Apply Filters / Search")
        self.btn_search.setStyleSheet("font-weight: bold; background-color: #1976d2; color: white; padding: 5px 15px;")
        self.btn_search.clicked.connect(self.on_search_clicked)
        btn_search_layout.addWidget(self.btn_search)

        self.btn_adv_search = QPushButton("🔍 Advanced Search...")
        self.btn_adv_search.setStyleSheet("font-weight: bold; background-color: #6a1b9a; color: white; padding: 5px 12px;")
        self.btn_adv_search.clicked.connect(self.open_advanced_search)
        btn_search_layout.addWidget(self.btn_adv_search)

        btn_reset = QPushButton("Reset")
        btn_reset.clicked.connect(self.reset_filters)
        btn_search_layout.addWidget(btn_reset)

        filters_layout.addLayout(btn_search_layout, 3, 4, 1, 2)
        layout.addWidget(filters_group)

    def get_filter_dict(self) -> Dict[str, Any]:
        filters = {
            "player": self.filter_player.text().strip(),
            "white": self.filter_white.text().strip(),
            "black": self.filter_black.text().strip(),
            "result": self.filter_result.currentText(),
            "eco": self.filter_eco.text().strip(),
            "date": self.filter_date.text().strip(),
            "event": self.filter_event.text().strip(),
            "site": self.filter_site.text().strip(),
            "include_deleted": self.chk_include_deleted.isChecked(),
            "only_deleted": self.chk_only_deleted.isChecked(),
            "fen": self.filter_fen.text().strip(),
        }
        if self.current_material_filter:
            filters["material"] = self.current_material_filter
        if self.current_cql_filter:
            filters["cql"] = self.current_cql_filter
        return filters

    def on_search_clicked(self):
        self.search_applied.emit(self.get_filter_dict())

    def reset_filters(self):
        self.filter_player.clear()
        self.filter_white.clear()
        self.filter_black.clear()
        self.filter_result.setCurrentIndex(0)
        self.filter_eco.clear()
        self.filter_date.clear()
        self.filter_event.clear()
        self.filter_site.clear()
        self.filter_fen.clear()
        self.chk_include_deleted.setChecked(True)
        self.chk_only_deleted.setChecked(False)
        self.current_material_filter = None
        self.current_cql_filter = None
        self.filters_cleared.emit()

    def open_advanced_search(self):
        current = self.get_filter_dict()
        dlg = AdvancedSearchDialog(current_filter=current, parent=self)
        if dlg.exec_() == QDialog.Accepted:
            f = dlg.get_filter_dict()
            # Sync quick fields
            self.filter_player.setText(f.get("player", ""))
            self.filter_white.setText(f.get("white", ""))
            self.filter_black.setText(f.get("black", ""))
            res_idx = self.filter_result.findText(f.get("result", "All"))
            if res_idx >= 0:
                self.filter_result.setCurrentIndex(res_idx)
            self.filter_eco.setText(f.get("eco", ""))
            self.filter_date.setText(f.get("date", ""))
            self.filter_event.setText(f.get("event", ""))
            self.filter_site.setText(f.get("site", ""))
            self.chk_include_deleted.setChecked(f.get("include_deleted", True))
            self.chk_only_deleted.setChecked(f.get("only_deleted", False))
            self.filter_fen.setText(f.get("fen", ""))

            self.current_material_filter = f.get("material")
            self.current_cql_filter = f.get("cql")

            self.search_applied.emit(self.get_filter_dict())
