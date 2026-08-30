#!/usr/bin/env python3
"""
PyQt5 GUI Client for SCID Chess Database Manager (chess-scid-rw backend).
Features:
- Pure on-stop virtual scrolling (150ms debounced lazy loading)
- Zero UI freeze during scrolling with non-blocking async IPC
- Full support for both SCID si4 and si5 database formats
- Search & multi-field filtering (Player, White, Black, ECO, Result, Date, Event, Site)
- Live game viewing (PGN reconstruction with Seven Tag Roster + variations + annotations)
- Database mutations: Add Game, Edit/Update Game, Delete/Undelete, Compact, Save
- PGN file Import & Export
- Real-time JSON-RPC protocol log inspector
"""

import os
import sys
import json
import queue
import subprocess
import threading
from typing import Optional, Dict, Any, Set

import chess
import chess.svg
import qtawesome as qta
from PyQt5.QtCore import Qt, pyqtSignal, QObject, QAbstractTableModel, QModelIndex, QTimer, QSize, QByteArray, QMimeData, QSettings
from PyQt5.QtGui import QFont, QColor, QPixmap, QPainter, QIcon, QDrag
from PyQt5.QtSvg import QSvgRenderer
from PyQt5.QtWidgets import (
    QApplication,
    QMainWindow,
    QWidget,
    QVBoxLayout,
    QHBoxLayout,
    QGridLayout,
    QSplitter,
    QLabel,
    QLineEdit,
    QPushButton,
    QComboBox,
    QTableView,
    QHeaderView,
    QTextEdit,
    QGroupBox,
    QFileDialog,
    QMessageBox,
    QTabWidget,
    QStatusBar,
    QFrame,
    QCheckBox,
    QDialog,
    QDialogButtonBox,
    QFormLayout,
    QRadioButton,
    QButtonGroup,
    QSpinBox,
    QTableWidgetItem,
    QProgressBar,
    QTableWidget,
    QMenu,
    QAction,
    QScrollArea,
)


class BackendClient(QObject):
    """
    Manages long-running Rust scid-mgr process communicating over stdin/stdout
    with a non-blocking asynchronous request queue.
    """
    response_received = pyqtSignal(dict)
    process_error = pyqtSignal(str)
    process_stopped = pyqtSignal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.process: Optional[subprocess.Popen] = None
        self.reader_thread: Optional[threading.Thread] = None
        self.writer_thread: Optional[threading.Thread] = None
        self.write_queue: queue.Queue = queue.Queue()
        self.running = False
        self.request_id = 0

    def is_running(self) -> bool:
        return self.process is not None and self.process.poll() is None

    def start(self, binary_path: str, db_path: Optional[str] = None):
        if self.is_running():
            self.stop()

        cmd = [binary_path, "--interactive"]
        if db_path and os.path.exists(db_path):
            cmd.append(db_path)

        try:
            self.process = subprocess.Popen(
                cmd,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                encoding="utf-8",
                bufsize=1,
                creationflags=subprocess.CREATE_NO_WINDOW if sys.platform == "win32" else 0,
            )
        except Exception as e:
            raise RuntimeError(f"Failed to spawn backend process: {e}")

        self.running = True
        self.write_queue = queue.Queue()

        # Background reader thread
        self.reader_thread = threading.Thread(target=self._read_stdout_loop, daemon=True)
        self.reader_thread.start()

        # Background writer thread
        self.writer_thread = threading.Thread(target=self._write_stdin_loop, daemon=True)
        self.writer_thread.start()

        # Monitor stderr
        threading.Thread(target=self._read_stderr_loop, daemon=True).start()

    def _read_stdout_loop(self):
        while self.running and self.process and self.process.stdout:
            line = self.process.stdout.readline()
            if not line:
                break
            line_str = line.strip()
            if not line_str:
                continue
            try:
                data = json.loads(line_str)
                self.response_received.emit(data)
            except json.JSONDecodeError as e:
                self.process_error.emit(f"Invalid JSON received: {line_str} ({e})")

        self.running = False
        self.process_stopped.emit()

    def _write_stdin_loop(self):
        while self.running:
            try:
                msg = self.write_queue.get(timeout=0.2)
            except queue.Empty:
                continue

            if msg is None:
                break

            if self.process and self.process.stdin:
                try:
                    self.process.stdin.write(msg)
                    self.process.stdin.flush()
                except Exception as e:
                    self.process_error.emit(f"Error writing to backend stdin: {e}")
                    break

    def _read_stderr_loop(self):
        while self.running and self.process and self.process.stderr:
            line = self.process.stderr.readline()
            if not line:
                break
            err_str = line.strip()
            if err_str:
                self.process_error.emit(f"[stderr] {err_str}")

    def send_request(self, command: str, params: Optional[dict] = None) -> int:
        if not self.is_running():
            raise RuntimeError("Backend process is not running.")

        self.request_id += 1
        req_id = self.request_id
        req_payload = {"id": req_id, "command": command}
        if params:
            req_payload.update(params)

        msg = json.dumps(req_payload) + "\n"
        self.write_queue.put(msg)
        return req_id

    def stop(self):
        if not self.is_running():
            return

        try:
            self.send_request("shutdown")
        except Exception:
            pass

        self.running = False
        self.write_queue.put(None)

        if self.process:
            try:
                if self.process.stdin:
                    self.process.stdin.close()
            except Exception:
                pass
            try:
                self.process.wait(timeout=1.0)
            except subprocess.TimeoutExpired:
                self.process.kill()
            self.process = None


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


def get_piece_pixmap(piece, size=40):
    """Generates a QPixmap from the python-chess SVG piece."""
    svg_string = chess.svg.piece(piece)
    renderer = QSvgRenderer(QByteArray(svg_string.encode("utf-8")))
    pixmap = QPixmap(size, size)
    pixmap.fill(Qt.transparent)

    painter = QPainter(pixmap)
    renderer.render(painter)
    painter.end()
    return pixmap


class SquareWidget(QLabel):
    """A custom label representing a single square on the chessboard."""
    def __init__(self, square_index, is_light, editor_parent):
        super().__init__()
        self.square_index = square_index
        self.editor_parent = editor_parent
        self.base_color = "#F0D9B5" if is_light else "#B58863"
        self.setFixedSize(42, 42)
        self.setAlignment(Qt.AlignCenter)
        self.setAcceptDrops(True)
        self.drag_start_pos = None
        self.update_background()

    def update_background(self, selected=False):
        color = "#99CC99" if selected else self.base_color
        self.setStyleSheet(f"background-color: {color}; border: 1px solid rgba(0,0,0,0.05);")

    def mousePressEvent(self, event):
        if event.button() == Qt.LeftButton:
            self.drag_start_pos = event.pos()
            self.editor_parent.square_clicked(self.square_index)

    def mouseMoveEvent(self, event):
        if not self.drag_start_pos or not (event.buttons() & Qt.LeftButton):
            return
        if (event.pos() - self.drag_start_pos).manhattanLength() < QApplication.startDragDistance():
            return

        active_btn = self.editor_parent.tool_group.checkedButton()
        if not active_btn or self.editor_parent.tools_map.get(active_btn) != "hand":
            return
        if not self.editor_parent.board.piece_at(self.square_index):
            return

        drag = QDrag(self)
        mime_data = QMimeData()
        mime_data.setText(str(self.square_index))
        drag.setMimeData(mime_data)
        if self.pixmap():
            drag.setPixmap(self.pixmap())
            drag.setHotSpot(self.pixmap().rect().center())
        self.clear()
        drag.exec_(Qt.MoveAction)
        self.editor_parent.update_board_ui()

    def dragEnterEvent(self, event):
        if event.mimeData().hasText():
            event.acceptProposedAction()

    def dropEvent(self, event):
        origin_idx = int(event.mimeData().text())
        self.editor_parent.handle_drop(origin_idx, self.square_index)
        event.acceptProposedAction()


