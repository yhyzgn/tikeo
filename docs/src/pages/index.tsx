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
    headline: 'Deploy it, connect Workers, configure it, and prove it works.',
    subtitle: 'A human operator manual for Tikeo: local quickstart, production deployment, SDK/API integration, Notification Center, configuration recipes, and development boundaries.',
    primaryCta: 'Run the quickstart',
    deployCta: 'Deploy production',
    integrateCta: 'Integrate an app',
    logoAlt: 'Tikeo breathing task-flow logo',
    logoCaption: 'Server · Worker Tunnel · SDKs · Notifications · Audit',
    pathTitle: 'Choose the path that matches your job',
    pathBody: 'The docs are organized by what you need to accomplish, not by source-code directory. Start with the closest role, then use the reference pages only when you need exact defaults or API details.',
    architectureAlt: 'Tikeo architecture diagram',
    quickTitle: 'A real first proof should produce evidence, not only a running process.',
    quickBody: 'Start Server, bootstrap Owner, create app-scoped credentials, connect a Worker outbound, trigger a job through the SDK Management API, and inspect instance logs.',
    quickEyebrow: 'Evidence path',
    manualsTitle: 'Manuals',
    manualsBody: 'Common deep manuals.',
    paths: [
      {title: 'Evaluate locally', body: 'Install tools, start Server/Web, connect one Worker, and run the management trigger smoke.', to: '/docs/getting-started/quickstart'},
      {title: 'Deploy and operate', body: 'Choose Compose, single binary, or Helm; configure database, TLS, Worker Tunnel, backups, and rollout checks.', to: '/docs/deployment/production'},
      {title: 'Integrate an application', body: 'Use SDK Worker clients and app-scoped Management API keys to create and trigger jobs safely.', to: '/docs/integrations/sdk-and-api'},
      {title: 'Configure by scenario', body: 'Copy recipes for local dev, PostgreSQL, MySQL, TLS/mTLS, OIDC, observability, notifications, and Worker defaults.', to: '/docs/reference/configuration-cookbook'},
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
    headline: '照着部署、接入 Worker、配置系统，并证明它真的可用。',
    subtitle: '这是 Tikeo 的人类运维手册：本地快速开始、生产部署、SDK/API 集成、通知中心、配置 recipe 和开发边界。',
    primaryCta: '运行快速开始',
    deployCta: '生产部署',
    integrateCta: '接入应用',
    logoAlt: 'Tikeo 呼吸式任务流标志',
    logoCaption: 'Server · Worker Tunnel · SDK · 通知 · 审计',
    pathTitle: '按你的任务选择阅读路径',
    pathBody: '文档按“你要完成什么”组织，而不是按源码目录堆叠。先选择最接近的角色路径，需要精确默认值或 API 细节时再进入参考页。',
    architectureAlt: 'Tikeo 架构图',
    quickTitle: '第一次验收应该产出证据，而不只是一个进程。',
    quickBody: '启动 Server，初始化 Owner，创建应用级凭证，让 Worker 主动出站连接，通过 SDK Management API 触发任务，并检查实例日志。',
    quickEyebrow: '证据路径',
    manualsTitle: '深入手册',
    manualsBody: '常用深入手册。',
    paths: [
      {title: '本地评估', body: '安装工具、启动 Server/Web、连接一个 Worker，并运行 management trigger smoke。', to: '/docs/getting-started/quickstart'},
      {title: '部署与运维', body: '选择 Compose、单二进制或 Helm；配置数据库、TLS、Worker Tunnel、备份和发布检查。', to: '/docs/deployment/production'},
      {title: '接入应用', body: '用 SDK Worker 客户端和应用级 Management API Key 安全创建与触发任务。', to: '/docs/integrations/sdk-and-api'},
      {title: '按场景配置', body: '复制本地开发、PostgreSQL、MySQL、TLS/mTLS、OIDC、观测、通知和 Worker 默认值 recipe。', to: '/docs/reference/configuration-cookbook'},
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
  const logoUrl = useBaseUrl('/img/tikeo-logo-breathe.gif');
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
                <img src={logoUrl} alt={localeCopy.logoAlt} />
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
