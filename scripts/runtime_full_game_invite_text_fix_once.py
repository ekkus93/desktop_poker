from pathlib import Path

path = Path("scripts/runtime_full_game_smoke.py")
text = path.read_text(encoding="utf-8")
old = '    wait_for_source(client, "Invite looks good")'
new = '    wait_for_source(client, "Invite decoded")'
if old in text:
    path.write_text(text.replace(old, new, 1), encoding="utf-8")
elif new not in text:
    raise SystemExit("expected full-game invite assertion not found")
