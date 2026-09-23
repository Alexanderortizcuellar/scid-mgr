#!/usr/bin/env python3
"""
Codebase Indexer for scid-mgr.
Scans Rust (src/) and Python (scripts/) files to extract all:
- Structs, Enums, Traits, Functions, Impl blocks (Rust)
- Classes, Methods, Functions (Python)
- JSON-RPC Server Commands (Rust & Python client)
- Generates a comprehensive, searchable CODEBASE_INDEX.md
"""

import os
import re
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent
SRC_DIR = ROOT_DIR / "src"
SCRIPTS_DIR = ROOT_DIR / "scripts"
OUTPUT_MD = ROOT_DIR / "CODEBASE_INDEX.md"

def scan_rust_file(filepath: Path):
    symbols = []
    with open(filepath, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()

    current_impl = None
    for idx, line in enumerate(lines, 1):
        stripped = line.strip()
        
        # Match impl blocks
        impl_match = re.match(r"^impl(?:\s*<[^>]+>)?\s+(?:(\w+(?:\s*<[^>]+>)?)\s+for\s+)?(\w+)", stripped)
        if impl_match:
            trait_name = impl_match.group(1)
            struct_name = impl_match.group(2)
            current_impl = f"{struct_name} (impl {trait_name})" if trait_name else struct_name

        if stripped.startswith("}") and current_impl and line.startswith("}"):
            current_impl = None

        # Match structs
        struct_match = re.match(r"^(?:pub(?:\([^\)]+\))?\s+)?struct\s+(\w+)", stripped)
        if struct_match:
            symbols.append({"type": "struct", "name": struct_match.group(1), "line": idx, "context": stripped})

        # Match enums
        enum_match = re.match(r"^(?:pub(?:\([^\)]+\))?\s+)?enum\s+(\w+)", stripped)
        if enum_match:
            symbols.append({"type": "enum", "name": enum_match.group(1), "line": idx, "context": stripped})

        # Match traits
        trait_match = re.match(r"^(?:pub(?:\([^\)]+\))?\s+)?trait\s+(\w+)", stripped)
        if trait_match:
            symbols.append({"type": "trait", "name": trait_match.group(1), "line": idx, "context": stripped})

        # Match functions / methods
        fn_match = re.match(r"^(?:pub(?:\([^\)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)", stripped)
        if fn_match:
            fn_name = fn_match.group(1)
            full_name = f"{current_impl}::{fn_name}" if current_impl else fn_name
            symbols.append({"type": "fn", "name": full_name, "line": idx, "context": stripped})

    return symbols

def scan_python_file(filepath: Path):
    symbols = []
    with open(filepath, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()

    current_class = None
    for idx, line in enumerate(lines, 1):
        stripped = line.strip()

        class_match = re.match(r"^class\s+(\w+)(?:\([^\)]*\))?:", stripped)
        if class_match:
            current_class = class_match.group(1)
            symbols.append({"type": "class", "name": current_class, "line": idx, "context": stripped})

        def_match = re.match(r"^def\s+(\w+)\s*\(", stripped)
        if def_match:
            symbols.append({"type": "func", "name": def_match.group(1), "line": idx, "context": stripped})
            current_class = None

        method_match = re.match(r"^\s{4}def\s+(\w+)\s*\(", line)
        if method_match and current_class:
            symbols.append({"type": "method", "name": f"{current_class}.{method_match.group(1)}", "line": idx, "context": stripped})

    return symbols

def scan_server_commands(server_file: Path):
    commands = []
    if not server_file.exists():
        return commands
    with open(server_file, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()
    for idx, line in enumerate(lines, 1):
        m = re.search(r'"([a-z_0-9]+)"\s*=>\s*\{', line)
        if m:
            cmd = m.group(1)
            commands.append({"cmd": cmd, "line": idx})
    return commands

def build_index():
    index_content = []
    index_content.append("# scid-mgr Codebase Symbol & Architecture Index\n")
    index_content.append("> Auto-generated index mapping modules, files, classes, structs, traits, functions, and IPC commands.\n")
    index_content.append("> To update this index at any time, run: `python scripts/index_codebase.py`\n\n")

    index_content.append("## Table of Contents\n")
    index_content.append("- [1. Architectural Overview](#1-architectural-overview)\n")
    index_content.append("- [2. JSON-RPC Server IPC Commands](#2-json-rpc-server-ipc-commands)\n")
    index_content.append("- [3. Rust Backend (`src/`)](#3-rust-backend-src)\n")
    index_content.append("- [4. Python GUI Frontend (`scripts/gui/`)](#4-python-gui-frontend-scriptsgui)\n")
    index_content.append("- [5. Alphabetical Symbol Quick-Find](#5-alphabetical-symbol-quick-find)\n\n")

    index_content.append("## 1. Architectural Overview\n\n")
    index_content.append("| Layer | Module / Directory | Key Responsibilities |\n")
    index_content.append("|---|---|---|\n")
    index_content.append("| **Backend Core** | `src/db.rs` | SCID `.si4`/`.si5` database wrapper, record reading, in-place sorting, editing, headers |\n")
    index_content.append("| **Backend Core** | `src/pgn_db.rs` | Fast memory-mapped / indexed PGN wrapper with string deduplication tables |\n")
    index_content.append("| **Backend Core** | `src/pgn_utils.rs` | Streaming PGN parser, chunk-based PGN importer with progress callbacks |\n")
    index_content.append("| **Indexing Engine** | `src/position_index.rs` | Inverted position index (`.pos.idx`) with Delta-Varint posting lists for instant candidate filtering |\n")
    index_content.append("| **Indexing Engine** | `src/tree_index.rs` | Opening tree index (`.tree.idx`) calculating move frequencies, win/draw/loss stats, ECO trees |\n")
    index_content.append("| **Indexing Engine** | `src/zero_copy_ingest.rs` | High-speed zero-copy stream ingestion for massive PGN datasets directly into indices |\n")
    index_content.append("| **Search (CQLite)** | `src/search/` | Complete AST query parser, semantic validator, tacticals, piece paths, square sets, FEN transformations |\n")
    index_content.append("| **IPC Server** | `src/server.rs` | JSON-RPC server on stdin/stdout connecting Python GUI and Rust engine with multithreading |\n")
    index_content.append("| **CLI Binary** | `src/main.rs` | CLI commands (`info`, `list`, `search`, `tree`, `index`, `sort`, `import`, `bench`) |\n")
    index_content.append("| **GUI Frontend** | `scripts/gui/main_window.py` | PySide6 / PyQt application window, layout, and event wiring |\n")
    index_content.append("| **GUI Client** | `scripts/gui/backend_client.py` | Subprocess IPC client communicating asynchronously with `scid-mgr --interactive` |\n")
    index_content.append("| **GUI Widgets** | `scripts/gui/widgets/` | Chessboard, opening tree explorer, CQLite search query visualizer, filter panel, game list |\n\n")

    # Server commands
    server_file = SRC_DIR / "server.rs"
    cmds = scan_server_commands(server_file)
    index_content.append("## 2. JSON-RPC Server IPC Commands\n\n")
    index_content.append(f"Defined in [`src/server.rs`](file:///{server_file.as_posix()}):\n\n")
    index_content.append("| Command | Line | Description |\n")
    index_content.append("|---|---|---|\n")
    for c in cmds:
        index_content.append(f"| `{c['cmd']}` | [`L{c['line']}`](file:///{server_file.as_posix()}#L{c['line']}) | Request handler for `{c['cmd']}` |\n")
    index_content.append("\n")

    # Scan Rust files
    index_content.append("## 3. Rust Backend (`src/`)\n\n")
    all_rust_symbols = []
    for root, _, files in sorted(os.walk(SRC_DIR)):
        for f in sorted(files):
            if f.endswith(".rs"):
                fp = Path(root) / f
                rel_path = fp.relative_to(ROOT_DIR).as_posix()
                symbols = scan_rust_file(fp)
                for s in symbols:
                    s["file"] = rel_path
                    s["abs_file"] = fp.as_posix()
                all_rust_symbols.extend(symbols)

                index_content.append(f"### [`{rel_path}`](file:///{fp.as_posix()})\n\n")
                if not symbols:
                    index_content.append("_No public structs or functions found._\n\n")
                    continue

                index_content.append("| Kind | Name | Line |\n")
                index_content.append("|---|---|---|\n")
                for s in symbols:
                    index_content.append(f"| `{s['type']}` | `{s['name']}` | [`L{s['line']}`](file:///{fp.as_posix()}#L{s['line']}) |\n")
                index_content.append("\n")

    # Scan Python files
    index_content.append("## 4. Python GUI Frontend (`scripts/gui/`)\n\n")
    all_py_symbols = []
    for root, _, files in sorted(os.walk(SCRIPTS_DIR)):
        for f in sorted(files):
            if f.endswith(".py"):
                fp = Path(root) / f
                rel_path = fp.relative_to(ROOT_DIR).as_posix()
                symbols = scan_python_file(fp)
                for s in symbols:
                    s["file"] = rel_path
                    s["abs_file"] = fp.as_posix()
                all_py_symbols.extend(symbols)

                index_content.append(f"### [`{rel_path}`](file:///{fp.as_posix()})\n\n")
                if not symbols:
                    index_content.append("_No classes or functions found._\n\n")
                    continue

                index_content.append("| Kind | Name | Line |\n")
                index_content.append("|---|---|---|\n")
                for s in symbols:
                    index_content.append(f"| `{s['type']}` | `{s['name']}` | [`L{s['line']}`](file:///{fp.as_posix()}#L{s['line']}) |\n")
                index_content.append("\n")

    # Alphabetical Symbol Index
    index_content.append("## 5. Alphabetical Symbol Quick-Find\n\n")
    all_symbols = sorted(all_rust_symbols + all_py_symbols, key=lambda x: x["name"].lower())
    index_content.append("| Symbol Name | Kind | File | Line |\n")
    index_content.append("|---|---|---|---|\n")
    for s in all_symbols:
        index_content.append(f"| `{s['name']}` | `{s['type']}` | [`{s['file']}`](file:///{s['abs_file']}#L{s['line']}) | L{s['line']} |\n")
    index_content.append("\n")

    with open(OUTPUT_MD, "w", encoding="utf-8") as out:
        out.write("".join(index_content))

    print(f"Successfully generated {OUTPUT_MD} with {len(all_symbols)} indexed symbols across Rust and Python!")

if __name__ == "__main__":
    build_index()
