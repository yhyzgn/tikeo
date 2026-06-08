import type {ReactNode} from 'react';
import Link from '@docusaurus/Link';
import Layout from '@theme/Layout';
import Heading from '@theme/Heading';

import styles from './index.module.css';

const capabilities = [
  ['No inbound worker ports', 'Workers dial out through the gRPC/HTTP2 Worker Tunnel, making cross-VPC and Kubernetes deployments practical.'],
  ['Workflow DAG canvas', 'Model scheduled jobs, API triggers, Map/MapReduce, retries, and replayable execution evidence.'],
  ['Multi-language workers', 'Use Rust, Go, and Java Spring Boot workers today, with Python and Node.js pages kept honest as the SDKs evolve.'],
  ['Governed scripts', 'Approval, signature, sandbox, audit, and alerting boundaries are documented as first-class operations features.'],
];

export default function Home(): ReactNode {
  return (
    <Layout
      title="Rust-native orchestration for jobs and workflows"
      description="Tikeo documentation for jobs, workflows, worker tunnels, multi-language SDKs, and governed scripts.">
      <main>
        <section className={styles.hero}>
          <div className="container">
            <div className={styles.heroGrid}>
              <div>
                <p className={styles.eyebrow}>Tikeo documentation</p>
                <Heading as="h1" className={styles.title}>
                  Rust-native orchestration for jobs, workflows, workers, and governed scripts.
                </Heading>
                <p className={styles.subtitle}>
                  No exposed worker ports. Multi-language workers. Workflow canvas. Audit-ready execution evidence.
                </p>
                <div className={styles.actions}>
                  <Link className="button button--primary button--lg" to="/docs/getting-started/quickstart">
                    Get started
                  </Link>
                  <Link className="button button--secondary button--lg" to="/docs/concepts/worker-tunnel">
                    View architecture
                  </Link>
                  <Link className="button button--outline button--lg" to="https://github.com/yhyzgn/tikeo">
                    GitHub
                  </Link>
                </div>
              </div>
              <div className={styles.logoCard} aria-label="Tikeo animated logo">
                <img src="/img/tikeo-logo-breathe.gif" alt="Tikeo breathing task-flow logo" />
                <span>Worker Tunnel · DAG · SDKs · Audit</span>
              </div>
            </div>
          </div>
        </section>
        <section className="container margin-vert--xl">
          <div className={styles.cards}>
            {capabilities.map(([title, body]) => (
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
              <p className={styles.eyebrow}>10-minute evaluation path</p>
              <h2>Run Server, Web, and one Worker demo from verified repo commands.</h2>
              <p>
                Start locally, inspect health endpoints, then connect a Rust, Go, or Java worker to the Worker Tunnel.
              </p>
            </div>
            <pre><code>{`cargo run --bin tikeo -- serve --config config/dev.toml
curl -fsS http://0.0.0.0:9090/healthz
(cd examples/rust/worker-demo && cargo run)`}</code></pre>
          </div>
        </section>
        <section className="container margin-vert--xl">
          <img className={styles.architecture} src="/img/tikeo-architecture.en.svg" alt="Tikeo architecture diagram" />
        </section>
      </main>
    </Layout>
  );
}
