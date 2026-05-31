interface TikeeLogoProps {
  size?: number;
  className?: string;
  showWordmark?: boolean;
}

export function TikeeLogo({ size = 44, className = '', showWordmark = false }: TikeeLogoProps) {
  const classes = ['tikee-logo', showWordmark ? 'tikee-logo--with-wordmark' : '', className].filter(Boolean).join(' ');
  return (
    <span className={classes} aria-label="tikee task orchestration logo" role="img">
      <svg className="tikee-logo__mark" width={size} height={size} viewBox="0 0 64 64" fill="none" aria-hidden="true">
        <defs>
          <linearGradient id="tikee-logo-shell" x1="12" y1="7" x2="53" y2="58" gradientUnits="userSpaceOnUse">
            <stop stopColor="var(--app-primary-color)" />
            <stop offset="0.55" stopColor="var(--app-info-color)" />
            <stop offset="1" stopColor="#7c3aed" />
          </linearGradient>
          <linearGradient id="tikee-logo-line" x1="17" y1="31" x2="50" y2="37" gradientUnits="userSpaceOnUse">
            <stop stopColor="#dbeafe" />
            <stop offset="0.5" stopColor="#ffffff" />
            <stop offset="1" stopColor="#e0f2fe" />
          </linearGradient>
          <filter id="tikee-logo-glow" x="-30%" y="-30%" width="160%" height="160%" colorInterpolationFilters="sRGB">
            <feDropShadow dx="0" dy="8" stdDeviation="6" floodColor="var(--app-primary-color)" floodOpacity="0.28" />
          </filter>
        </defs>
        <path className="tikee-logo__shell" d="M32 5.5L54.5 18.5V45.5L32 58.5L9.5 45.5V18.5L32 5.5Z" fill="url(#tikee-logo-shell)" filter="url(#tikee-logo-glow)" />
        <path className="tikee-logo__inner" d="M32 13L47 21.5V42.5L32 51L17 42.5V21.5L32 13Z" fill="rgba(255,255,255,0.12)" stroke="rgba(255,255,255,0.28)" strokeWidth="1.6" />
        <path className="tikee-logo__track" d="M19 25.5H44" stroke="url(#tikee-logo-line)" strokeWidth="3.8" strokeLinecap="round" />
        <path className="tikee-logo__track" d="M32 25.5V45" stroke="url(#tikee-logo-line)" strokeWidth="3.8" strokeLinecap="round" />
        <path className="tikee-logo__track" d="M32 35.5H46" stroke="url(#tikee-logo-line)" strokeWidth="3.8" strokeLinecap="round" />
        <path className="tikee-logo__flow tikee-logo__flow--top" d="M19 25.5H44" stroke="#ffffff" strokeWidth="3.8" strokeLinecap="round" pathLength="100" />
        <path className="tikee-logo__flow tikee-logo__flow--right" d="M32 35.5H46" stroke="#ffffff" strokeWidth="3.8" strokeLinecap="round" pathLength="100" />
        <path className="tikee-logo__arrow" d="M41 31L47 35.5L41 40" stroke="#ffffff" strokeWidth="3.4" strokeLinecap="round" strokeLinejoin="round" />
        <circle className="tikee-logo__node tikee-logo__node--one" cx="19" cy="25.5" r="4.9" fill="#ffffff" />
        <circle className="tikee-logo__node tikee-logo__node--two" cx="32" cy="25.5" r="4.9" fill="#ffffff" />
        <circle className="tikee-logo__node tikee-logo__node--three" cx="32" cy="45" r="4.9" fill="#ffffff" />
        <circle className="tikee-logo__core tikee-logo__core--one" cx="19" cy="25.5" r="2" fill="var(--app-primary-color)" />
        <circle className="tikee-logo__core tikee-logo__core--two" cx="32" cy="25.5" r="2" fill="var(--app-info-color)" />
        <circle className="tikee-logo__core tikee-logo__core--three" cx="32" cy="45" r="2" fill="#7c3aed" />
      </svg>
      {showWordmark ? <span className="tikee-logo__wordmark">tikee</span> : null}
    </span>
  );
}
