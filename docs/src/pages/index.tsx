import type {ReactNode} from 'react';
import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import Heading from '@theme/Heading';
import useBaseUrl from '@docusaurus/useBaseUrl';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';

import styles from './index.module.css';

type Card = {title: string; body: string; to: string};
type LocaleCopy = {
  title: string;
  description: string;
  eyebrow: string;
  headline: string;
  subtitle: string;
  primaryCta: string;
  deployCta: string;
  integrateCta: string;
  logoAlt: string;
  logoCaption: string;
  pathTitle: string;
  pathBody: string;
  architectureAlt: string;
  paths: Card[];
  manuals: Card[];
  quickTitle: string;
  quickBody: string;
  quickEyebrow: string;
  manualsTitle: string;
  manualsBody: string;
};

const copy: Record<'en' | 'zh-CN', LocaleCopy> = {
  en: {
    title: 'Tikeo operator manual',
    description: 'Human-readable Tikeo manuals for deployment, SDK integration, configuration, development, and operations.',
    eyebrow: 'Tikeo documentation',
    headline: 'A control plane for scheduled work, workflow execution, and outbound Workers.',
    subtitle: 'Use these docs to evaluate Tikeo locally, deploy a production topology, connect SDK Workers, configure notifications, and operate with auditable execution evidence.',
    primaryCta: 'Start locally',
    deployCta: 'Plan production',
    integrateCta: 'Connect SDKs',
    logoAlt: 'Tikeo task orchestration logo',
    logoCaption: 'Server · Worker Tunnel · SDKs · Notifications · Audit trail',
    pathTitle: 'Start from the work you need to complete',
    pathBody: 'The documentation is organized around operational outcomes: evaluating the platform, deploying it, integrating Workers, and tuning configuration with exact defaults when needed.',
    architectureAlt: 'Tikeo architecture diagram',
    quickTitle: 'Bring up a local control plane and collect execution evidence.',
    quickBody: 'Start the Server and Web console, bootstrap the Owner account, issue app-scoped credentials, connect a Worker through the outbound tunnel, trigger a job, and review the resulting instance logs.',
    quickEyebrow: 'Verification path',
    manualsTitle: 'Reference manuals',
    manualsBody: 'Detailed guides for capabilities that usually need policy, integration, or production-readiness decisions.',
    paths: [
      {title: 'Evaluate locally', body: 'Install the toolchain, start Server and Web, connect one outbound Worker, and run the Management API trigger smoke.', to: '/docs/getting-started/quickstart'},
      {title: 'Deploy and operate', body: 'Choose a binary, Compose, or Kubernetes path; configure storage, TLS, Worker Tunnel networking, backups, and rollout checks.', to: '/docs/deployment/production'},
      {title: 'Integrate applications', body: 'Connect Rust, Go, Java, Python, or Node.js Workers and use app-scoped Management API keys for controlled job creation and triggering.', to: '/docs/integrations/sdk-and-api'},
      {title: 'Tune configuration', body: 'Use scenario recipes for local development, databases, TLS/mTLS, OIDC, observability, notifications, and Worker defaults.', to: '/docs/reference/configuration-cookbook'},
    ],
    manuals: [
      {title: 'Notification Center', body: 'Channels, templates, policies, task-status bindings, delivery attempts, message trace, and provider redaction.', to: '/docs/user-guide/notifications'},
      {title: 'SDK manuals', body: 'Rust, Go, Java/Spring Boot, Python, and Node.js dependency coordinates, minimal Workers, and Management clients.', to: '/docs/sdks/rust'},
      {title: 'Development and extension', body: 'Repository map, API/Web/SDK change workflows, provider extension boundaries, and release readiness.', to: '/docs/development/overview'},
      {title: 'Troubleshooting', body: 'Use symptoms, logs, Worker state, storage readiness, proxy behavior, and smoke reports to diagnose failures.', to: '/docs/reference/troubleshooting'},
    ],
  },
  'zh-CN': {
    title: 'Tikeo 运维手册',
    description: '面向人的 Tikeo 文档：部署、SDK 集成、配置、开发扩展与运维验收。',
    eyebrow: 'Tikeo 文档',
    headline: '面向计划任务、工作流与出站 Worker 的执行控制平面。',
    subtitle: '从本地评估到生产部署，系统说明如何接入 SDK Worker、配置通知与权限、治理脚本执行，并基于实例日志和审计记录完成验收。',
    primaryCta: '本地启动',
    deployCta: '规划生产部署',
    integrateCta: '接入 SDK',
    logoAlt: 'Tikeo 任务编排标志',
    logoCaption: 'Server · Worker Tunnel · SDK · 通知 · 审计链路',
    pathTitle: '按交付目标选择阅读路径',
    pathBody: '文档围绕实际运维结果组织：评估平台、完成部署、接入 Worker、调优配置，并在需要时进入参考页核对默认值与 API 细节。',
    architectureAlt: 'Tikeo 架构图',
    quickTitle: '启动本地控制平面，并产出可核验的执行记录。',
    quickBody: '启动 Server 与 Web 控制台，初始化 Owner，签发应用级凭证，让 Worker 通过出站隧道连接，触发任务后检查实例日志与状态流转。',
    quickEyebrow: '验收路径',
    manualsTitle: '能力手册',
    manualsBody: '面向策略、集成和生产可用性决策的详细说明。',
    paths: [
      {title: '本地评估', body: '安装工具链，启动 Server 与 Web，连接一个出站 Worker，并运行 Management API 触发链路 smoke。', to: '/docs/getting-started/quickstart'},
      {title: '部署与运维', body: '选择单二进制、Compose 或 Kubernetes 路径；配置存储、TLS、Worker Tunnel 网络、备份和发布检查。', to: '/docs/deployment/production'},
      {title: '接入应用', body: '接入 Rust、Go、Java、Python 或 Node.js Worker，并用应用级 Management API Key 控制作业创建与触发。', to: '/docs/integrations/sdk-and-api'},
      {title: '调优配置', body: '按场景核对本地开发、数据库、TLS/mTLS、OIDC、观测、通知和 Worker 默认值。', to: '/docs/reference/configuration-cookbook'},
    ],
    manuals: [
      {title: '通知中心', body: '渠道、模板、策略、任务状态绑定、投递 attempt、消息 trace 和 provider 脱敏。', to: '/docs/user-guide/notifications'},
      {title: 'SDK 手册', body: 'Rust、Go、Java/Spring Boot、Python、Node.js 依赖坐标、最小 Worker 和 Management client。', to: '/docs/sdks/rust'},
      {title: '开发与扩展', body: '仓库地图、API/Web/SDK 变更流程、provider 扩展边界和发布准备。', to: '/docs/development/overview'},
      {title: '故障排查', body: '用现象、日志、Worker 状态、存储 readiness、代理行为和 smoke 报告诊断故障。', to: '/docs/reference/troubleshooting'},
    ],
  },
};

