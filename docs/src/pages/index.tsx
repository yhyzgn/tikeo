import type {ReactNode} from 'react';
import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import Heading from '@theme/Heading';
import useBaseUrl from '@docusaurus/useBaseUrl';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';

import styles from './index.module.css';

type LocaleCopy = {
  title: string;
  description: string;
  eyebrow: string;
  headline: string;
  subtitle: string;
  primaryCta: string;
  architectureCta: string;
  githubCta: string;
  logoAlt: string;
  logoCaption: string;
  quickEyebrow: string;
  quickTitle: string;
  quickBody: string;
  architectureAlt: string;
  capabilities: Array<[string, string]>;
};

const copy: Record<'en' | 'zh-CN', LocaleCopy> = {
  en: {
    title: 'Rust-native orchestration for jobs and workflows',
    description: 'Tikeo documentation for jobs, workflows, worker tunnels, multi-language SDKs, and governed scripts.',
    eyebrow: 'Tikeo documentation',
    headline: 'Rust-native orchestration for jobs, workflows, workers, and governed scripts.',
    subtitle: 'No exposed worker ports. Multi-language workers. Workflow canvas. Audit-ready execution evidence.',
    primaryCta: 'Get started',
    architectureCta: 'View architecture',
    githubCta: 'GitHub',
    logoAlt: 'Tikeo breathing task-flow logo',
    logoCaption: 'Worker Tunnel · DAG · SDKs · Audit',
    quickEyebrow: '10-minute evaluation path',
    quickTitle: 'Run Server, Web, and one Worker demo from verified repo commands.',
    quickBody: 'Start locally, inspect health endpoints, then connect a Rust, Go, Java, Python, or Node.js worker to the Worker Tunnel.',
    architectureAlt: 'Tikeo architecture diagram',
    capabilities: [
      ['No inbound worker ports', 'Workers dial out through the gRPC/HTTP2 Worker Tunnel, making cross-VPC and Kubernetes deployments practical.'],
      ['Workflow DAG canvas', 'Model scheduled jobs, API triggers, Map/MapReduce, retries, and replayable execution evidence.'],
      ['Multi-language workers', 'Use Rust, Go, Java Spring Boot, Python, and Node.js worker docs from verified SDK/demo entry points.'],
      ['Governed scripts', 'Approval, signature, sandbox, audit, and alerting boundaries are documented as first-class operations features.'],
    ],
  },
  'zh-CN': {
    title: 'Rust 原生任务与工作流编排',
    description: 'Tikeo 文档：任务、工作流、Worker Tunnel、多语言 SDK 与受治理脚本。',
    eyebrow: 'Tikeo 文档',
    headline: '面向任务、工作流、Worker 与受治理脚本的 Rust 原生编排平台。',
    subtitle: '无需暴露 Worker 入站端口。多语言 Worker。工作流画布。可审计执行证据。',
    primaryCta: '快速开始',
    architectureCta: '查看架构',
    githubCta: 'GitHub',
    logoAlt: 'Tikeo 呼吸式任务流标志',
    logoCaption: 'Worker Tunnel · DAG · SDK · 审计',
    quickEyebrow: '10 分钟评估路径',
    quickTitle: '用仓库中已验证的命令启动 Server、Web 和一个 Worker demo。',
    quickBody: '先本地启动服务并检查健康端点，再连接 Rust、Go、Java、Python 或 Node.js Worker 到 Worker Tunnel。',
    architectureAlt: 'Tikeo 架构图',
    capabilities: [
      ['Worker 无需入站端口', 'Worker 主动通过 gRPC/HTTP2 Worker Tunnel 连接 Server，更适合跨 VPC 与 Kubernetes 部署。'],
      ['工作流 DAG 画布', '建模定时任务、API 触发、Map/MapReduce、重试与可回放执行证据。'],
      ['多语言 Worker', 'Rust、Go、Java Spring Boot、Python、Node.js 文档都来自已验证 SDK/demo 入口。'],
      ['受治理脚本', '审批、签名、沙箱、审计与告警边界作为一等运维能力记录。'],
    ],
  },
};

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
                  <Link className="button button--secondary button--lg" to="/docs/concepts/worker-tunnel">
                    {localeCopy.architectureCta}
                  </Link>
                  <Link className="button button--outline button--lg" to="https://github.com/yhyzgn/tikeo">
                    {localeCopy.githubCta}
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
          <div className={styles.cards}>
            {localeCopy.capabilities.map(([title, body]) => (
              <article className={styles.card} key={title}>
                <h2>{title}</h2>
                <p>{body}</p>
              </article>
            ))}
          </div>
        </section>
        <section className="container margin-vert--xl">
          <div className={styles.quickstart}>
            <div>
              <p className={styles.eyebrow}>{localeCopy.quickEyebrow}</p>
              <h2>{localeCopy.quickTitle}</h2>
              <p>{localeCopy.quickBody}</p>
            </div>
            <pre><code>{`cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
(cd examples/rust/worker-demo && cargo run)`}</code></pre>
          </div>
        </section>
        <section className="container margin-vert--xl">
          <img className={styles.architecture} src={architectureUrl} alt={localeCopy.architectureAlt} />
        </section>
      </main>
    </Layout>
  );
}
