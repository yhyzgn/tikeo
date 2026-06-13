import type {CSSProperties, ReactNode, SVGProps} from 'react';
import {useId} from 'react';

import styles from './styles.module.css';

type Props = {
  className?: string;
  title?: string;
  decorative?: boolean;
  size?: number;
};

export default function TikeoLogoMark({
  className,
  title = 'Tikeo task orchestration logo',
  decorative = false,
  size,
}: Props): ReactNode {
  const reactId = useId().replace(/:/g, '');
  const shellId = `tikeo-logo-shell-${reactId}`;
  const lineId = `tikeo-logo-line-${reactId}`;
  const glowId = `tikeo-logo-glow-${reactId}`;
  const style = size ? ({'--tikeo-logo-size': `${size}px`} as CSSProperties) : undefined;
  const ariaProps: SVGProps<SVGSVGElement> = decorative ? {'aria-hidden': true} : {role: 'img', 'aria-label': title};

  return (
    <svg
      className={[styles.logo, className].filter(Boolean).join(' ')}
      style={style}
      viewBox="4 4 56 56"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...ariaProps}
    >
      {!decorative && <title>{title}</title>}
      <defs>
        <linearGradient id={shellId} x1="12" y1="7" x2="53" y2="58" gradientUnits="userSpaceOnUse">
          <stop stopColor="var(--app-primary-color)" />
          <stop offset="0.55" stopColor="var(--app-info-color)" />
          <stop offset="1" stopColor="var(--tikeo-logo-accent)" />
        </linearGradient>
        <linearGradient id={lineId} x1="17" y1="31" x2="50" y2="37" gradientUnits="userSpaceOnUse">
          <stop stopColor="var(--tikeo-logo-line-start)" />
          <stop offset="0.5" stopColor="var(--tikeo-logo-node-fill)" />
          <stop offset="1" stopColor="var(--tikeo-logo-line-end)" />
        </linearGradient>
        <filter id={glowId} x="-30%" y="-30%" width="160%" height="160%" colorInterpolationFilters="sRGB">
          <feDropShadow dx="0" dy="8" stdDeviation="6" floodColor="var(--app-primary-color)" floodOpacity="0.28" />
        </filter>
      </defs>
      <path className={styles.shell} d="M32 5.5L54.5 18.5V45.5L32 58.5L9.5 45.5V18.5L32 5.5Z" fill={`url(#${shellId})`} filter={`url(#${glowId})`} />
      <path className={styles.inner} d="M32 13L47 21.5V42.5L32 51L17 42.5V21.5L32 13Z" fill="var(--tikeo-logo-inner-fill)" stroke="var(--tikeo-logo-inner-stroke)" strokeWidth="1.6" />
      <path className={styles.track} d="M19 25.5H44" stroke={`url(#${lineId})`} strokeWidth="3.8" strokeLinecap="round" />
      <path className={styles.track} d="M32 25.5V45" stroke={`url(#${lineId})`} strokeWidth="3.8" strokeLinecap="round" />
      <path className={styles.track} d="M32 35.5H46" stroke={`url(#${lineId})`} strokeWidth="3.8" strokeLinecap="round" />
      <path className={`${styles.flow} ${styles.flowTop}`} d="M19 25.5H44" stroke="var(--tikeo-logo-node-fill)" strokeWidth="3.8" strokeLinecap="round" pathLength="100" />
      <path className={`${styles.flow} ${styles.flowRight}`} d="M32 35.5H46" stroke="var(--tikeo-logo-node-fill)" strokeWidth="3.8" strokeLinecap="round" pathLength="100" />
      <path className={styles.arrow} d="M41 31L47 35.5L41 40" stroke="var(--tikeo-logo-node-fill)" strokeWidth="3.4" strokeLinecap="round" strokeLinejoin="round" />
      <circle className={`${styles.node} ${styles.nodeOne}`} cx="19" cy="25.5" r="4.9" fill="var(--tikeo-logo-node-fill)" />
      <circle className={`${styles.node} ${styles.nodeTwo}`} cx="32" cy="25.5" r="4.9" fill="var(--tikeo-logo-node-fill)" />
      <circle className={`${styles.node} ${styles.nodeThree}`} cx="32" cy="45" r="4.9" fill="var(--tikeo-logo-node-fill)" />
      <circle className={`${styles.core} ${styles.coreOne}`} cx="19" cy="25.5" r="2" fill="var(--app-primary-color)" />
      <circle className={`${styles.core} ${styles.coreTwo}`} cx="32" cy="25.5" r="2" fill="var(--app-info-color)" />
      <circle className={`${styles.core} ${styles.coreThree}`} cx="32" cy="45" r="2" fill="var(--tikeo-logo-accent)" />
    </svg>
  );
}