class ChessBoardEditorWidget(QWidget):
    """
    Embedded Chess Board Editor with piece palette, click-to-place, drag-and-drop,
    and partial position support (e.g. single Queen on d4).
    """
    fen_changed = pyqtSignal(str)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.board = chess.Board()
        self.selected_square = None
        self.tool_group = QButtonGroup(self)
        self.tool_group.setExclusive(True)
        self.tools_map = {}
        self.square_widgets = {}

        self.init_ui()

    def init_ui(self):
        main_layout = QVBoxLayout(self)
        main_layout.setContentsMargins(0, 0, 0, 0)
        main_layout.setSpacing(4)

        # Black piece toolbar
        top_tb = self.create_toolbar(color=chess.BLACK)
        main_layout.addLayout(top_tb)

        # 8x8 Board grid
        board_layout = QGridLayout()
        board_layout.setSpacing(0)

        for rank in range(7, -1, -1):
            for file in range(8):
                sq_idx = chess.square(file, rank)
                is_light = (file + rank) % 2 != 0
                sq_w = SquareWidget(sq_idx, is_light, self)
                self.square_widgets[sq_idx] = sq_w
                board_layout.addWidget(sq_w, 7 - rank, file)

        b_container = QHBoxLayout()
        b_container.addStretch()
        b_container.addLayout(board_layout)
        b_container.addStretch()
        main_layout.addLayout(b_container)

        # White piece toolbar
        bot_tb = self.create_toolbar(color=chess.WHITE)
        main_layout.addLayout(bot_tb)

        self.update_board_ui()
        if self.tool_group.buttons():
            self.tool_group.buttons()[0].setChecked(True)

    def create_toolbar(self, color):
        layout = QHBoxLayout()
        layout.setAlignment(Qt.AlignCenter)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(2)

        # Hand / Move tool
        btn_hand = QPushButton()
        btn_hand.setIcon(qta.icon("fa5s.hand-pointer", color="#333" if color == chess.BLACK else "#1976d2"))
        btn_hand.setCheckable(True)
        btn_hand.setFixedSize(38, 38)
        btn_hand.setToolTip("Hand: Click or Drag to move pieces on the board")
        self.tool_group.addButton(btn_hand)
        self.tools_map[btn_hand] = "hand"
        layout.addWidget(btn_hand)

        # Piece buttons
        piece_types = [
            chess.KING,
            chess.QUEEN,
            chess.ROOK,
            chess.BISHOP,
            chess.KNIGHT,
            chess.PAWN,
        ]
        for pt in piece_types:
            piece = chess.Piece(pt, color)
            btn = QPushButton()
            btn.setCursor(Qt.PointingHandCursor)
            pixmap = get_piece_pixmap(piece, size=32)
            btn.setIcon(QIcon(pixmap))
            btn.setIconSize(QSize(32, 32))
            btn.setCheckable(True)
            btn.setFixedSize(38, 38)
            btn.setToolTip(f"Place {piece.symbol()} on square")
            self.tool_group.addButton(btn)
            self.tools_map[btn] = piece
            layout.addWidget(btn)

        # Trash tool
        btn_trash = QPushButton()
        btn_trash.setIcon(qta.icon("fa5s.trash-alt", color="#d32f2f"))
        btn_trash.setCheckable(True)
        btn_trash.setFixedSize(38, 38)
        btn_trash.setToolTip("Trash: Click squares to remove pieces")
        self.tool_group.addButton(btn_trash)
        self.tools_map[btn_trash] = "trash"
        layout.addWidget(btn_trash)

        return layout

    def square_clicked(self, square_index):
        active_btn = self.tool_group.checkedButton()
        if not active_btn:
            return
        tool = self.tools_map.get(active_btn)

        if tool == "trash":
            self.board.remove_piece_at(square_index)
            self.selected_square = None
        elif isinstance(tool, chess.Piece):
            self.board.set_piece_at(square_index, tool)
            self.selected_square = None
        elif tool == "hand":
            if self.selected_square is None:
                if self.board.piece_at(square_index):
                    self.selected_square = square_index
            else:
                if self.selected_square != square_index:
                    p = self.board.piece_at(self.selected_square)
                    if p:
                        self.board.set_piece_at(square_index, p)
                        self.board.remove_piece_at(self.selected_square)
                self.selected_square = None

        self.update_board_ui()

    def handle_drop(self, origin_idx, target_idx):
        if origin_idx != target_idx:
            p = self.board.piece_at(origin_idx)
            if p:
                self.board.set_piece_at(target_idx, p)
                self.board.remove_piece_at(origin_idx)
        self.selected_square = None
        self.update_board_ui()

    def update_board_ui(self):
        for sq_idx, w in self.square_widgets.items():
            p = self.board.piece_at(sq_idx)
            if p:
                w.setPixmap(get_piece_pixmap(p, size=36))
            else:
                w.clear()
            w.update_background(selected=(sq_idx == self.selected_square))

        fen_str = self.board.fen()
        self.fen_changed.emit(fen_str)

    def clear_board(self):
        self.board.clear()
        self.selected_square = None
        self.update_board_ui()

    def reset_to_initial(self):
        self.board.reset()
        self.selected_square = None
        self.update_board_ui()

    def set_fen(self, fen: str):
        try:
            self.board.set_fen(fen)
            self.selected_square = None
            self.update_board_ui()
        except Exception:
            pass

    def get_board_fen(self) -> str:
        return self.board.fen()


