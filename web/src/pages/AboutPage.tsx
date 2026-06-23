import {
  ApiOutlined,
  BranchesOutlined,
  CheckCircleOutlined,
  CloudServerOutlined,
  CodeOutlined,
  DeploymentUnitOutlined,
  GithubOutlined,
  GlobalOutlined,
  LinkOutlined,
  RocketOutlined,
  SafetyCertificateOutlined,
  ThunderboltOutlined,
} from '@ant-design/icons';
import { Alert, Button, Card, Col, Descriptions, Empty, Row, Skeleton, Space, Statistic, Tag, Timeline, Typography } from 'antd';
import { useCallback, useEffect, useMemo, useState, type CSSProperties } from 'react';

import { getClusterDiagnostics, getSystemInfo, type ClusterDiagnosticsResponse, type SystemInfoResponse } from '../api/client';
import { TikeoLogo } from '../components/TikeoLogo';
import { useRouteActive } from '../hooks/useRouteActivation';
import { ROUTE_META } from '../routes';

interface GithubRelease {
  tag_name: string;
  name: string | null;
  html_url: string;
  published_at: string | null;
  prerelease: boolean;
  draft: boolean;
}

const REPO_OWNER = 'yhyzgn';
const REPO_NAME = 'tikeo';
const GITHUB_REPO_URL = `https://github.com/${REPO_OWNER}/${REPO_NAME}`;
const DOCS_URL = 'https://docs.tikeo.net';
const RELEASES_URL = `${GITHUB_REPO_URL}/releases`;
const ISSUES_URL = `${GITHUB_REPO_URL}/issues`;
const LICENSE_URL = `${GITHUB_REPO_URL}/blob/main/LICENSE`;

function formatDate(value: string | null | undefined): string {
  if (!value) return '-';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, { year: 'numeric', month: '2-digit', day: '2-digit' }).format(date);
}

function normalizeVersion(value: string | null | undefined): string {
  return (value ?? '').trim().replace(/^v/i, '');
}

function compareVersions(current: string | null | undefined, latest: string | null | undefined): 'same' | 'behind' | 'unknown' {
  const left = normalizeVersion(current);
  const right = normalizeVersion(latest);
  if (!left || !right) return 'unknown';
  if (left === right) return 'same';
  return 'behind';
}

function expectedGitTag(version: string | null | undefined): string {
  const normalized = normalizeVersion(version);
  return normalized ? `v${normalized}` : '';
}

function gitTagState(info: SystemInfoResponse | null): 'same' | 'mismatch' | 'unknown' {
  if (!info?.gitTag) return 'unknown';
  return info.gitTag === expectedGitTag(info.version) ? 'same' : 'mismatch';
}

function parseReleaseTagFromBadge(svg: string): string | null {
  const textValues = [...svg.matchAll(/<text[^>]*>([^<]+)<\/text>/g)]
    .map((match) => match[1].replace(/&[^;]+;/g, '').trim())
    .filter(Boolean);
  return textValues.find((value) => /^v?\d+\.\d+\.\d+/.test(value)) ?? null;
}

function releaseFromTag(tag: string): GithubRelease {
  const normalized = tag.startsWith('v') ? tag : `v${tag}`;
  return {
    tag_name: normalized,
    name: normalized,
    html_url: `${GITHUB_REPO_URL}/releases/tag/${normalized}`,
    published_at: null,
    prerelease: normalized.includes('-'),
    draft: false,
  };
}

async function fetchLatestReleaseFromBadge(): Promise<GithubRelease> {
  const response = await fetch(`https://img.shields.io/github/v/release/${REPO_OWNER}/${REPO_NAME}?label=release`, {
    headers: { accept: 'image/svg+xml' },
  });
  if (!response.ok) {
    throw new Error(`GitHub release badge fallback failed: ${response.status}`);
  }
  const tag = parseReleaseTagFromBadge(await response.text());
  if (!tag) {
    throw new Error('GitHub release badge fallback did not contain a release tag');
  }
  return releaseFromTag(tag);
}

