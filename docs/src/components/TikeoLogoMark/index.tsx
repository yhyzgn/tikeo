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
  title = 'Tikeo animated task-flow logo',
  decorative = false,
  size,
}: Props): ReactNode {
  const reactId = useId().replace(/:/g, '');
  const shellGradientId = `tikeo-logo-shell-${reactId}`;
  const innerGradientId = `tikeo-logo-inner-${reactId}`;
  const glowId = `tikeo-logo-glow-${reactId}`;
  const style = size ? ({'--tikeo-logo-size': `${size}px`} as CSSProperties) : undefined;
  const ariaProps: SVGProps<SVGSVGElement> = decorative ? {'aria-hidden': true} : {role: 'img', 'aria-label': title};

  return (
    <svg
      className={[styles.logo, className].filter(Boolean).join(' ')}
      style={style}
      viewBox="0 0 220 220"
      xmlns="http://www.w3.org/2000/svg"
      {...ariaProps}
    >
      {!decorative && <title>{title}</title>}
      <defs>
        <linearGradient id={shellGradientId} x1="50" y1="42" x2="170" y2="178" gradientUnits="userSpaceOnUse">
          <stop offset="0%" stopColor="var(--tikeo-logo-primary)" />
          <stop offset="54%" stopColor="var(--tikeo-logo-secondary)" />
          <stop offset="100%" stopColor="var(--tikeo-logo-tertiary)" />
        </linearGradient>
        <linearGradient id={innerGradientId} x1="74" y1="70" x2="148" y2="152" gradientUnits="userSpaceOnUse">
          <stop offset="0%" stopColor="var(--tikeo-logo-inner-hot)" stopOpacity="0.36" />
          <stop offset="100%" stopColor="var(--tikeo-logo-inner-cool)" stopOpacity="0.16" />
        </linearGradient>
        <filter id={glowId} x="-28%" y="-28%" width="156%" height="156%">
          <feGaussianBlur stdDeviation="4.2" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      <g className={styles.breathShell}>
        <path
          className={styles.shadow}
          d="M110 40 171 75v70l-61 35-61-35V75l61-35Z"
        />
        <path
          className={styles.shell}
          d="M110 40 171 75v70l-61 35-61-35V75l61-35Z"
          fill={`url(#${shellGradientId})`}
          filter={`url(#${glowId})`}
        />
        <path
          className={styles.innerShell}
          d="M110 62 151 86v48l-41 24-41-24V86l41-24Z"
          fill={`url(#${innerGradientId})`}
        />
      </g>

      <g className={styles.flow}>
        <path className={styles.flowBackbone} d="M75 94H111V145" />
        <path className={styles.flowArrow} d="M111 94H146" />
        <path className={styles.arrowHead} d="M136 77 156 97 136 117" />
        <circle className={styles.nodeHalo} cx="75" cy="94" r="16" />
        <circle className={styles.nodeHalo} cx="111" cy="94" r="16" />
        <circle className={styles.nodeHalo} cx="111" cy="145" r="16" />
        <circle className={styles.node} cx="75" cy="94" r="7" />
        <circle className={styles.nodeAccent} cx="111" cy="94" r="7" />
        <circle className={styles.node} cx="111" cy="145" r="7" />
      </g>
    </svg>
  );
}
