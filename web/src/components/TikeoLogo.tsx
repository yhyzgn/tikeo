interface TikeoLogoProps {
  size?: number;
  className?: string;
  showWordmark?: boolean;
}

export function TikeoLogo({ size = 44, className = '', showWordmark = false }: TikeoLogoProps) {
  const classes = ['tikeo-logo', showWordmark ? 'tikeo-logo--with-wordmark' : '', className].filter(Boolean).join(' ');
  return (
    <span className={classes} aria-label="tikeo task orchestration logo" role="img">
      <svg className="tikeo-logo__mark" width={size} height={size} viewBox="4 4 56 56" fill="none" aria-hidden="true">
        <defs>
          <linearGradient id="tikeo-logo-shell" x1="12" y1="7" x2="53" y2="58" gradientUnits="userSpaceOnUse">
            <stop stopColor="var(--app-primary-color)" />
            <stop offset="0.55" stopColor="var(--app-info-color)" />
            <stop offset="1" stopColor="var(--tikeo-logo-accent)" />
          </linearGradient>
          <linearGradient id="tikeo-logo-line" x1="17" y1="31" x2="50" y2="37" gradientUnits="userSpaceOnUse">
            <stop stopColor="var(--tikeo-logo-line-start)" />
            <stop offset="0.5" stopColor="var(--tikeo-logo-node-fill)" />
            <stop offset="1" stopColor="var(--tikeo-logo-line-end)" />
          </linearGradient>
          <filter id="tikeo-logo-glow" x="-30%" y="-30%" width="160%" height="160%" colorInterpolationFilters="sRGB">
            <feDropShadow dx="0" dy="8" stdDeviation="6" floodColor="var(--app-primary-color)" floodOpacity="0.28" />
          </filter>
        </defs>
        <path className="tikeo-logo__shell" d="M32 5.5L54.5 18.5V45.5L32 58.5L9.5 45.5V18.5L32 5.5Z" fill="url(#tikeo-logo-shell)" filter="url(#tikeo-logo-glow)" />
        <path className="tikeo-logo__inner" d="M32 13L47 21.5V42.5L32 51L17 42.5V21.5L32 13Z" fill="var(--tikeo-logo-inner-fill)" stroke="var(--tikeo-logo-inner-stroke)" strokeWidth="1.6" />
        <path className="tikeo-logo__track" d="M19 25.5H44" stroke="url(#tikeo-logo-line)" strokeWidth="3.8" strokeLinecap="round" />
        <path className="tikeo-logo__track" d="M32 25.5V45" stroke="url(#tikeo-logo-line)" strokeWidth="3.8" strokeLinecap="round" />
        <path className="tikeo-logo__track" d="M32 35.5H46" stroke="url(#tikeo-logo-line)" strokeWidth="3.8" strokeLinecap="round" />
        <path className="tikeo-logo__flow tikeo-logo__flow--top" d="M19 25.5H44" stroke="var(--tikeo-logo-node-fill)" strokeWidth="3.8" strokeLinecap="round" pathLength="100" />
        <path className="tikeo-logo__flow tikeo-logo__flow--right" d="M32 35.5H46" stroke="var(--tikeo-logo-node-fill)" strokeWidth="3.8" strokeLinecap="round" pathLength="100" />
        <path className="tikeo-logo__arrow" d="M41 31L47 35.5L41 40" stroke="var(--tikeo-logo-node-fill)" strokeWidth="3.4" strokeLinecap="round" strokeLinejoin="round" />
        <circle className="tikeo-logo__node tikeo-logo__node--one" cx="19" cy="25.5" r="4.9" fill="var(--tikeo-logo-node-fill)" />
        <circle className="tikeo-logo__node tikeo-logo__node--two" cx="32" cy="25.5" r="4.9" fill="var(--tikeo-logo-node-fill)" />
        <circle className="tikeo-logo__node tikeo-logo__node--three" cx="32" cy="45" r="4.9" fill="var(--tikeo-logo-node-fill)" />
        <circle className="tikeo-logo__core tikeo-logo__core--one" cx="19" cy="25.5" r="2" fill="var(--app-primary-color)" />
        <circle className="tikeo-logo__core tikeo-logo__core--two" cx="32" cy="25.5" r="2" fill="var(--app-info-color)" />
        <circle className="tikeo-logo__core tikeo-logo__core--three" cx="32" cy="45" r="2" fill="var(--tikeo-logo-accent)" />
      </svg>
      {showWordmark ? <span className="tikeo-logo__wordmark">tikeo</span> : null}
    </span>
  );
}
