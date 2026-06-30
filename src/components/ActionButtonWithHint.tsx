interface Props {
  label: string;
  hint: string;
  disabled?: boolean;
  buttonClassName?: string;
  onClick: () => void;
}

export function ActionButtonWithHint({
  label,
  hint,
  disabled = false,
  buttonClassName,
  onClick,
}: Props) {
  return (
    <div className={`action-btn-hint-wrap${disabled ? " is-disabled" : ""}`}>
      <button
        type="button"
        className={buttonClassName}
        disabled={disabled}
        onClick={onClick}
      >
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
