import sys
import json
from PyQt5.QtGui import QFont
from PyQt5.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QPushButton, QTextEdit

class ProtocolLogPanelWidget(QWidget):
    """
    Protocol and backend JSON communication log viewer.
    Safely sanitizes large payloads to keep GUI event loop responsive.
    """
    def __init__(self, parent=None):
        super().__init__(parent)
        self.init_ui()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(6, 6, 6, 6)
        layout.setSpacing(6)

        actions = QHBoxLayout()
        btn_clear = QPushButton("Clear Logs")
        btn_clear.clicked.connect(self.clear_logs)
        actions.addWidget(btn_clear)
        actions.addStretch()
        layout.addLayout(actions)

        self.log_viewer = QTextEdit()
        self.log_viewer.setReadOnly(True)
        mono_font = QFont("Consolas" if sys.platform == "win32" else "Monospace", 10)
        self.log_viewer.setFont(mono_font)
        self.log_viewer.setLineWrapMode(QTextEdit.NoWrap)
        self.log_viewer.document().setMaximumBlockCount(1000)
        layout.addWidget(self.log_viewer)

    def clear_logs(self):
        self.log_viewer.clear()

    def append_message(self, message: str):
        self.log_viewer.append(message)

    def append_json_payload(self, data: dict):
        """
        Sanitize and summarize large payload lists before serializing to text
        to prevent blocking the Qt main thread.
        """
        try:
            log_payload = self._sanitize_payload(data)
            self.log_viewer.append(json.dumps(log_payload, indent=2))
        except Exception as e:
            self.log_viewer.append(f"[Log Formatting Error]: {e}")

    def _sanitize_payload(self, data: dict) -> dict:
        if not isinstance(data, dict):
            return data

        sanitized = dict(data)
        d_val = sanitized.get("data")
        if isinstance(d_val, dict):
            d_copy = dict(d_val)
            
            # Summarize games lists
            if "games" in d_copy and isinstance(d_copy["games"], list):
                count = len(d_copy["games"])
                if count > 5:
                    d_copy["games"] = f"[{count} games returned]"
            
            # Summarize moves lists in opening tree
            if "moves" in d_copy and isinstance(d_copy["moves"], list):
                count = len(d_copy["moves"])
                if count > 10:
                    d_copy["moves"] = f"[{count} candidate moves returned]"
            
            # Summarize sample game IDs
            if "sample_game_ids" in d_copy and isinstance(d_copy["sample_game_ids"], list):
                count = len(d_copy["sample_game_ids"])
                if count > 10:
                    d_copy["sample_game_ids"] = f"[{count} game IDs]"

            # Summarize sample games list
            if "sample_games" in d_copy and isinstance(d_copy["sample_games"], list):
                count = len(d_copy["sample_games"])
                if count > 5:
                    d_copy["sample_games"] = f"[{count} sample game summaries]"

            sanitized["data"] = d_copy
        return sanitized
