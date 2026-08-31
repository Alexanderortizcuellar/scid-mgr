import os
import sys
import json
import queue
import subprocess
import threading
from typing import Optional, Dict, Any

from PyQt5.QtCore import QObject, pyqtSignal

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

    def start(self, binary_path: str, db_path: Optional[str] = None, threads: Optional[int] = None):
        if self.is_running():
            self.stop()

        cmd = [binary_path, "--interactive"]
        if threads and threads > 0:
            cmd.extend(["--threads", str(threads)])
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


