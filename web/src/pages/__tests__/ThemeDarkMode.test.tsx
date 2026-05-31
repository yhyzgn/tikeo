import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const appSource = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');

describe('dark mode theme coverage', () => {
  test('publishes data-theme and shared color tokens for custom modules', () => {
    expect(appSource).toContain('document.documentElement.dataset.theme = mode');
    expect(styles).toContain('html[data-theme="dark"]');
    expect(styles).toContain('--app-surface-solid');
    expect(styles).toContain('--app-text-strong');
    expect(styles).toContain('--app-canvas-bg');
  });

  test('custom cards canvases and feature modules consume theme variables', () => {
    expect(styles).toContain('background: var(--app-hero-bg)');
    expect(styles).toContain('background: var(--app-surface) !important');
    expect(styles).toContain('background: var(--app-canvas-bg)');
    expect(styles).toContain('html[data-theme="dark"] .scheduling-advice-stat-card.ant-card');
    expect(styles).toContain('html[data-theme="dark"] .topology-canvas-card--fullscreen');
    expect(styles).toContain('html[data-theme="dark"] .api-key-secret-box');
  });
});
