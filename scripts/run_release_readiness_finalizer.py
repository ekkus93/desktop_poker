#!/usr/bin/env python3
"""Run the release-readiness finalizer with guarded first-match replacements."""

from __future__ import annotations

import scripts.finalize_release_readiness_baseline as finalizer


def replace_first(text: str, old: str, new: str, *, label: str) -> str:
    count = text.count(old)
    if count == 0:
        print(f"warning: {label}: source text already changed or not found")
        return text
    if count > 1:
        print(f"note: {label}: {count} matches; updating the first section-ordered match")
    return text.replace(old, new, 1)


def check_first(text: str, item: str) -> str:
    unchecked = f"- [ ] {item}"
    checked = f"- [x] {item}"
    if checked in text:
        return text
    count = text.count(unchecked)
    if count == 0:
        print(f"warning: TODO item not found: {item}")
        return text
    if count > 1:
        print(f"note: TODO item repeated {count} times; checking the first: {item}")
    return text.replace(unchecked, checked, 1)


finalizer.replace_once = replace_first
finalizer.check_item = check_first
finalizer.main()