class AdvancedSearchDialog(QDialog):
    """
    ChessBase-style Advanced Multi-Tab Search Dialog.
    Tabs:
      1. 🏷️ Game Info (Players, Result, ECO, Date, Event, Site, Status)
      2. ♟️ Position / Board (Visual Board Editor, FEN string, Depth / Max Ply, Presets)
      3. ⚖️ Material (Piece counts for White & Black, Presets, Scope mode)
    """
    def __init__(self, current_filter: Optional[dict] = None, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Advanced Search (ChessBase style)")
        self.resize(800, 680)

        main_layout = QVBoxLayout(self)
        self.tabs = QTabWidget()
        main_layout.addWidget(self.tabs)

        # TAB 1: Game Info
        self.tab_info = QWidget()
        info_layout = QGridLayout(self.tab_info)
        info_layout.setContentsMargins(15, 15, 15, 15)
        info_layout.setSpacing(10)

        info_layout.addWidget(QLabel("Player (Any):"), 0, 0)
        self.in_player = QLineEdit()
        self.in_player.setPlaceholderText("e.g. Carlsen or Kasparov")
        info_layout.addWidget(self.in_player, 0, 1)

        info_layout.addWidget(QLabel("Result:"), 0, 2)
        self.in_result = QComboBox()
        self.in_result.addItems(["All", "1-0", "0-1", "1/2-1/2", "*"])
        info_layout.addWidget(self.in_result, 0, 3)

        info_layout.addWidget(QLabel("White:"), 1, 0)
        self.in_white = QLineEdit()
        info_layout.addWidget(self.in_white, 1, 1)

        info_layout.addWidget(QLabel("Black:"), 1, 2)
        self.in_black = QLineEdit()
        info_layout.addWidget(self.in_black, 1, 3)

        info_layout.addWidget(QLabel("ECO Code:"), 2, 0)
        self.in_eco = QLineEdit()
        self.in_eco.setPlaceholderText("e.g. B90 or E97")
        info_layout.addWidget(self.in_eco, 2, 1)

        info_layout.addWidget(QLabel("Date / Year:"), 2, 2)
        self.in_date = QLineEdit()
        self.in_date.setPlaceholderText("e.g. 2024 or 1999.01")
        info_layout.addWidget(self.in_date, 2, 3)

        info_layout.addWidget(QLabel("Event:"), 3, 0)
        self.in_event = QLineEdit()
        info_layout.addWidget(self.in_event, 3, 1)

        info_layout.addWidget(QLabel("Site:"), 3, 2)
        self.in_site = QLineEdit()
        info_layout.addWidget(self.in_site, 3, 3)

        del_box = QGroupBox("Status Filter")
        del_box_layout = QHBoxLayout(del_box)
        self.chk_include_del = QCheckBox("Include Deleted Games")
        self.chk_include_del.setChecked(True)
        del_box_layout.addWidget(self.chk_include_del)
        self.chk_only_del = QCheckBox("Only Deleted Games")
        del_box_layout.addWidget(self.chk_only_del)
        info_layout.addWidget(del_box, 4, 0, 1, 4)

        self.tabs.addTab(self.tab_info, "🏷️ Game Info")

        # TAB 2: Position (Board Editor & FEN)
        self.tab_pos = QWidget()
        pos_layout = QHBoxLayout(self.tab_pos)
        pos_layout.setContentsMargins(12, 12, 12, 12)
        pos_layout.setSpacing(15)

        # Left Column: Visual Chess Board Editor
        board_col = QVBoxLayout()
        self.board_editor = ChessBoardEditorWidget(self)
        self.board_editor.fen_changed.connect(self.on_board_fen_changed)
        board_col.addWidget(self.board_editor)

        board_btn_row = QHBoxLayout()
        btn_clear_b = QPushButton("Clear Board (Empty)")
        btn_clear_b.clicked.connect(self.board_editor.clear_board)
        board_btn_row.addWidget(btn_clear_b)

        btn_init_b = QPushButton("Initial Position")
        btn_init_b.clicked.connect(self.board_editor.reset_to_initial)
        board_btn_row.addWidget(btn_init_b)

        btn_qd4 = QPushButton("Queen on d4 (Demo)")
        btn_qd4.clicked.connect(lambda: self.set_single_piece_demo(chess.QUEEN, chess.WHITE, chess.D4))
        board_btn_row.addWidget(btn_qd4)

        board_col.addLayout(board_btn_row)
        pos_layout.addLayout(board_col, 0)

        # Right Column: Position Settings & Presets
        controls_col = QVBoxLayout()
        controls_col.setSpacing(10)

        controls_col.addWidget(QLabel("<b>Board FEN / Piece Placement:</b>"))
        self.in_fen = QLineEdit()
        self.in_fen.setPlaceholderText("e.g. 8/8/8/8/3Q4/8/8/8 or full FEN")
        self.in_fen.textChanged.connect(self.on_fen_text_edited)
        controls_col.addWidget(self.in_fen)

        depth_row = QHBoxLayout()
        depth_row.addWidget(QLabel("Max Search Ply (Depth):"))
        self.spin_max_ply = QSpinBox()
        self.spin_max_ply.setRange(1, 1000)
        self.spin_max_ply.setValue(250)
        depth_row.addWidget(self.spin_max_ply)
        depth_row.addStretch()
        controls_col.addLayout(depth_row)

        desc_lbl = QLabel(
            "💡 <i>Tip: Setup any piece pattern above (e.g. a single Queen on d4). "
            "The search engine will find all games where those pieces appear on those squares!</i>"
        )
        desc_lbl.setWordWrap(True)
        desc_lbl.setStyleSheet("color: #555; font-size: 11px; padding: 4px;")
        controls_col.addWidget(desc_lbl)

        preset_box = QGroupBox("Common Position Presets")
        preset_grid = QGridLayout(preset_box)

        btn_start_pos = QPushButton("Standard Start Pos")
        btn_start_pos.clicked.connect(lambda: self.board_editor.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"))
        preset_grid.addWidget(btn_start_pos, 0, 0)

        btn_najdorf = QPushButton("Sicilian Najdorf (6.Be2)")
        btn_najdorf.clicked.connect(lambda: self.board_editor.set_fen("rnbqkb1r/1p2pppp/p2p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6"))
        preset_grid.addWidget(btn_najdorf, 0, 1)

        btn_french = QPushButton("French Defense (3.Nc3)")
        btn_french.clicked.connect(lambda: self.board_editor.set_fen("rnbqkbnr/ppp2ppp/4p3/3p4/3PP3/2N5/PPP2PPP/R1BQKBNR b KQkq - 1 3"))
        preset_grid.addWidget(btn_french, 1, 0)

        btn_italian = QPushButton("Italian Game (Giocco Piano)")
        btn_italian.clicked.connect(lambda: self.board_editor.set_fen("r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4"))
        preset_grid.addWidget(btn_italian, 1, 1)

        btn_qgd = QPushButton("Queen's Gambit Declined")
        btn_qgd.clicked.connect(lambda: self.board_editor.set_fen("rnbqkb1r/ppp2ppp/4pn2/3p4/2PP4/2N5/PP2PPPP/R1BQKBNR w KQkq - 0 4"))
        preset_grid.addWidget(btn_qgd, 2, 0)

        btn_clear_fen = QPushButton("Clear FEN")
        btn_clear_fen.clicked.connect(lambda: self.board_editor.clear_board())
        preset_grid.addWidget(btn_clear_fen, 2, 1)

        controls_col.addWidget(preset_box)
        controls_col.addStretch()
        pos_layout.addLayout(controls_col, 1)

        self.tabs.addTab(self.tab_pos, "♟️ Position / Board")

        # TAB 3: Material Search
        self.tab_mat = QWidget()
        mat_layout = QVBoxLayout(self.tab_mat)
        mat_layout.setContentsMargins(15, 15, 15, 15)
        mat_layout.setSpacing(12)

        # Presets dropdown
        preset_row = QHBoxLayout()
        preset_row.addWidget(QLabel("Material Preset:"))
        self.combo_mat_preset = QComboBox()
        self.combo_mat_preset.addItems([
            "-- Custom Material --",
            "Rook Endgame (R+P vs R+P)",
            "Queen Endgame (Q+P vs Q+P)",
            "Minor Piece Endgame (B vs N)",
            "Queen vs Rook (Q vs R)",
            "Opposite-Colored Bishops (WB=1, BB=1)",
            "Queen Sacrifice / Queenless (WQ=0, BQ=1)",
            "Pawn Endgame (Pawns only)",
            "Reset All Pieces to Any",
        ])
        self.combo_mat_preset.currentIndexChanged.connect(self.on_material_preset_changed)
        preset_row.addWidget(self.combo_mat_preset, 1)
        mat_layout.addLayout(preset_row)

        # Piece Count Matrix
        grid_box = QGroupBox("Exact Piece Counts (Leave 'Any' for unconstrained)")
        grid = QGridLayout(grid_box)
        grid.setSpacing(8)

        pieces = [
            ("♕ Queen", "q"),
            ("♖ Rook", "r"),
            ("♗ Bishop", "b"),
            ("♘ Knight", "n"),
            ("♙ Pawn", "p"),
        ]

        grid.addWidget(QLabel("<b>Color</b>"), 0, 0)
        for col_idx, (pname, _) in enumerate(pieces, start=1):
            grid.addWidget(QLabel(f"<b>{pname}</b>"), 0, col_idx)

        grid.addWidget(QLabel("<b>White:</b>"), 1, 0)
        self.mat_white = {}
        for col_idx, (_, pkey) in enumerate(pieces, start=1):
            cb = QComboBox()
            cb.addItems(["Any"] + [str(i) for i in (range(9) if pkey == 'p' else range(3))])
            self.mat_white[pkey] = cb
            grid.addWidget(cb, 1, col_idx)

        grid.addWidget(QLabel("<b>Black:</b>"), 2, 0)
        self.mat_black = {}
        for col_idx, (_, pkey) in enumerate(pieces, start=1):
            cb = QComboBox()
            cb.addItems(["Any"] + [str(i) for i in (range(9) if pkey == 'p' else range(3))])
            self.mat_black[pkey] = cb
            grid.addWidget(cb, 2, col_idx)

        mat_layout.addWidget(grid_box)

        # Bishop Color Sub-options
        bish_box = QGroupBox("Bishop Color Verification")
        bish_layout = QHBoxLayout(bish_box)
        self.chk_opposite_bishops = QCheckBox("Opposite-Colored Bishops (White & Black on different color squares)")
        self.chk_same_bishops = QCheckBox("Same-Colored Bishops (White & Black on same color squares)")
        self.chk_opposite_bishops.toggled.connect(lambda on: on and self.chk_same_bishops.setChecked(False))
        self.chk_same_bishops.toggled.connect(lambda on: on and self.chk_opposite_bishops.setChecked(False))
        bish_layout.addWidget(self.chk_opposite_bishops)
        bish_layout.addWidget(self.chk_same_bishops)
        mat_layout.addWidget(bish_box)

        # Match Scope Box
        mode_box = QGroupBox("Search Scope")
        mode_layout = QVBoxLayout(mode_box)
        self.rb_final_pos = QRadioButton("Final Position only (Endgames - Ultra fast ~20ms)")
        self.rb_final_pos.setChecked(True)
        mode_layout.addWidget(self.rb_final_pos)
        self.rb_any_move = QRadioButton("Any Move during game (Middlegame / Sacrifices / Combinations ~150ms)")
        mode_layout.addWidget(self.rb_any_move)
        mat_layout.addWidget(mode_box)

        mat_layout.addStretch()
        self.tabs.addTab(self.tab_mat, "⚖️ Material")

        # Bottom Action Buttons
        btn_box = QHBoxLayout()
        btn_reset = QPushButton("🔄 Reset All Filters")
        btn_reset.clicked.connect(self.reset_all)
        btn_box.addWidget(btn_reset)
        btn_box.addStretch()

        btn_cancel = QPushButton("Cancel")
        btn_cancel.clicked.connect(self.reject)
        btn_box.addWidget(btn_cancel)

        btn_search = QPushButton("🔍 Search Games")
        btn_search.setStyleSheet("font-weight: bold; background-color: #1976d2; color: white; padding: 6px 20px;")
        btn_search.clicked.connect(self.accept)
        btn_box.addWidget(btn_search)

        main_layout.addLayout(btn_box)

        if current_filter:
            self.load_filter(current_filter)

    def on_material_preset_changed(self, idx: int):
        if idx == 0:
            return
        
        # Reset all to Any first
        for cb in self.mat_white.values(): cb.setCurrentIndex(0)
        for cb in self.mat_black.values(): cb.setCurrentIndex(0)
        self.chk_opposite_bishops.setChecked(False)
        self.chk_same_bishops.setChecked(False)
        self.rb_final_pos.setChecked(True)

        def set_val(side, pkey, val):
            d = self.mat_white if side == "w" else self.mat_black
            idx = d[pkey].findText(str(val))
            if idx >= 0: d[pkey].setCurrentIndex(idx)

        if idx == 1: # Rook Endgame (R+P vs R+P)
            set_val("w", "q", 0); set_val("w", "r", 1); set_val("w", "b", 0); set_val("w", "n", 0)
            set_val("b", "q", 0); set_val("b", "r", 1); set_val("b", "b", 0); set_val("b", "n", 0)
        elif idx == 2: # Queen Endgame (Q+P vs Q+P)
            set_val("w", "q", 1); set_val("w", "r", 0); set_val("w", "b", 0); set_val("w", "n", 0)
            set_val("b", "q", 1); set_val("b", "r", 0); set_val("b", "b", 0); set_val("b", "n", 0)
        elif idx == 3: # Minor Piece Endgame (B vs N)
            set_val("w", "q", 0); set_val("w", "r", 0); set_val("w", "b", 1); set_val("w", "n", 0)
            set_val("b", "q", 0); set_val("b", "r", 0); set_val("b", "b", 0); set_val("b", "n", 1)
        elif idx == 4: # Queen vs Rook
            set_val("w", "q", 1); set_val("w", "r", 0); set_val("w", "b", 0); set_val("w", "n", 0)
            set_val("b", "q", 0); set_val("b", "r", 1); set_val("b", "b", 0); set_val("b", "n", 0)
        elif idx == 5: # Opposite-Colored Bishops
            set_val("w", "q", 0); set_val("w", "r", 0); set_val("w", "b", 1); set_val("w", "n", 0)
            set_val("b", "q", 0); set_val("b", "r", 0); set_val("b", "b", 1); set_val("b", "n", 0)
            self.chk_opposite_bishops.setChecked(True)
        elif idx == 6: # Queen Sacrifice (WQ=0, BQ=1)
            set_val("w", "q", 0)
            set_val("b", "q", 1)
            self.rb_any_move.setChecked(True)
        elif idx == 7: # Pawn Endgame
            set_val("w", "q", 0); set_val("w", "r", 0); set_val("w", "b", 0); set_val("w", "n", 0)
            set_val("b", "q", 0); set_val("b", "r", 0); set_val("b", "b", 0); set_val("b", "n", 0)

    def on_board_fen_changed(self, fen: str):
        if self.in_fen.text().strip() != fen.strip():
            self.in_fen.blockSignals(True)
            self.in_fen.setText(fen)
            self.in_fen.blockSignals(False)

    def on_fen_text_edited(self, text: str):
        t = text.strip()
        if t:
            self.board_editor.blockSignals(True)
            self.board_editor.set_fen(t)
            self.board_editor.blockSignals(False)

    def set_single_piece_demo(self, role, color, square):
        self.board_editor.clear_board()
        self.board_editor.board.set_piece_at(square, chess.Piece(role, color))
        self.board_editor.update_board_ui()

    def reset_all(self):
        self.in_player.clear()
        self.in_white.clear()
        self.in_black.clear()
        self.in_result.setCurrentIndex(0)
        self.in_eco.clear()
        self.in_date.clear()
        self.in_event.clear()
        self.in_site.clear()
        self.chk_include_del.setChecked(True)
        self.chk_only_del.setChecked(False)
        self.board_editor.reset_to_initial()
        self.spin_max_ply.setValue(250)
        self.combo_mat_preset.setCurrentIndex(0)
        for cb in self.mat_white.values(): cb.setCurrentIndex(0)
        for cb in self.mat_black.values(): cb.setCurrentIndex(0)
        self.chk_opposite_bishops.setChecked(False)
        self.chk_same_bishops.setChecked(False)
        self.rb_final_pos.setChecked(True)

    def load_filter(self, f: dict):
        if "player" in f: self.in_player.setText(f["player"])
        if "white" in f: self.in_white.setText(f["white"])
        if "black" in f: self.in_black.setText(f["black"])
        if "result" in f:
            idx = self.in_result.findText(f["result"])
            if idx >= 0: self.in_result.setCurrentIndex(idx)
        if "eco" in f: self.in_eco.setText(f["eco"])
        if "date" in f: self.in_date.setText(f["date"])
        if "event" in f: self.in_event.setText(f["event"])
        if "site" in f: self.in_site.setText(f["site"])
        if "include_deleted" in f: self.chk_include_del.setChecked(f["include_deleted"])
        if "only_deleted" in f: self.chk_only_del.setChecked(f["only_deleted"])
        if "fen" in f: self.in_fen.setText(f["fen"])
        
        mat = f.get("material")
        if mat:
            mapping_w = {'white_queens': 'q', 'white_rooks': 'r', 'white_bishops': 'b', 'white_knights': 'n', 'white_pawns': 'p'}
            for f_key, pkey in mapping_w.items():
                if f_key in mat and mat[f_key] is not None:
                    idx = self.mat_white[pkey].findText(str(mat[f_key]))
                    if idx >= 0: self.mat_white[pkey].setCurrentIndex(idx)
            mapping_b = {'black_queens': 'q', 'black_rooks': 'r', 'black_bishops': 'b', 'black_knights': 'n', 'black_pawns': 'p'}
            for f_key, pkey in mapping_b.items():
                if f_key in mat and mat[f_key] is not None:
                    idx = self.mat_black[pkey].findText(str(mat[f_key]))
                    if idx >= 0: self.mat_black[pkey].setCurrentIndex(idx)
            if mat.get("opposite_bishops"):
                self.chk_opposite_bishops.setChecked(True)
            elif mat.get("same_bishops"):
                self.chk_same_bishops.setChecked(True)

            if mat.get("match_any_ply"):
                self.rb_any_move.setChecked(True)
            else:
                self.rb_final_pos.setChecked(True)

    def get_filter_dict(self) -> dict:
        f = {}
        p = self.in_player.text().strip()
        if p: f["player"] = p
        w = self.in_white.text().strip()
        if w: f["white"] = w
        b = self.in_black.text().strip()
        if b: f["black"] = b
        res = self.in_result.currentText()
        if res != "All": f["result"] = res
        eco = self.in_eco.text().strip()
        if eco: f["eco"] = eco
        dt = self.in_date.text().strip()
        if dt: f["date"] = dt
        ev = self.in_event.text().strip()
        if ev: f["event"] = ev
        st = self.in_site.text().strip()
        if st: f["site"] = st
        f["include_deleted"] = self.chk_include_del.isChecked()
        f["only_deleted"] = self.chk_only_del.isChecked()

        fen = self.in_fen.text().strip()
        if fen: f["fen"] = fen

        mat = {}
        def parse_val(cb):
            t = cb.currentText()
            return None if t == "Any" else int(t)

        mapping_w = {'q': 'white_queens', 'r': 'white_rooks', 'b': 'white_bishops', 'n': 'white_knights', 'p': 'white_pawns'}
        for pkey, f_key in mapping_w.items():
            v = parse_val(self.mat_white[pkey])
            if v is not None: mat[f_key] = v

        mapping_b = {'q': 'black_queens', 'r': 'black_rooks', 'b': 'black_bishops', 'n': 'black_knights', 'p': 'black_pawns'}
        for pkey, f_key in mapping_b.items():
            v = parse_val(self.mat_black[pkey])
            if v is not None: mat[f_key] = v

        if self.chk_opposite_bishops.isChecked():
            mat["opposite_bishops"] = True
        elif self.chk_same_bishops.isChecked():
            mat["same_bishops"] = True

        if mat:
            mat["match_any_ply"] = self.rb_any_move.isChecked()
            mat["max_ply"] = self.spin_max_ply.value()
            f["material"] = mat

        return f


class BenchmarkDialog(QDialog):
    def __init__(self, client: BackendClient, current_stats: Optional[dict] = None, parent=None):
        super().__init__(parent)
        self.setWindowTitle("📊 Database Performance Benchmark & Metrics")
        self.resize(920, 620)
        self.client = client
        self.current_stats = current_stats
        self.report_data = None
        self.init_ui()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(12, 12, 12, 12)
        layout.setSpacing(10)

        # Header info card
        info_frame = QFrame()
        info_frame.setFrameShape(QFrame.StyledPanel)
        info_frame.setStyleSheet("background-color: #f8f9fa; border: 1px solid #dee2e6; border-radius: 6px; padding: 6px;")
        info_layout = QHBoxLayout(info_frame)

        db_name = "None"
        fmt = "-"
        total = 0
        if self.current_stats:
            path = self.current_stats.get("path") or self.current_stats.get("index_path", "")
            db_name = os.path.basename(path)
            fmt = str(self.current_stats.get("format", "")).upper()
            total = self.current_stats.get("total_games", 0)

        self.lbl_db_info = QLabel(f"Database: {db_name} ({fmt}) | Total Games: {total:,}")
        self.lbl_db_info.setStyleSheet("font-weight: bold; font-size: 13px; color: #212529;")
        info_layout.addWidget(self.lbl_db_info)
        info_layout.addStretch()

        self.chk_heavy = QCheckBox("Deep Position Search")
        self.chk_heavy.setToolTip("Performs full position search across opening plies (recommended for databases < 500k games)")
        info_layout.addWidget(self.chk_heavy)

        self.btn_run = QPushButton("▶ Run Full Benchmark")
        self.btn_run.setStyleSheet("font-weight: bold; background-color: #2e7d32; color: white; padding: 6px 14px; border-radius: 4px;")
        self.btn_run.clicked.connect(self.run_benchmark)
        info_layout.addWidget(self.btn_run)

        layout.addWidget(info_frame)

        # Progress Bar
        self.progress_bar = QProgressBar()
        self.progress_bar.setRange(0, 0)
        self.progress_bar.setVisible(False)
        layout.addWidget(self.progress_bar)

        # Results Table
        self.table = QTableWidget()
        self.table.setColumnCount(5)
        self.table.setHorizontalHeaderLabels(["Category", "Benchmark Operation", "Time (ms)", "Count / Matches", "Details & Throughput"])
        self.table.horizontalHeader().setSectionResizeMode(0, QHeaderView.ResizeToContents)
        self.table.horizontalHeader().setSectionResizeMode(1, QHeaderView.Stretch)
        self.table.horizontalHeader().setSectionResizeMode(2, QHeaderView.ResizeToContents)
        self.table.horizontalHeader().setSectionResizeMode(3, QHeaderView.ResizeToContents)
        self.table.horizontalHeader().setSectionResizeMode(4, QHeaderView.Stretch)
        self.table.setAlternatingRowColors(True)
        self.table.setStyleSheet("QTableWidget { font-size: 11px; } QHeaderView::section { font-weight: bold; }")
        layout.addWidget(self.table)

        # Summary footer
        self.lbl_summary = QLabel("Ready to benchmark. Click 'Run Full Benchmark' to measure performance across all operations.")
        self.lbl_summary.setStyleSheet("font-style: italic; color: #555; font-size: 11px;")
        layout.addWidget(self.lbl_summary)

        # Actions row
        btn_box = QHBoxLayout()
        self.btn_copy = QPushButton("📋 Copy Report")
        self.btn_copy.clicked.connect(self.copy_report)
        self.btn_copy.setEnabled(False)
        btn_box.addWidget(self.btn_copy)
        btn_box.addStretch()

        btn_close = QPushButton("Close")
        btn_close.clicked.connect(self.accept)
        btn_box.addWidget(btn_close)
        layout.addLayout(btn_box)

    def run_benchmark(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Not Connected", "Please open a database first.")
            return
        self.btn_run.setEnabled(False)
        self.progress_bar.setVisible(True)
        self.lbl_summary.setText("Running comprehensive multi-threaded benchmarks... Please wait...")
        self.table.setRowCount(0)
        self.client.send_request("benchmark", {"heavy": self.chk_heavy.isChecked()})

    def display_report(self, report: dict):
        self.report_data = report
        self.btn_run.setEnabled(True)
        self.progress_bar.setVisible(False)
        self.btn_copy.setEnabled(True)

        total_games = report.get("total_games", 0)
        fmt = report.get("format", "")
        size_mb = report.get("file_size_mb", 0.0)
        db_path = report.get("db_path", "")
        db_name = os.path.basename(db_path)

        self.lbl_db_info.setText(f"Database: {db_name} ({fmt.upper()}) | Total Games: {total_games:,} | Size: {size_mb:.2f} MB")

        results = report.get("results", [])
        self.table.setRowCount(len(results))
        for row, item in enumerate(results):
            cat_text = item.get("category", "")
            cat = QTableWidgetItem(cat_text)
            name = QTableWidgetItem(item.get("name", ""))
            ms = item.get("elapsed_ms", 0.0)
            ms_item = QTableWidgetItem(f"{ms:,.2f} ms")
            ms_item.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)

            count = item.get("count", 0)
            count_item = QTableWidgetItem(f"{count:,}")
            count_item.setTextAlignment(Qt.AlignRight | Qt.AlignVCenter)

            notes = QTableWidgetItem(item.get("notes", ""))

            # Color code performance
            if "Sort" in cat_text or "Filter" in cat_text or "Index" in cat_text:
                if ms < 500:
                    ms_item.setForeground(QColor("#2e7d32")) # Green
                elif ms < 2000:
                    ms_item.setForeground(QColor("#e65100")) # Orange
                else:
                    ms_item.setForeground(QColor("#c62828")) # Red

            self.table.setItem(row, 0, cat)
            self.table.setItem(row, 1, name)
            self.table.setItem(row, 2, ms_item)
            self.table.setItem(row, 3, count_item)
            self.table.setItem(row, 4, notes)

        tot_ms = report.get("total_time_ms", 0.0)
        self.lbl_summary.setText(f"Completed {len(results)} benchmark operations in {tot_ms:,.2f} ms ({tot_ms/1000.0:.2f} s).")

    def copy_report(self):
        if not self.report_data:
            return
        lines = []
        lines.append("DATABASE PERFORMANCE BENCHMARK REPORT")
        lines.append("=" * 80)
        lines.append(f"Database:    {self.report_data.get('db_path')}")
        lines.append(f"Format:      {self.report_data.get('format')}")
        lines.append(f"Total Games: {self.report_data.get('total_games', 0):,}")
        lines.append(f"Disk Size:   {self.report_data.get('file_size_mb', 0.0):.2f} MB")
        lines.append("-" * 80)
        lines.append(f"{'Category':<22} | {'Operation':<42} | {'Time (ms)':>10} | {'Details'}")
        lines.append("-" * 80)
        for it in self.report_data.get("results", []):
            lines.append(f"{it.get('category',''):<22} | {it.get('name',''):<42} | {it.get('elapsed_ms',0.0):>10.2f} | {it.get('notes','')}")
        lines.append("=" * 80)
        lines.append(f"Total Benchmark Time: {self.report_data.get('total_time_ms',0.0):.2f} ms\n")
        QApplication.clipboard().setText("\n".join(lines))
        QMessageBox.information(self, "Copied", "Benchmark report copied to clipboard!")


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


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("SCID Chess Database Manager (chess-scid-rw)")
        self.resize(1340, 880)

        self.client = BackendClient(self)
        self.client.response_received.connect(self.on_response_received)
        self.client.process_error.connect(self.on_process_error)
        self.client.process_stopped.connect(self.on_process_stopped)

        self.table_model = VirtualScidTableModel(self.client, self)
        self.table_model.stats_updated.connect(self.on_model_stats_updated)

        self.selected_game_id: Optional[int] = None
        self.current_db_stats: Optional[dict] = None
        self.current_material_filter: Optional[dict] = None

        # 150ms scroll debounce timer
        self.scroll_timer = QTimer(self)
        self.scroll_timer.setSingleShot(True)
        self.scroll_timer.setInterval(150)
        self.scroll_timer.timeout.connect(self.on_scroll_settled)

        self.init_ui()
        self.auto_detect_defaults()

    def init_ui(self):
        main_widget = QWidget()
        self.setCentralWidget(main_widget)
        root_layout = QVBoxLayout(main_widget)
        root_layout.setContentsMargins(10, 10, 10, 10)
        root_layout.setSpacing(8)

        # 1. Connection & Database Management Group
        conn_group = QGroupBox("Backend Connection & SCID Database")
        conn_layout = QGridLayout(conn_group)
        conn_layout.setContentsMargins(10, 8, 10, 8)
        conn_layout.setSpacing(6)

        # Binary Path
        conn_layout.addWidget(QLabel("scid-mgr Binary:"), 0, 0)
        self.binary_input = QLineEdit()
        conn_layout.addWidget(self.binary_input, 0, 1)
        btn_browse_bin = QPushButton("Browse...")
        btn_browse_bin.clicked.connect(self.browse_binary)
        conn_layout.addWidget(btn_browse_bin, 0, 2)

        # Database Path (SCID or PGN)
        conn_layout.addWidget(QLabel("Chess DB / PGN:"), 1, 0)
        self.db_input = QLineEdit()
        self.db_input.setPlaceholderText("Select .si5, .si4, or .pgn file...")
        conn_layout.addWidget(self.db_input, 1, 1)
        btn_browse_db = QPushButton("Open DB / PGN...")
        btn_browse_db.clicked.connect(self.browse_db)
        conn_layout.addWidget(btn_browse_db, 1, 2)

        # SCID C++ Engine (Optional Legacy)
        conn_layout.addWidget(QLabel("SCID C++ (Optional):"), 2, 0)
        scid_cpp_row = QHBoxLayout()
        self.scid_cpp_input = QLineEdit()
        scid_cpp_row.addWidget(self.scid_cpp_input)
        self.chk_use_scid_cpp = QCheckBox("Use external SCID C++ binary instead of Native Rust (~1.2s)")
        self.chk_use_scid_cpp.setChecked(False)
        self.chk_use_scid_cpp.setStyleSheet("color: #666;")
        scid_cpp_row.addWidget(self.chk_use_scid_cpp)
        conn_layout.addLayout(scid_cpp_row, 2, 1)
        btn_browse_scid = QPushButton("Browse...")
        btn_browse_scid.clicked.connect(self.browse_scid_cpp)
        conn_layout.addWidget(btn_browse_scid, 2, 2)

        # Database action buttons row
        db_actions_layout = QHBoxLayout()
        self.btn_connect = QPushButton("Start Backend")
        self.btn_connect.setStyleSheet("font-weight: bold; background-color: #2e7d32; color: white; padding: 6px 12px;")
        self.btn_connect.clicked.connect(self.toggle_backend)
        db_actions_layout.addWidget(self.btn_connect)

        self.btn_new_db = QPushButton("New DB...")
        self.btn_new_db.clicked.connect(self.create_new_db)
        db_actions_layout.addWidget(self.btn_new_db)

        self.btn_import_pgn = QPushButton("Import PGN...")
        self.btn_import_pgn.clicked.connect(self.import_pgn)
        db_actions_layout.addWidget(self.btn_import_pgn)

        self.btn_export_pgn = QPushButton("Export PGN...")
        self.btn_export_pgn.clicked.connect(self.export_pgn)
        db_actions_layout.addWidget(self.btn_export_pgn)

        self.btn_compact = QPushButton("Compact DB")
        self.btn_compact.clicked.connect(self.compact_db)
        db_actions_layout.addWidget(self.btn_compact)

        self.btn_save = QPushButton("Save DB")
        self.btn_save.setStyleSheet("font-weight: bold; background-color: #0288d1; color: white; padding: 6px 12px;")
        self.btn_save.clicked.connect(self.save_db)
        db_actions_layout.addWidget(self.btn_save)

        self.btn_benchmark = QPushButton("📊 Metrics / Benchmark...")
        self.btn_benchmark.setStyleSheet("font-weight: bold; padding: 6px 12px;")
        self.btn_benchmark.clicked.connect(self.open_benchmark_dialog)
        db_actions_layout.addWidget(self.btn_benchmark)

        conn_layout.addLayout(db_actions_layout, 3, 0, 1, 3)
        root_layout.addWidget(conn_group)

        # 2. Database Stats Bar
        self.stats_bar = QFrame()
        self.stats_bar.setFrameShape(QFrame.StyledPanel)
        stats_layout = QHBoxLayout(self.stats_bar)
        stats_layout.setContentsMargins(10, 4, 10, 4)

        self.lbl_status = QLabel("Status: Disconnected")
        self.lbl_status.setStyleSheet("font-weight: bold; color: #d32f2f;")
        stats_layout.addWidget(self.lbl_status)

        stats_layout.addSpacing(15)
        self.lbl_format = QLabel("Format: -")
        stats_layout.addWidget(self.lbl_format)

        stats_layout.addSpacing(15)
        self.lbl_games_count = QLabel("Total Games: -")
        stats_layout.addWidget(self.lbl_games_count)

        stats_layout.addSpacing(15)
        self.lbl_active_count = QLabel("Active: -")
        stats_layout.addWidget(self.lbl_active_count)

        stats_layout.addSpacing(15)
        self.lbl_deleted_count = QLabel("Deleted: -")
        stats_layout.addWidget(self.lbl_deleted_count)

        stats_layout.addSpacing(15)
        self.lbl_players_count = QLabel("Players: -")
        stats_layout.addWidget(self.lbl_players_count)

        stats_layout.addSpacing(15)
        self.lbl_events_count = QLabel("Events: -")
        stats_layout.addWidget(self.lbl_events_count)

        stats_layout.addStretch()
        btn_refresh_info = QPushButton("Refresh Info")
        btn_refresh_info.clicked.connect(self.refresh_database_info)
        stats_layout.addWidget(btn_refresh_info)

        root_layout.addWidget(self.stats_bar)

        # 3. Filters Group
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
        root_layout.addWidget(filters_group)

        # 4. Main Splitter: Virtual Table on Left, Details & Logs on Right
        splitter = QSplitter(Qt.Horizontal)

        # Left Container: Virtual Games Table
        left_container = QWidget()
        left_layout = QVBoxLayout(left_container)
        left_layout.setContentsMargins(0, 0, 0, 0)

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
        self.table_view.verticalScrollBar().valueChanged.connect(self.on_scroll_changed)
        left_layout.addWidget(self.table_view)

        self._set_default_column_widths()
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

        left_layout.addLayout(vscroll_bar)
        splitter.addWidget(left_container)

        # Right Container: Tabs (Raw PGN Viewer + Metadata + JSON Protocol Log)
        self.tabs = QTabWidget()

        # PGN Tab
        pgn_widget = QWidget()
        pgn_layout = QVBoxLayout(pgn_widget)
        pgn_layout.setContentsMargins(6, 6, 6, 6)

        pgn_header_layout = QHBoxLayout()
        self.lbl_selected_game = QLabel("Selected Game: None")
        self.lbl_selected_game.setStyleSheet("font-weight: bold; font-size: 13px;")
        pgn_header_layout.addWidget(self.lbl_selected_game)
        pgn_header_layout.addStretch()

        self.btn_add_game = QPushButton("Add Game...")
        self.btn_add_game.clicked.connect(self.add_game_dialog)
        pgn_header_layout.addWidget(self.btn_add_game)

        self.btn_edit_game = QPushButton("Edit Game...")
        self.btn_edit_game.clicked.connect(self.edit_game_dialog)
        pgn_header_layout.addWidget(self.btn_edit_game)

        self.btn_del_game = QPushButton("Delete")
        self.btn_del_game.setStyleSheet("color: #d32f2f;")
        self.btn_del_game.clicked.connect(self.delete_selected_game)
        pgn_header_layout.addWidget(self.btn_del_game)

        self.btn_undel_game = QPushButton("Undelete")
        self.btn_undel_game.clicked.connect(self.undelete_selected_game)
        pgn_header_layout.addWidget(self.btn_undel_game)

        btn_copy_pgn = QPushButton("Copy PGN")
        btn_copy_pgn.clicked.connect(self.copy_pgn_text)
        pgn_header_layout.addWidget(btn_copy_pgn)

        pgn_layout.addLayout(pgn_header_layout)

        self.pgn_viewer = QTextEdit()
        self.pgn_viewer.setReadOnly(True)
        mono_font = QFont("Consolas" if sys.platform == "win32" else "Monospace", 10)
        self.pgn_viewer.setFont(mono_font)
        pgn_layout.addWidget(self.pgn_viewer)

        self.tabs.addTab(pgn_widget, "PGN Game Text")

        # JSON Logs Tab
        logs_widget = QWidget()
        logs_layout = QVBoxLayout(logs_widget)
        logs_layout.setContentsMargins(6, 6, 6, 6)

        log_actions = QHBoxLayout()
        btn_clear_logs = QPushButton("Clear Logs")
        btn_clear_logs.clicked.connect(self.clear_logs)
        log_actions.addWidget(btn_clear_logs)
        log_actions.addStretch()
        logs_layout.addLayout(log_actions)

        self.log_viewer = QTextEdit()
        self.log_viewer.setReadOnly(True)
        self.log_viewer.setFont(mono_font)
        logs_layout.addWidget(self.log_viewer)

        self.tabs.addTab(logs_widget, "Protocol Logs")

        splitter.addWidget(self.tabs)
        splitter.setSizes([800, 480])
        root_layout.addWidget(splitter, 1)

        # Status Bar
        self.status_bar = QStatusBar()
        self.setStatusBar(self.status_bar)
        self.status_bar.showMessage("Ready. Select SCID database and start backend.")

    def _set_default_column_widths(self):
        widths = [50, 140, 50, 140, 50, 65, 50, 80, 130, 110, 50, 60]
        for col, w in enumerate(widths):
            self.table_view.setColumnWidth(col, w)

    def show_header_context_menu(self, pos):
        menu = QMenu(self)
        menu.setStyleSheet("font-size: 12px;")

        # Title
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
        act_reset_widths.triggered.connect(self._set_default_column_widths)

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

    def auto_detect_defaults(self):
        project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        bin_names = ["scid-mgr.exe", "scid-mgr"]
        target_dirs = [
            os.path.join(project_root, "target", "release"),
            os.path.join(project_root, "target", "debug"),
        ]

        for t_dir in target_dirs:
            for b_name in bin_names:
                candidate = os.path.join(t_dir, b_name)
                if os.path.exists(candidate):
                    self.binary_input.setText(candidate)
                    break
            if self.binary_input.text():
                break

        # Auto-detect official SCID C++ engine
        downloads_scid = r"C:\Users\ASUS\Downloads\scid-v5.2.202603_windows_x64\scid_windows_x64\bin\scid.exe"
        scid_candidates = [
            downloads_scid,
            r"C:\Program Files\Scid\bin\scid.exe",
            r"C:\Program Files (x86)\Scid\bin\scid.exe",
        ]
        for scid_cand in scid_candidates:
            if os.path.exists(scid_cand):
                self.scid_cpp_input.setText(scid_cand)
                break

    def browse_scid_cpp(self):
        path, _ = QFileDialog.getOpenFileName(
            self, "Select Official SCID scid.exe Binary", "", "Executables (*.exe);;All Files (*)"
        )
        if path:
            self.scid_cpp_input.setText(path)

    def browse_binary(self):
        path, _ = QFileDialog.getOpenFileName(
            self, "Select scid-mgr Binary", "", "Executables (*.exe);;All Files (*)"
        )
        if path:
            self.binary_input.setText(path)

    def browse_db(self):
        path, _ = QFileDialog.getOpenFileName(
            self,
            "Select Chess Database or PGN File",
            "",
            "Chess Databases (*.si5 *.si4 *.pgn);;SCID Databases (*.si5 *.si4 *.sn5 *.sn4 *.sg5 *.sg4);;PGN Files (*.pgn);;All Files (*)",
        )
        if path:
            self.db_input.setText(path)
            if self.client.is_running():
                self.client.send_request("open", {"path": path})

    def create_new_db(self):
        dialog = NewDatabaseDialog(self)
        if dialog.exec_() == QDialog.Accepted:
            db_path, fmt = dialog.get_data()
            if not db_path:
                return
            self.db_input.setText(db_path)
            if self.client.is_running():
                self.client.send_request("create", {"path": db_path, "format": fmt})
            else:
                bin_path = self.binary_input.text().strip()
                if bin_path and os.path.exists(bin_path):
                    self.start_backend_and_open(db_path, create_format=fmt)

    def toggle_backend(self):
        if self.client.is_running():
            self.client.stop()
            self.update_ui_disconnected()
        else:
            bin_path = self.binary_input.text().strip()
            db_path = self.db_input.text().strip()
            if not bin_path or not os.path.exists(bin_path):
                QMessageBox.warning(self, "Invalid Binary", f"Cannot find binary at:\n{bin_path}")
                return

            try:
                self.client.start(bin_path, db_path if db_path and os.path.exists(db_path) else None)
                self.update_ui_connected()
                self.log_viewer.append(f"[GUI] Spawned backend: {bin_path}")
                if db_path:
                    self.client.send_request("open", {"path": db_path})
                else:
                    self.refresh_database_info()
            except Exception as e:
                QMessageBox.critical(self, "Startup Error", str(e))

    def start_backend_and_open(self, db_path: str, create_format: Optional[str] = None):
        bin_path = self.binary_input.text().strip()
        if not bin_path or not os.path.exists(bin_path):
            return
        self.client.start(bin_path)
        self.update_ui_connected()
        if create_format:
            self.client.send_request("create", {"path": db_path, "format": create_format})
        else:
            self.client.send_request("open", {"path": db_path})

    def update_ui_connected(self):
        self.lbl_status.setText("Status: Connected")
        self.lbl_status.setStyleSheet("font-weight: bold; color: #2e7d32;")
        self.btn_connect.setText("Stop Backend")
        self.btn_connect.setStyleSheet("font-weight: bold; background-color: #d32f2f; color: white; padding: 6px 12px;")

    def update_ui_disconnected(self):
        self.lbl_status.setText("Status: Disconnected")
        self.lbl_status.setStyleSheet("font-weight: bold; color: #d32f2f;")
        self.btn_connect.setText("Start Backend")
        self.btn_connect.setStyleSheet("font-weight: bold; background-color: #2e7d32; color: white; padding: 6px 12px;")
        self.table_model.clear()
        self.lbl_format.setText("Format: -")
        self.lbl_games_count.setText("Total Games: -")
        self.lbl_active_count.setText("Active: -")
        self.lbl_deleted_count.setText("Deleted: -")
        self.lbl_players_count.setText("Players: -")
        self.lbl_events_count.setText("Events: -")
        self.pgn_viewer.clear()
        self.lbl_selected_game.setText("Selected Game: None")

    def refresh_database_info(self):
        if self.client.is_running():
            self.client.send_request("info")

    def on_scroll_changed(self, _val):
        # Restart debounce timer on every scroll tick
        self.scroll_timer.start()

    def on_scroll_settled(self):
        top_row = self.table_view.rowAt(0)
        bottom_row = self.table_view.rowAt(self.table_view.viewport().height())

        if top_row == -1:
            top_row = 0
        if bottom_row == -1:
            bottom_row = min(self.table_model.total_count, top_row + 50)

        self.table_model.request_chunks_for_range(top_row, bottom_row)

    def open_advanced_search(self):
        current = {
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
            current["material"] = self.current_material_filter

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

            if self.table_model.sort_col is not None and self.table_model.sort_col in self.table_model.COLUMN_SORT_FIELDS:
                f["sort_by"] = self.table_model.COLUMN_SORT_FIELDS[self.table_model.sort_col]
                f["sort_asc"] = self.table_model.sort_asc
            self.table_model.set_filters(f)

    def on_search_clicked(self):
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

        if self.table_model.sort_col is not None and self.table_model.sort_col in self.table_model.COLUMN_SORT_FIELDS:
            filters["sort_by"] = self.table_model.COLUMN_SORT_FIELDS[self.table_model.sort_col]
            filters["sort_asc"] = self.table_model.sort_asc
        self.table_model.set_filters(filters)

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
        self.table_model.set_filters({})

    def on_table_selection_changed(self, selected, _deselected):
        indexes = selected.indexes()
        if not indexes:
            return
        row = indexes[0].row()
        game = self.table_model.get_game_at(row)
        if game:
            game_id = game.get("id", row)
            self.selected_game_id = game_id
            self.lbl_selected_game.setText(
                f"Selected Game #{game_id}: {game.get('white')} vs {game.get('black')} ({game.get('result')})"
            )
            if self.client.is_running():
                self.client.send_request("get_pgn", {"index": game_id})

    def add_game_dialog(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Start backend first.")
            return
        dialog = AddEditGameDialog("Add Game to SCID Database", parent=self)
        if dialog.exec_() == QDialog.Accepted:
            pgn = dialog.get_pgn()
            if pgn:
                self.client.send_request("add_game", {"pgn": pgn})

    def edit_game_dialog(self):
        if self.selected_game_id is None or not self.client.is_running():
            QMessageBox.warning(self, "No Selection", "Please select a game to edit.")
            return
        current_pgn = self.pgn_viewer.toPlainText()
        dialog = AddEditGameDialog(f"Edit Game #{self.selected_game_id}", initial_pgn=current_pgn, parent=self)
        if dialog.exec_() == QDialog.Accepted:
            pgn = dialog.get_pgn()
            if pgn:
                self.client.send_request("update_game", {"index": self.selected_game_id, "pgn": pgn})

    def delete_selected_game(self):
        if self.selected_game_id is None or not self.client.is_running():
            return
        self.client.send_request("delete_game", {"index": self.selected_game_id})

    def undelete_selected_game(self):
        if self.selected_game_id is None or not self.client.is_running():
            return
        self.client.send_request("undelete_game", {"index": self.selected_game_id})

    def compact_db(self):
        if not self.client.is_running():
            return
        self.client.send_request("compact")

    def save_db(self):
        if not self.client.is_running():
            return
        self.client.send_request("save")

    def import_pgn(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Start backend and open a database first.")
            return
        path, _ = QFileDialog.getOpenFileName(self, "Select PGN File to Import", "", "PGN Files (*.pgn);;All Files (*)")
        if path:
            file_size_mb = os.path.getsize(path) / (1024 * 1024)
            
            params = {"pgn_path": path}
            scid_exe = self.scid_cpp_input.text().strip()
            if self.chk_use_scid_cpp.isChecked() and scid_exe and os.path.exists(scid_exe):
                params["scid_exe"] = scid_exe
                self.status_bar.showMessage(f"Importing {os.path.basename(path)} with SCID C++ engine (~5s)...")
            else:
                self.status_bar.showMessage(f"Importing {os.path.basename(path)} ({file_size_mb:.1f} MB)...")
            
            # Setup Progress Dialog
            from PyQt5.QtWidgets import QProgressDialog
            self.import_progress_dialog = QProgressDialog(
                f"Importing {os.path.basename(path)}...\nStarting ingest engine...",
                "Cancel",
                0,
                100,
                self,
            )
            self.import_progress_dialog.setWindowTitle("Importing PGN Games")
            self.import_progress_dialog.setWindowModality(Qt.WindowModal)
            self.import_progress_dialog.setMinimumDuration(0)
            self.import_progress_dialog.setValue(0)
            self.import_progress_dialog.show()

            self.client.send_request("import_pgn", params)

    def export_pgn(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Start backend and open a database first.")
            return
        path, _ = QFileDialog.getSaveFileName(self, "Export Database to PGN", "export.pgn", "PGN Files (*.pgn);;All Files (*)")
        if path:
            self.status_bar.showMessage(f"Exporting to {path}...")
            from PyQt5.QtWidgets import QProgressDialog
            self.export_progress_dialog = QProgressDialog(
                f"Exporting {os.path.basename(path)}...\nFormatting PGN streams...",
                "Cancel",
                0,
                100,
                self,
            )
            self.export_progress_dialog.setWindowTitle("Exporting PGN Games")
            self.export_progress_dialog.setWindowModality(Qt.WindowModal)
            self.export_progress_dialog.setMinimumDuration(0)
            self.export_progress_dialog.setValue(0)
            self.export_progress_dialog.show()

            self.client.send_request("export_pgn", {"output_path": path})

    def copy_pgn_text(self):
        text = self.pgn_viewer.toPlainText()
        if text:
            QApplication.clipboard().setText(text)
            self.status_bar.showMessage("PGN copied to clipboard.", 3000)

    def clear_logs(self):
        self.log_viewer.clear()

    def on_model_stats_updated(self, total: int, loaded: int):
        self.lbl_vscroll_info.setText(
            f"Matching Games: {total:,} | Cached in Memory: {loaded:,} | ⚡ Scroll Debouncing Active"
        )

    def on_response_received(self, data: dict):
        # Handle async progress notification events
        if data.get("event") == "import_progress":
            prog = data.get("data", {})
            percent = int(prog.get("percent", 0))
            imported = prog.get("imported_games", 0)
            errors = prog.get("errors", 0)
            proc_mb = prog.get("processed_bytes", 0) / (1024 * 1024)
            tot_mb = prog.get("total_bytes", 0) / (1024 * 1024)
            speed = prog.get("speed_gps", 0.0)
            eta = prog.get("eta_seconds", 0)

            msg = (
                f"Importing PGN Games...\n"
                f"Progress: {percent}% ({proc_mb:.1f} / {tot_mb:.1f} MB)\n"
                f"Games Imported: {imported:,} (Errors: {errors})\n"
                f"Speed: {speed:,.0f} games/sec | ETA: {eta}s"
            )
            if hasattr(self, "import_progress_dialog") and self.import_progress_dialog:
                self.import_progress_dialog.setLabelText(msg)
                self.import_progress_dialog.setValue(percent)

            self.status_bar.showMessage(f"Importing: {percent}% | {imported:,} games ({speed:,.0f} g/s, ETA: {eta}s)")
            return

        if data.get("event") == "export_progress":
            prog = data.get("data", {})
            percent = int(prog.get("percent", 0))
            exported = prog.get("exported_games", 0)
            total = prog.get("total_games", 0)
            speed = prog.get("speed_gps", 0.0)
            eta = prog.get("eta_seconds", 0)

            msg = (
                f"Exporting PGN Games...\n"
                f"Progress: {percent}%\n"
                f"Games Exported: {exported:,} / {total:,}\n"
                f"Speed: {speed:,.0f} games/sec | ETA: {eta}s"
            )
            if hasattr(self, "export_progress_dialog") and self.export_progress_dialog:
                self.export_progress_dialog.setLabelText(msg)
                self.export_progress_dialog.setValue(percent)

            self.status_bar.showMessage(f"Exporting: {percent}% | {exported:,} games ({speed:,.0f} g/s, ETA: {eta}s)")
            return

        if data.get("event") == "search_progress":
            prog = data.get("data", {})
            scanned = prog.get("scanned", 0)
            total = prog.get("total", 0)
            matches = prog.get("matches", 0)
            pct = prog.get("percent", 0.0)
            self.status_bar.showMessage(f"🔍 Searching PGN: {scanned:,} / {total:,} games ({pct:.1f}%) — Found {matches:,} matches...")
            return

        # Log to tab
        self.log_viewer.append(json.dumps(data, indent=2))

        status = data.get("status")
        err = data.get("error")
        if status != "ok" and err:
            if hasattr(self, "import_progress_dialog") and self.import_progress_dialog:
                self.import_progress_dialog.close()
            if hasattr(self, "export_progress_dialog") and self.export_progress_dialog:
                self.export_progress_dialog.close()
            self.status_bar.showMessage(f"Error: {err}", 5000)
            return

        resp_data = data.get("data", {})

        # Handle stats updates
        if "stats" in resp_data:
            stats = resp_data["stats"]
            self.current_db_stats = stats
            fmt = stats.get("format", "").upper()
            self.lbl_format.setText(f"Format: {fmt}")
            self.lbl_games_count.setText(f"Total Games: {stats.get('total_games', 0):,}")
            self.lbl_active_count.setText(f"Active: {stats.get('active_games', 0):,}")
            self.lbl_deleted_count.setText(f"Deleted: {stats.get('deleted_games', 0):,}")
            self.lbl_players_count.setText(f"Players: {stats.get('players_count', 0):,}")
            self.lbl_events_count.setText(f"Events: {stats.get('events_count', 0):,}")

            # Reload model
            self.table_model.set_filters(self.table_model.filters)

        # Handle PGN response
        if "pgn" in resp_data:
            self.pgn_viewer.setPlainText(resp_data["pgn"])

        # Handle mutations
        if "reclaimed_bytes" in resp_data:
            reclaimed = resp_data["reclaimed_bytes"]
            self.status_bar.showMessage(f"Compaction completed. Reclaimed {reclaimed} bytes.", 4000)
            self.refresh_database_info()

        if "imported" in resp_data:
            if hasattr(self, "import_progress_dialog") and self.import_progress_dialog:
                self.import_progress_dialog.setValue(100)
                self.import_progress_dialog.close()

            imp = resp_data["imported"]
            err_count = resp_data.get("errors", 0)
            QMessageBox.information(
                self, "Import Complete", f"Imported {imp:,} games successfully ({err_count} errors)."
            )
            self.refresh_database_info()

        if "exported" in resp_data:
            if hasattr(self, "export_progress_dialog") and self.export_progress_dialog:
                self.export_progress_dialog.setValue(100)
                self.export_progress_dialog.close()
            exp = resp_data["exported"]
            QMessageBox.information(self, "Export Complete", f"Exported {exp:,} games to PGN successfully.")

        if "deleted" in resp_data or "index" in resp_data and "pgn" not in resp_data:
            self.refresh_database_info()

        # Handle benchmark report
        if "results" in resp_data and "total_time_ms" in resp_data:
            if hasattr(self, "benchmark_dialog") and self.benchmark_dialog:
                self.benchmark_dialog.display_report(resp_data)

    def open_benchmark_dialog(self):
        if not self.client.is_running():
            QMessageBox.warning(self, "Backend Offline", "Please start backend and open a database first.")
            return
        self.benchmark_dialog = BenchmarkDialog(self.client, current_stats=self.current_db_stats, parent=self)
        self.benchmark_dialog.show()

    def on_process_error(self, err_msg: str):
        self.log_viewer.append(f"[ERROR] {err_msg}")
        self.status_bar.showMessage(err_msg, 5000)

    def on_process_stopped(self):
        self.update_ui_disconnected()
        self.log_viewer.append("[GUI] Backend process stopped.")

    def closeEvent(self, event):
        self.client.stop()
        event.accept()


def main():
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec_())


if __name__ == "__main__":
    main()
