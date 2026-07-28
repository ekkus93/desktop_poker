import {
  useEffect,
  useRef,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
} from "react";

const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

function focusableElements(container: HTMLElement): HTMLElement[] {
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (element) =>
      !element.hasAttribute("hidden") &&
      element.getAttribute("aria-hidden") !== "true",
  );
}

export function AccessibleDialog({
  children,
  className = "dialog-card",
  description,
  kicker,
  onCancel,
  title,
  titleId,
}: {
  children: ReactNode;
  className?: string;
  description?: string;
  kicker?: string;
  onCancel: () => void;
  title: string;
  titleId: string;
}) {
  const dialogRef = useRef<HTMLElement>(null);
  const onCancelRef = useRef(onCancel);

  useEffect(() => {
    onCancelRef.current = onCancel;
  }, [onCancel]);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) {
      return;
    }

    const returnFocusTarget =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    const firstFocusable = focusableElements(dialog)[0];
    (firstFocusable ?? dialog).focus();

    return () => {
      if (returnFocusTarget?.isConnected) {
        returnFocusTarget.focus();
      }
    };
  }, []);

  function handleKeyDown(event: ReactKeyboardEvent<HTMLElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      onCancelRef.current();
      return;
    }

    if (event.key !== "Tab") {
      return;
    }

    const dialog = dialogRef.current;
    if (!dialog) {
      return;
    }
    const focusable = focusableElements(dialog);
    if (focusable.length === 0) {
      event.preventDefault();
      dialog.focus();
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  return (
    <section
      aria-describedby={description ? `${titleId}-description` : undefined}
      aria-labelledby={titleId}
      aria-modal="true"
      className={className}
      onKeyDown={handleKeyDown}
      ref={dialogRef}
      role="dialog"
      tabIndex={-1}
    >
      {kicker ? <p className="kicker">{kicker}</p> : null}
      <h3 id={titleId}>{title}</h3>
      {description ? <p id={`${titleId}-description`}>{description}</p> : null}
      {children}
    </section>
  );
}
