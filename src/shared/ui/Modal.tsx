import { useEffect, useRef, type ReactNode } from 'react';

/** The overlay + panel shell: click-outside, Escape, and focus on open. */
export function Modal({
  title, subtitle, subtitleClassName, className, onClose, children,
}: {
  title: ReactNode;
  subtitle?: ReactNode;
  /** Extra classes on `.wizard-subtitle`. */
  subtitleClassName?: string;
  /** Extra classes on `.wizard-modal`. */
  className?: string;
  onClose: () => void;
  children: ReactNode;
}) {
  const panelRef = useRef<HTMLDivElement>(null);

  // The panel takes focus unless an `autoFocus` child already has it, and hands it back on close.
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const panel = panelRef.current;
    if (panel && !panel.contains(previous)) panel.focus();
    return () => previous?.focus?.();
  }, []);

  return (
    <div className="wizard-overlay" onClick={onClose}>
      <div
        ref={panelRef}
        className={className ? `wizard-modal ${className}` : 'wizard-modal'}
        role="dialog"
        aria-modal="true"
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        // Stopped here: the same Escape would also close the popover hosting this modal.
        onKeyDown={(e) => { if (e.key === 'Escape') { e.stopPropagation(); onClose(); } }}
      >
        <div className="wizard-header">
          <div className="wizard-title">{title}</div>
          {subtitle && (
            <div className={subtitleClassName ? `wizard-subtitle ${subtitleClassName}` : 'wizard-subtitle'}>
              {subtitle}
            </div>
          )}
          <button className="wizard-close" onClick={onClose}>×</button>
        </div>
        {children}
      </div>
    </div>
  );
}