function CardGrid({items}: {items: Card[]}): ReactNode {
  return (
    <div className={styles.cards}>
      {items.map((item) => (
        <Link className={styles.cardLink} to={item.to} key={item.title}>
          <article className={styles.card}>
            <h3>{item.title}</h3>
            <p>{item.body}</p>
          </article>
        </Link>
      ))}
    </div>
  );
}

export default function Home(): ReactNode {
  const {i18n} = useDocusaurusContext();
  const localeCopy = i18n.currentLocale === 'zh-CN' ? copy['zh-CN'] : copy.en;
  const localeSuffix = i18n.currentLocale === 'zh-CN' ? 'zh-CN' : 'en';
  const logoUrl = useBaseUrl('/img/tikeo-logo.svg');
  const architectureUrl = useBaseUrl(`/img/tikeo-architecture.${localeSuffix}.svg`);

  return (
    <Layout title={localeCopy.title} description={localeCopy.description}>
      <main>
        <section className={styles.hero}>
          <div className="container">
            <div className={styles.heroGrid}>
              <div>
                <p className={styles.eyebrow}>{localeCopy.eyebrow}</p>
                <Heading as="h1" className={styles.title}>
                  {localeCopy.headline}
                </Heading>
                <p className={styles.subtitle}>{localeCopy.subtitle}</p>
                <div className={styles.actions}>
                  <Link className="button button--primary button--lg" to="/docs/getting-started/quickstart">
                    {localeCopy.primaryCta}
                  </Link>
                  <Link className="button button--secondary button--lg" to="/docs/deployment/production">
                    {localeCopy.deployCta}
                  </Link>
                  <Link className="button button--outline button--lg" to="/docs/integrations/sdk-and-api">
                    {localeCopy.integrateCta}
                  </Link>
                </div>
              </div>
              <div className={styles.logoCard} aria-label={localeCopy.logoAlt}>
                <img className={styles.heroLogo} src={logoUrl} alt={localeCopy.logoAlt} />
                <span>{localeCopy.logoCaption}</span>
              </div>
            </div>
          </div>
        </section>
        <section className="container margin-vert--xl">
          <div className={styles.sectionHeader}>
            <p className={styles.eyebrow}>{localeCopy.pathTitle}</p>
            <p>{localeCopy.pathBody}</p>
          </div>
          <CardGrid items={localeCopy.paths} />
        </section>
        <section className="container margin-vert--xl">
          <div className={styles.quickstart}>
            <div>
              <p className={styles.eyebrow}>{localeCopy.quickEyebrow}</p>
              <h2>{localeCopy.quickTitle}</h2>
              <p>{localeCopy.quickBody}</p>
            </div>
            <pre><code>{`cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://127.0.0.1:9090/readyz
TIKEO_MANAGEMENT_TRIGGER_REBUILD_SERVER=0 scripts/management-trigger-e2e-smoke.sh`}</code></pre>
          </div>
        </section>
        <section className="container margin-vert--xl">
          <div className={styles.sectionHeader}>
            <p className={styles.eyebrow}>{localeCopy.manualsTitle}</p>
            <p>{localeCopy.manualsBody}</p>
          </div>
          <CardGrid items={localeCopy.manuals} />
        </section>
        <section className="container margin-vert--xl">
          <img className={styles.architecture} src={architectureUrl} alt={localeCopy.architectureAlt} />
        </section>
      </main>
    </Layout>
  );
}
