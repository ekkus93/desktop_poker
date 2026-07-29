from pathlib import Path

path = Path("src/api/desktop.contract.test.ts")
text = path.read_text(encoding="utf-8")
old = '''    expect(sortedKeys(hostRuntimeHealth)).toEqual(
      expectedKeys("HostRuntimeHealth"),
    );'''
new = '''    expect(sortedKeys(debugState.hostRuntimeHealth ?? {})).toEqual(
      expectedKeys("HostRuntimeHealth"),
    );'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("expected HostRuntimeHealth contract assertion was not found")
path.write_text(text, encoding="utf-8")
