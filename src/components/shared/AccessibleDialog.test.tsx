import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { AccessibleDialog } from "./AccessibleDialog";

function DialogHarness({ onCancel = vi.fn() }: { onCancel?: () => void }) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <button onClick={() => setOpen(true)} type="button">
        Open dialog
      </button>
      {open ? (
        <AccessibleDialog
          description="Confirm this operation."
          kicker="Confirmation"
          onCancel={() => {
            onCancel();
            setOpen(false);
          }}
          title="Accessible confirmation"
          titleId="accessible-confirmation-title"
        >
          <div className="button-row">
            <button type="button">Confirm</button>
            <button
              onClick={() => {
                onCancel();
                setOpen(false);
              }}
              type="button"
            >
              Cancel
            </button>
          </div>
        </AccessibleDialog>
      ) : null}
    </>
  );
}

describe("AccessibleDialog", () => {
  it("moves focus into the dialog and exposes modal labelling", () => {
    render(<DialogHarness />);
    fireEvent.click(screen.getByRole("button", { name: "Open dialog" }));

    const dialog = screen.getByRole("dialog", {
      name: "Accessible confirmation",
    });
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-describedby")).toBe(
      "accessible-confirmation-title-description",
    );
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Confirm" }),
    );
  });

  it("contains forward and reverse Tab navigation", () => {
    render(<DialogHarness />);
    fireEvent.click(screen.getByRole("button", { name: "Open dialog" }));

    const confirm = screen.getByRole("button", { name: "Confirm" });
    const cancel = screen.getByRole("button", { name: "Cancel" });
    cancel.focus();
    fireEvent.keyDown(cancel, { key: "Tab" });
    expect(document.activeElement).toBe(confirm);

    confirm.focus();
    fireEvent.keyDown(confirm, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(cancel);
  });

  it("closes on Escape and returns focus to the opener", () => {
    const onCancel = vi.fn();
    render(<DialogHarness onCancel={onCancel} />);
    const opener = screen.getByRole("button", { name: "Open dialog" });
    fireEvent.click(opener);

    fireEvent.keyDown(
      screen.getByRole("dialog", { name: "Accessible confirmation" }),
      { key: "Escape" },
    );

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(opener);
  });
});
