from pathlib import Path

path = Path("src-tauri/tests/desktop_contract.rs")
text = path.read_text(encoding="utf-8")

old_import = '''    npc::{
        profile_store::NpcProfileListResult, LlmProviderConfig, LlmProviderSettings,
        LlmProviderType,
    },
};'''
new_import = '''    networking::HostRuntimeHealth,
    npc::{
        profile_store::NpcProfileListResult, LlmProviderConfig, LlmProviderSettings,
        LlmProviderType,
    },
};'''
if old_import in text:
    text = text.replace(old_import, new_import, 1)
elif new_import not in text:
    raise SystemExit("expected desktop contract import block was not found")

anchor = '''    assert_contract_keys(
        "NpcProfileListResult",
        &NpcProfileListResult {
'''
insertion = '''    assert_contract_keys("HostRuntimeHealth", &HostRuntimeHealth::default());

    assert_contract_keys(
        "NpcProfileListResult",
        &NpcProfileListResult {
'''
if anchor in text:
    text = text.replace(anchor, insertion, 1)
elif 'assert_contract_keys("HostRuntimeHealth"' not in text:
    raise SystemExit("expected desktop contract assertion anchor was not found")

path.write_text(text, encoding="utf-8")
