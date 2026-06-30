interface Props {
  label: string;
  hint: string;
  disabled?: boolean;
  onClick: () => void;
}

export function ActionButtonWithHint({ label, hint, disabled = false, onClick }: Props) {
  return (
    <div className={`action-btn-hint-wrap${disabled ? " is-disabled" : ""}`}>
      <button type="button" disabled={disabled} onClick={onClick}>
        {label}
      </button>
      <span className="action-btn-info" tabIndex={0} aria-label={hint}>
        i
        <span className="action-btn-tooltip" role="tooltip">
          {hint}
        </span>
      </span>
    </div>
  );
}