async function fetchLatestRelease(): Promise<GithubRelease> {
  const response = await fetch(`https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest`, {
    headers: { accept: 'application/vnd.github+json' },
  });
  if (response.ok) {
    return await response.json() as GithubRelease;
  }
  try {
    return await fetchLatestReleaseFromBadge();
  } catch {
    if (response.status === 403) {
      throw new Error('GitHub API anonymous quota or network policy returned 403; latest release fallback also failed.');
    }
    throw new Error(`GitHub latest release request failed: ${response.status}`);
  }
}

const capabilityCards = [
  { title: 'Worker Tunnel', desc: '业务 Worker 主动出站连接，不需要暴露执行器入站端口。', icon: <CloudServerOutlined />, color: '#2563eb' },
  { title: 'Raft FSOD HA', desc: 'Leader fencing、shard ownership 与 durable outbox 共同支撑多 Pod Server。', icon: <DeploymentUnitOutlined />, color: '#7c3aed' },
  { title: '多语言 SDK', desc: 'Java、Rust、Go、Python、Node.js Worker 共享同一调度协议。', icon: <CodeOutlined />, color: '#0891b2' },
  { title: '治理与审计', desc: 'RBAC、API-Key、脚本发布门禁、通知投递与审计证据闭环。', icon: <SafetyCertificateOutlined />, color: '#0f766e' },
];

const ecosystemLinks = [
  { label: 'GitHub Repository', href: GITHUB_REPO_URL, icon: <GithubOutlined /> },
  { label: 'Documentation', href: DOCS_URL, icon: <GlobalOutlined /> },
  { label: 'Releases', href: RELEASES_URL, icon: <RocketOutlined /> },
  { label: 'Issues', href: ISSUES_URL, icon: <LinkOutlined /> },
  { label: 'License', href: LICENSE_URL, icon: <SafetyCertificateOutlined /> },
];

