import { describe, expect, it } from "vitest";
import readme from "../../README.md?raw";
import workflow from "../../.github/workflows/ci.yml?raw";

describe("CI workflow smoke coverage", () => {
  it("keeps the GitHub Actions workflow file and the required verify steps", () => {
    expect(workflow).toContain("name: CI");
    expect(workflow).toContain("- name: Check formatting");
    expect(workflow).toContain("- name: Lint Rust");
    expect(workflow).toContain("- name: Test Rust");
    expect(workflow).toContain("- name: Lint frontend");
    expect(workflow).toContain("- name: Test frontend");
    expect(workflow).toContain("- name: Build frontend");
  });

  it("keeps release publishing gated to version tags only", () => {
    expect(workflow).toContain("tags:");
    expect(workflow).toContain('- "v*"');
    expect(workflow).toContain("pull_request:");
    expect(workflow).toContain("if: startsWith(github.ref, 'refs/tags/')");
    expect(workflow).toContain("uses: tauri-apps/tauri-action@v0");
    expect(workflow).toContain("tagName: ${{ github.ref_name }}");
  });

  it("keeps the README badge pointed at the active workflow file", () => {
    expect(readme).toContain(
      "https://github.com/ekkus93/desktop_poker/actions/workflows/ci.yml/badge.svg",
    );
    expect(readme).toContain(
      "https://github.com/ekkus93/desktop_poker/actions/workflows/ci.yml",
    );
  });
});
