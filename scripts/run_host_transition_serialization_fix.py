#!/usr/bin/env python3
"""Anchor the one-shot host transition patch, then execute it."""

from __future__ import annotations

import runpy
from pathlib import Path

path = Path("scripts/apply_host_transition_serialization_fix.py")
text = path.read_text()
old = '''replace_once(
    "src-tauri/src/networking/runtime/host.rs",
    """            let authoritative_state = Arc::clone(&authoritative_state);
            let tournament_runtime = Arc::clone(&tournament_runtime);
            let clients = Arc::clone(&clients);
""",
    """            let authoritative_state = Arc::clone(&authoritative_state);
            let tournament_runtime = Arc::clone(&tournament_runtime);
            let transition_lock = Arc::clone(&transition_lock);
            let clients = Arc::clone(&clients);
""",
)
'''
new = '''replace_once(
    "src-tauri/src/networking/runtime/host.rs",
    """        let accept_thread = {
            let authoritative_state = Arc::clone(&authoritative_state);
            let tournament_runtime = Arc::clone(&tournament_runtime);
            let clients = Arc::clone(&clients);
""",
    """        let accept_thread = {
            let authoritative_state = Arc::clone(&authoritative_state);
            let tournament_runtime = Arc::clone(&tournament_runtime);
            let transition_lock = Arc::clone(&transition_lock);
            let clients = Arc::clone(&clients);
""",
)
'''
if text.count(old) != 1:
    raise SystemExit(
        f"repair script: expected one ambiguous clone replacement, found {text.count(old)}"
    )
path.write_text(text.replace(old, new, 1))
runpy.run_path(str(path), run_name="__main__")
