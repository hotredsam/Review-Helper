import { useRef, type ReactNode } from "react";
import { useFocusTrap } from "../lib/focusTrap";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
}

/** Lightweight modal: scrim + centered card. Closes on Escape or scrim click.
 *  The scrim uses the theme `--scrim` token, so it works in every theme.
 *  Focus handling (trap, restore) lives in the shared useFocusTrap hook. */
export function Modal({ open, onClose, title, children }: ModalProps) {
  const dialogRef = useRef<HTMLDivElement>(null);
  useFocusTrap(dialogRef, open, onClose);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-4"
      onClick={onClose}
      role="presentation"
    >
      <div
        ref={dialogRef}
        className="w-full max-w-md rounded-xl border border-border bg-overlay p-5 shadow-xl"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
      >
        <h2 className="mb-4 text-base font-semibold text-fg">{title}</h2>
        {children}
      </div>
    </div>
  );
}
