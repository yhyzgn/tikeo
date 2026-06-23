import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../AboutPage.tsx', import.meta.url), 'utf8');
const routes = readFileSync(new URL('../../routes.tsx', import.meta.url), 'utf8');
const app = readFileSync(new URL('../../App.tsx', import.meta.url), 'utf8');
const shell = readFileSync(new URL('../../components/AppShell.tsx', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../../styles.css', import.meta.url), 'utf8');
const client = readFileSync(new URL('../../api/client.ts', import.meta.url), 'utf8');

describe('about page', () => {
  test('is wired into route metadata, app routes, and API client', () => {
    expect(routes).toContain("about: { path: '/about'");
    expect(routes).toContain("label: '关于'");
    expect(routes).toContain('InfoCircleOutlined');
    expect(routes).toContain("menu: false");
    expect(app).toContain("import('./pages/AboutPage')");
    expect(app).toContain('ROUTE_META.about.path');
    expect(shell).toContain('<InfoCircleOutlined />');
    expect(shell).toContain('ROUTE_META.about.path');
    expect(client).toContain('interface SystemInfoResponse');
    expect(client).toContain('gitTag: string');
    expect(client).toContain('buildTime: string');
    expect(client).toContain("request<SystemInfoResponse>('/api/v1/system/info')");
  });

  test('shows product, runtime version, GitHub latest release, and ecosystem links', () => {
    expect(source).toContain('About Tikeo');
    expect(source).toContain('关于 Tikeo');
    expect(source).toContain('getSystemInfo');
    expect(source).toContain('getClusterDiagnostics');
    expect(source).toContain('fetchLatestRelease');
    expect(source).toContain('api.github.com/repos');
    expect(source).toContain('img.shields.io/github/v/release');
    expect(source).toContain('parseReleaseTagFromBadge');
    expect(source).toContain('GitHub 最新发行版');
    expect(source).toContain('当前运行版本');
    expect(source).toContain('运行 Git Tag');
    expect(source).toContain('Tag 与版本一致');
    expect(source).toContain('Git Commit');
    expect(source).toContain('GitHub Repository');
    expect(source).toContain('Documentation');
    expect(source).toContain('Releases');
    expect(source).toContain('Issues');
    expect(source).toContain('License');
  });

  test('ships a dedicated polished visual layout', () => {
    expect(styles).toContain('.about-page');
    expect(styles).toContain('.about-hero');
    expect(styles).toContain('.about-version-orbit');
    expect(styles).toContain('.about-capability-card');
    expect(styles).toContain('.about-link-grid');
    expect(styles).toContain('.about-principle-grid');
    expect(styles).toContain('@keyframes about-orbit-spin');
  });
});
