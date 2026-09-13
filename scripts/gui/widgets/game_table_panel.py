from typing import Optional, Dict, Any

from PyQt5.QtCore import Qt, pyqtSignal, QTimer, QSettings
from PyQt5.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QLabel,
    QPushButton, QTableView, QHeaderView, QMenu, QAction
)

from ..models import VirtualScidTableModel
from ..dialogs.columns_dialog import ColumnsConfigDialog

class GameTablePanelWidget(QWidget):
    """
    Virtual scrolling games table view with column configuration,
    sorting, and header context menus.
    """
    game_selected = pyqtSignal(int, dict)  # game_id, game_data
    scroll_settled = pyqtSignal(int, int)  # top_row, bottom_row

    def __init__(self, table_model: VirtualScidTableModel, parent=None):
        super().__init__(parent)
        self.table_model = table_model
        self.table_model.stats_updated.connect(self.on_model_stats_updated)

        # 150ms scroll debounce timer
        self.scroll_timer = QTimer(self)
        self.scroll_timer.setSingleShot(True)
        self.scroll_timer.setInterval(150)
        self.scroll_timer.timeout.connect(self._on_scroll_settled)

        self.init_ui()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(4)

        self.table_view = QTableView()
        self.table_view.setModel(self.table_model)
        self.table_view.setSelectionBehavior(QTableView.SelectRows)
        self.table_view.setSelectionMode(QTableView.SingleSelection)

        header = self.table_view.horizontalHeader()
        header.setSectionResizeMode(QHeaderView.Interactive)
        header.setSectionsClickable(True)
        header.setStretchLastSection(True)
        header.sectionClicked.connect(self.table_model.toggle_sort_column)
        header.setContextMenuPolicy(Qt.CustomContextMenu)
        header.customContextMenuRequested.connect(self.show_header_context_menu)

        self.table_view.verticalHeader().setDefaultSectionSize(26)
        self.table_view.selectionModel().selectionChanged.connect(self.on_table_selection_changed)
        
        sb = self.table_view.verticalScrollBar()
        sb.valueChanged.connect(self._on_scroll_changed)
        sb.sliderReleased.connect(self._on_scroll_settled)
        layout.addWidget(self.table_view)

        self.set_default_column_widths()
        self.load_column_settings()

        # Virtual Scroll Status Bar
        vscroll_bar = QHBoxLayout()
        self.lbl_vscroll_info = QLabel("Matching Games: 0 | Cached: 0 | ⚡ Virtual Scrolling Active")
        self.lbl_vscroll_info.setStyleSheet("color: #444; font-size: 11px; padding: 2px;")
        vscroll_bar.addWidget(self.lbl_vscroll_info)
        vscroll_bar.addStretch()

        btn_columns = QPushButton("⚙ Columns...")
        btn_columns.setStyleSheet("font-size: 11px; padding: 2px 8px;")
        btn_columns.setToolTip("Configure visible columns")
        btn_columns.clicked.connect(self.open_columns_dialog)
        vscroll_bar.addWidget(btn_columns)

        layout.addLayout(vscroll_bar)

    def set_default_column_widths(self):
        widths = [50, 140, 50, 140, 50, 65, 50, 80, 130, 110, 50, 60]
        for col, w in enumerate(widths):
            self.table_view.setColumnWidth(col, w)

    def show_header_context_menu(self, pos):
        menu = QMenu(self)
        menu.setStyleSheet("font-size: 12px;")

        title_action = menu.addAction("👁 Column Visibility:")
        title_action.setEnabled(False)
        menu.addSeparator()

        for col, name in enumerate(VirtualScidTableModel.HEADERS):
            act = QAction(f"{col + 1}. {name}", menu, checkable=True)
            act.setChecked(not self.table_view.isColumnHidden(col))
            act.setData(col)
            act.triggered.connect(lambda checked, c=col: self.toggle_column_visibility(c, checked))
            menu.addAction(act)

        menu.addSeparator()
        act_dialog = menu.addAction("⚙ Configure Columns...")
        act_dialog.triggered.connect(self.open_columns_dialog)

        act_show_all = menu.addAction("Show All Columns")
        act_show_all.triggered.connect(self.show_all_columns)

        act_reset_widths = menu.addAction("Reset Column Widths")
        act_reset_widths.triggered.connect(self.set_default_column_widths)

        menu.exec_(self.table_view.horizontalHeader().mapToGlobal(pos))

    def toggle_column_visibility(self, col: int, visible: bool):
        self.table_view.setColumnHidden(col, not visible)
        self.save_column_settings()

    def show_all_columns(self):
        for col in range(len(VirtualScidTableModel.HEADERS)):
            self.table_view.setColumnHidden(col, False)
        self.save_column_settings()

    def open_columns_dialog(self):
        dlg = ColumnsConfigDialog(self.table_view, VirtualScidTableModel.HEADERS, self)
        dlg.exec_()

    def save_column_settings(self):
        settings = QSettings("ChessScidMgr", "ScidGui")
        hidden_cols = [col for col in range(len(VirtualScidTableModel.HEADERS)) if self.table_view.isColumnHidden(col)]
        settings.setValue("columns_hidden", hidden_cols)

    def load_column_settings(self):
        settings = QSettings("ChessScidMgr", "ScidGui")
        hidden_cols = settings.value("columns_hidden", [])
        if isinstance(hidden_cols, list):
            for col in hidden_cols:
                try:
                    c = int(col)
                    if 0 <= c < len(VirtualScidTableModel.HEADERS):
                        self.table_view.setColumnHidden(c, True)
                except (ValueError, TypeError):
                    pass

    def on_model_stats_updated(self, total: int, loaded: int):
        self.lbl_vscroll_info.setText(
            f"Matching Games: {total:,} | Cached in Memory: {loaded:,} | ⚡ Scroll Debouncing Active"
        )

    def _on_scroll_changed(self, _val):
        self.scroll_timer.start()

    def _on_scroll_settled(self):
        top_row = self.table_view.rowAt(0)
        bottom_row = self.table_view.rowAt(self.table_view.viewport().height())

        if top_row == -1:
            top_row = 0
        if bottom_row == -1:
            bottom_row = min(self.table_model.total_count, top_row + 50)

        self.table_model.request_chunks_for_range(top_row, bottom_row)
        self.scroll_settled.emit(top_row, bottom_row)

    def on_table_selection_changed(self, selected, _deselected):
        indexes = selected.indexes()
        if not indexes:
            return
        row = indexes[0].row()
        game = self.table_model.get_game_at(row)
        if game:
            game_id = game.get("id", row)
            self.game_selected.emit(game_id, game)
