import type { ReactNode } from "react";
import { Modal } from "./Modal";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  body: ReactNode;
  confirmLabel: string;
  /** Destructive styling (default). Pass false for non-destructive confirms. */
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

/** House standard for destructive actions: an in-app Modal confirm — never
 *  window.confirm, which silently returns false under wry. Inherits Modal's
 *  focus trap, Escape-to-cancel, and focus restore. */
export function ConfirmDialog({ open, title, body, confirmLabel, danger = true, onConfirm, onCancel }: ConfirmDialogProps) {
  return (
    <Modal open={open} onClose={onCancel} title={title}>
      <div className="space-y-4">
        <div className="text-sm text-fg-muted">{body}</div>
        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="rounded-md border border-border px-3 py-1.5 text-sm text-fg-muted hover:bg-surface-2"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            className={
              "rounded-md px-3 py-1.5 text-sm font-medium " +
              (danger ? "bg-danger text-danger-fg hover:opacity-90" : "bg-accent text-accent-fg hover:bg-accent-hover")
            }
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </Modal>
  );
}
