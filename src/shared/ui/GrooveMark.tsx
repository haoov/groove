/** The app mark. Rings follow currentColor; the branch and its node are teal. */
export function GrooveMark({ size = 16 }: { size?: number }) {
  return (
    <svg
      className="groove-mark"
      width={size}
      height={size}
      viewBox="10 10 80 80"
      fill="none"
      aria-hidden="true"
    >
      <path
        d="M77.85 30.5 A34 34 0 1 0 84 50"
        stroke="currentColor"
        strokeWidth="8.5"
        strokeLinecap="round"
      />
      <path
        d="M59.31 33.87 A18.625 18.625 0 0 0 33.87 59.31"
        stroke="currentColor"
        strokeWidth="7"
        strokeLinecap="round"
      />
      <path
        className="groove-mark-branch"
        d="M84 50 L50 50"
        strokeWidth="8.5"
        strokeLinecap="round"
      />
      <circle className="groove-mark-node" cx="50" cy="50" r="7.5" />
    </svg>
  );
}
