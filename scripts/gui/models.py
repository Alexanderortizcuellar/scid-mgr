from typing import Optional, Dict, Any, Set
from PyQt5.QtCore import Qt, QAbstractTableModel, QModelIndex, pyqtSignal
from PyQt5.QtGui import QColor
from .backend_client import BackendClient

class VirtualScidTableModel(QAbstractTableModel):
    """
    Pure passive virtual scrolling table model for SCID games.
    data() only reads from cache. It NEVER initiates network/pipe calls during rendering.
    Data chunks are fetched strictly when scrolling settles.
    """
    HEADERS = [
        "ID", "White", "W.Elo", "Black", "B.Elo", "Result", "ECO", "Date", "Event", "Site", "Round", "Status"
    ]
    COLUMN_SORT_FIELDS = {
        0: "id",
        1: "white",
        2: "white_elo",
        3: "black",
        4: "black_elo",
        5: "result",
        6: "eco",
        7: "date",
        8: "event",
        9: "site",
        10: "round",
    }
    CHUNK_SIZE = 100
    stats_updated = pyqtSignal(int, int)  # total_games, loaded_games

    def __init__(self, client: BackendClient, parent=None):
        super().__init__(parent)
        self.client = client
        self.total_count = 0
        self.filters: Dict[str, Any] = {}
        self.cached_chunks: Dict[int, list] = {}
        self.in_flight_pages: Set[int] = set()
        self.sort_col: Optional[int] = None
        self.sort_asc: bool = True

        self.client.response_received.connect(self.on_backend_response)

    def rowCount(self, parent=QModelIndex()) -> int:
        return self.total_count

    def columnCount(self, parent=QModelIndex()) -> int:
        return len(self.HEADERS)

    def headerData(self, section: int, orientation: Qt.Orientation, role=Qt.DisplayRole):
        if orientation == Qt.Horizontal and role == Qt.DisplayRole:
            title = self.HEADERS[section]
            if self.sort_col == section:
                title += " ▲" if self.sort_asc else " ▼"
            return title
        if orientation == Qt.Vertical and role == Qt.DisplayRole:
            return str(section + 1)
        return None

    def toggle_sort_column(self, col: int):
        if col not in self.COLUMN_SORT_FIELDS:
            return
        if self.sort_col == col:
            self.sort_asc = not self.sort_asc
        else:
            self.sort_col = col
            self.sort_asc = True

        self.filters["sort_by"] = self.COLUMN_SORT_FIELDS[col]
        self.filters["sort_asc"] = self.sort_asc
        self.headerDataChanged.emit(Qt.Horizontal, 0, len(self.HEADERS) - 1)
        self.invalidate_cache_and_reload()

    def data(self, index: QModelIndex, role=Qt.DisplayRole):
        if not index.isValid():
            return None

        row = index.row()
        col = index.column()
        page = row // self.CHUNK_SIZE
        offset_in_page = row % self.CHUNK_SIZE

        chunk = self.cached_chunks.get(page)
        game_item = chunk[offset_in_page] if (chunk and offset_in_page < len(chunk)) else None

        if role == Qt.DisplayRole:
            if game_item:
                return self._format_cell(game_item, col)
            return ""

        if role == Qt.ForegroundRole and game_item:
            if game_item.get("deleted"):
                return QColor("#d32f2f")  # Red for deleted games

        return None

    def _format_cell(self, g: dict, col: int) -> str:
        if col == 0:
            return str(g.get("id", ""))
        elif col == 1:
            return g.get("white", "")
        elif col == 2:
            elo = g.get("white_elo")
            return str(elo) if elo and elo > 0 else ""
        elif col == 3:
            return g.get("black", "")
        elif col == 4:
            elo = g.get("black_elo")
            return str(elo) if elo and elo > 0 else ""
        elif col == 5:
            return g.get("result", "")
        elif col == 6:
            return g.get("eco", "")
        elif col == 7:
            return g.get("date", "")
        elif col == 8:
            return g.get("event", "")
        elif col == 9:
            return g.get("site", "")
        elif col == 10:
            return g.get("round", "")
        elif col == 11:
            status_flags = []
            if g.get("deleted"):
                status_flags.append("DELETED")
            if g.get("non_standard_start"):
                status_flags.append("FEN")
            return " | ".join(status_flags) if status_flags else "OK"
        return ""

    def get_game_at(self, row: int) -> Optional[dict]:
        page = row // self.CHUNK_SIZE
        offset = row % self.CHUNK_SIZE
        chunk = self.cached_chunks.get(page)
        if chunk and offset < len(chunk):
            return chunk[offset]
        return None

    def request_chunks_for_range(self, top_row: int, bottom_row: int):
        """
        Triggered only when the user has settled on a visible viewport range.
        """
        if self.total_count == 0 or not self.client.is_running():
            return

        start_page = max(0, top_row // self.CHUNK_SIZE)
        end_page = min((self.total_count - 1) // self.CHUNK_SIZE, (bottom_row // self.CHUNK_SIZE) + 1)

        for page in range(start_page, end_page + 1):
            if page not in self.cached_chunks and page not in self.in_flight_pages:
                self._request_chunk(page)

    def _request_chunk(self, page: int):
        if page in self.in_flight_pages or not self.client.is_running():
            return
        self.in_flight_pages.add(page)
        params = dict(self.filters)
        params["page"] = page
        params["page_size"] = self.CHUNK_SIZE
        self.client.send_request("query_games", params)

    def set_filters(self, filters: dict):
        self.beginResetModel()
        self.filters = dict(filters)
        self.cached_chunks.clear()
        self.in_flight_pages.clear()
        self.total_count = 0
        self.endResetModel()

        if self.client.is_running():
            self._request_chunk(0)

    def invalidate_cache_and_reload(self):
        self.beginResetModel()
        self.cached_chunks.clear()
        self.in_flight_pages.clear()
        self.endResetModel()

        if self.client.is_running():
            self._request_chunk(0)

    def clear(self):
        self.beginResetModel()
        self.cached_chunks.clear()
        self.in_flight_pages.clear()
        self.total_count = 0
        self.endResetModel()
        self.stats_updated.emit(0, 0)

    def on_backend_response(self, data: dict):
        if data.get("status") != "ok":
            return
        resp_data = data.get("data", {})
        if "games" not in resp_data:
            return

        page = resp_data.get("page", 0)
        total = resp_data.get("total", 0)
        games = resp_data.get("games", [])

        if page in self.in_flight_pages:
            self.in_flight_pages.remove(page)

        self.cached_chunks[page] = games

        if total != self.total_count:
            self.beginResetModel()
            self.total_count = total
            self.endResetModel()
        else:
            start_row = page * self.CHUNK_SIZE
            end_row = min(self.total_count - 1, start_row + len(games) - 1)
            if start_row <= end_row:
                top_left = self.index(start_row, 0)
                bottom_right = self.index(end_row, len(self.HEADERS) - 1)
                self.dataChanged.emit(top_left, bottom_right, [Qt.DisplayRole, Qt.ForegroundRole])

        loaded_count = sum(len(c) for c in self.cached_chunks.values())
        self.stats_updated.emit(self.total_count, loaded_count)


