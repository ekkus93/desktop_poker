import { describe, expect, it } from "vitest";
import readme from "../../README.md?raw";
import workflow from "../../.github/workflows/ci.yml?raw";

describe("CI workflow smoke coverage", () => {
  it("keeps the GitHub Actions workflow file and the required verify steps", () => {
    expect(workflow).toContain("name: CI");
    expect(workflow).toContain("- name: Check Rust formatting");
    expect(workflow).toContain("- name: Lint Rust");
    expect(workflow).toContain("- name: Test Rust");
    expect(workflow).toContain("- name: Audit poker-core dependency tree");
    expect(workflow).toContain("- name: Check frontend formatting");
    expect(workflow).toContain("- name: Lint frontend");
    expect(workflow).toContain("- name: Test frontend");
    expect(workflow).toContain("- name: Test browser geometry");
    expect(workflow).toContain("- name: Build frontend");
  });

  it("runs browser geometry in the prebuilt Playwright container (no runtime browser download)", () => {
    // The runtime `playwright install` deterministically hung on the hosted
    // runner; the geometry job must use the prebuilt image instead.
    expect(workflow).toContain("image: mcr.microsoft.com/playwright:");
    // No runtime browser-install command (the comment may mention it; the
    // invoked command must not appear).
    expect(workflow).not.toContain("npx playwright install");
  });

  it("caps every job's runtime so a hung step can't bill to the 6-hour ceiling", () => {
    // At least one timeout-minutes per job (verify, geometry, release).
    const caps = workflow.match(/timeout-minutes:/g) ?? [];
    expect(caps.length).toBeGreaterThanOrEqual(3);
    expect(workflow).toContain("cancel-in-progress: true");
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