export function AboutPage() {
  const active = useRouteActive(ROUTE_META.about.path);
  const [systemInfo, setSystemInfo] = useState<SystemInfoResponse | null>(null);
  const [clusterDiagnostics, setClusterDiagnostics] = useState<ClusterDiagnosticsResponse | null>(null);
  const [latestRelease, setLatestRelease] = useState<GithubRelease | null>(null);
  const [loading, setLoading] = useState(true);
  const [releaseError, setReleaseError] = useState<string | null>(null);
  const [systemError, setSystemError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setSystemError(null);
    setReleaseError(null);
    const [system, cluster, release] = await Promise.allSettled([
      getSystemInfo(),
      getClusterDiagnostics(),
      fetchLatestRelease(),
    ]);
    if (system.status === 'fulfilled') {
      setSystemInfo(system.value);
    } else {
      setSystemError(system.reason instanceof Error ? system.reason.message : '系统信息加载失败');
    }
    if (cluster.status === 'fulfilled') setClusterDiagnostics(cluster.value);
    if (release.status === 'fulfilled') {
      setLatestRelease(release.value);
    } else {
      setReleaseError(release.reason instanceof Error ? release.reason.message : 'GitHub release 加载失败');
    }
    setLoading(false);
  }, []);

  useEffect(() => { if (active) void load(); }, [active, load]);

  const versionState = useMemo(() => compareVersions(systemInfo?.version, latestRelease?.tag_name), [systemInfo?.version, latestRelease?.tag_name]);
  const runtimeTagState = useMemo(() => gitTagState(systemInfo), [systemInfo]);
  const clusterStatus = clusterDiagnostics?.smartGateway?.status ?? clusterDiagnostics?.status?.role ?? 'unknown';
  const nodeCount = clusterDiagnostics?.nodes.length ?? clusterDiagnostics?.status?.nodes ?? 0;

  return (
    <div className="page-stack about-page">
      <section className="hero-panel about-hero">
        <div className="about-hero__mark"><TikeoLogo size={96} /></div>
        <div className="hero-panel__content">
          <div className="hero-panel__header">
            <Tag color="blue" className="soft-tag">About Tikeo</Tag>
            <Typography.Title level={1}>关于 Tikeo</Typography.Title>
          </div>
          <Typography.Paragraph className="hero-panel__desc">
            Tikeo 是一个 Rust 原生的任务编排控制平面：把计划任务、API 触发、工作流 DAG、出站 Worker Tunnel、多语言 SDK、脚本治理、通知投递、审计证据与云原生部署面组合成一个平台。
          </Typography.Paragraph>
          <Space wrap>
            <Button type="primary" icon={<GithubOutlined />} href={GITHUB_REPO_URL} target="_blank">GitHub</Button>
            <Button icon={<GlobalOutlined />} href={DOCS_URL} target="_blank">文档站</Button>
            <Button icon={<RocketOutlined />} href={RELEASES_URL} target="_blank">Releases</Button>
            <Button onClick={() => void load()} loading={loading}>刷新信息</Button>
          </Space>
        </div>
        <div className="about-version-orbit" aria-label="版本状态">
          <span className="about-version-orbit__ring" />
          <strong>{systemInfo?.version ?? '-'}</strong>
          <span>running version</span>
          <Tag color={versionState === 'same' ? 'green' : versionState === 'behind' ? 'gold' : 'default'}>
            {versionState === 'same' ? '已是最新' : versionState === 'behind' ? '有新版本' : '等待 release 信息'}
          </Tag>
        </div>
      </section>

      {(systemError || releaseError) ? (
        <Alert
          showIcon
          type="warning"
          message="部分信息加载失败"
          description={<span data-runtime-text>{[systemError, releaseError].filter(Boolean).join('；')}</span>}
        />
      ) : null}

      <Row gutter={[16, 16]}>
        <Col xs={24} md={12} xl={6}>
          <Card className="metric-card about-stat-card">
            <Statistic prefix={<ThunderboltOutlined />} title="当前运行版本" value={systemInfo?.version ?? '-'} loading={loading && !systemInfo} />
          </Card>
        </Col>
        <Col xs={24} md={12} xl={6}>
          <Card className="metric-card about-stat-card">
            <Statistic prefix={<RocketOutlined />} title="GitHub 最新发行版" value={latestRelease?.tag_name ?? '-'} loading={loading && !latestRelease} />
          </Card>
        </Col>
        <Col xs={24} md={12} xl={6}>
          <Card className="metric-card about-stat-card">
            <Statistic prefix={<GithubOutlined />} title="运行 Git Tag" value={systemInfo?.gitTag || '-'} loading={loading && !systemInfo} />
          </Card>
        </Col>
        <Col xs={24} md={12} xl={6}>
          <Card className="metric-card about-stat-card">
            <Statistic prefix={<DeploymentUnitOutlined />} title="Server 节点 / 状态" value={`${nodeCount} · ${clusterStatus}`} loading={loading && !clusterDiagnostics} />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} xl={15}>
          <Card className="clean-card about-card" title="项目信息">
            <Skeleton loading={loading && !systemInfo} active paragraph={{ rows: 4 }}>
              <Descriptions column={{ xs: 1, md: 2 }} bordered size="small">
                <Descriptions.Item label="项目名称">{systemInfo?.name ?? 'tikeo'}</Descriptions.Item>
                <Descriptions.Item label="运行版本">{systemInfo?.version ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="运行目标">{systemInfo?.target ?? '-'}</Descriptions.Item>
                <Descriptions.Item label="运行 Git Tag">
                  <Space size={6}>
                    <span>{systemInfo?.gitTag || '-'}</span>
                    <Tag color={runtimeTagState === 'same' ? 'green' : runtimeTagState === 'mismatch' ? 'red' : 'default'}>
                      {runtimeTagState === 'same' ? 'Tag 与版本一致' : runtimeTagState === 'mismatch' ? 'Tag 与版本不一致' : '未绑定 Tag'}
                    </Tag>
                  </Space>
                </Descriptions.Item>
                <Descriptions.Item label="Git Commit">{systemInfo?.gitSha || '-'}</Descriptions.Item>
                <Descriptions.Item label="构建时间">{systemInfo?.buildTime || '-'}</Descriptions.Item>
                <Descriptions.Item label="工作区状态">{systemInfo?.gitDirty || '-'}</Descriptions.Item>
                <Descriptions.Item label="最新发行版">
                  {latestRelease ? <a href={latestRelease.html_url} target="_blank" rel="noreferrer">{latestRelease.tag_name}</a> : '-'}
                </Descriptions.Item>
                <Descriptions.Item label="发布时间">{formatDate(latestRelease?.published_at)}</Descriptions.Item>
                <Descriptions.Item label="发行状态">
                  {latestRelease ? <Space><Tag color={latestRelease.prerelease ? 'gold' : 'green'}>{latestRelease.prerelease ? 'pre-release' : 'stable'}</Tag>{latestRelease.draft ? <Tag>draft</Tag> : null}</Space> : '-'}
                </Descriptions.Item>
                <Descriptions.Item label="仓库" span={2}><a href={GITHUB_REPO_URL} target="_blank" rel="noreferrer">{`${REPO_OWNER}/${REPO_NAME}`}</a></Descriptions.Item>
              </Descriptions>
            </Skeleton>
          </Card>
        </Col>
        <Col xs={24} xl={9}>
          <Card className="clean-card about-card" title="版本判断">
            <Timeline
              items={[
                { color: systemInfo ? 'green' : 'gray', dot: <CheckCircleOutlined />, children: <span>Server system info：<strong>{systemInfo?.version ?? '未加载'}</strong></span> },
                { color: runtimeTagState === 'same' ? 'green' : runtimeTagState === 'mismatch' ? 'red' : 'gray', dot: <GithubOutlined />, children: <span>Runtime git tag：<strong>{systemInfo?.gitTag || '未绑定 Tag'}</strong></span> },
                { color: latestRelease ? 'blue' : 'gray', dot: <RocketOutlined />, children: <span>GitHub latest release：<strong>{latestRelease?.tag_name ?? '未加载'}</strong></span> },
                { color: versionState === 'same' ? 'green' : versionState === 'behind' ? 'gold' : 'gray', children: versionState === 'same' ? '当前运行版本与最新发行版一致。' : versionState === 'behind' ? 'GitHub 上已有不同版本，请按 release note 评估升级。' : '等待完整版本信息后再判断。' },
              ]}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        {capabilityCards.map((item) => (
          <Col xs={24} md={12} xl={6} key={item.title}>
            <Card className="clean-card about-capability-card">
              <div className="about-capability-card__icon" style={{ '--capability-color': item.color } as CSSProperties}>{item.icon}</div>
              <Typography.Title level={4}>{item.title}</Typography.Title>
              <Typography.Paragraph type="secondary">{item.desc}</Typography.Paragraph>
            </Card>
          </Col>
        ))}
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={10}>
          <Card className="clean-card about-card" title="生态链接">
            <div className="about-link-grid">
              {ecosystemLinks.map((item) => (
                <a key={item.href} href={item.href} target="_blank" rel="noreferrer">
                  {item.icon}<span>{item.label}</span><LinkOutlined />
                </a>
              ))}
            </div>
          </Card>
        </Col>
        <Col xs={24} lg={14}>
          <Card className="clean-card about-card" title="为什么这个项目存在">
            <div className="about-principle-grid">
              <div><BranchesOutlined /><strong>统一编排</strong><span>Cron、API、Broadcast、Workflow、Script、Plugin 和 SDK Job 共享同一实例证据模型。</span></div>
              <div><ApiOutlined /><strong>出站执行</strong><span>Worker 主动连接 Server Tunnel，业务网络不需要暴露任意执行端口。</span></div>
              <div><SafetyCertificateOutlined /><strong>治理优先</strong><span>RBAC、API-Key、脚本审批、通知投递和审计记录围绕生产交接设计。</span></div>
              <div><RocketOutlined /><strong>云原生交付</strong><span>Docker、Compose、Helm、Kubernetes、Terraform、OpenAPI 和多语言 SDK 都是一等发布面。</span></div>
            </div>
          </Card>
        </Col>
      </Row>

      {!latestRelease && !loading && !releaseError ? <Empty description="暂无 GitHub release 信息" /> : null}
    </div>
  );
}
