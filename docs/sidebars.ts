import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    'index',
    {
      type: 'category',
      label: 'Getting Started',
      items: [
        'getting-started/installation',
        'getting-started/quickstart',
        'getting-started/seed-demo-data',
      ],
    },
    {
      type: 'category',
      label: 'Core Concepts',
      items: [
        'concepts/worker-tunnel',
        'concepts/workflows',
      ],
    },
    {
      type: 'category',
      label: 'User Guide',
      items: [
        'user-guide/dashboard',
        'user-guide/jobs',
        'user-guide/instances',
        'user-guide/workers',
        'user-guide/workflows',
        'user-guide/notifications',
        'user-guide/alerts',
        'user-guide/scripts',
        'user-guide/security-policy-center',
        'user-guide/audit',
        'user-guide/settings',
      ],
    },
    {
      type: 'category',
      label: 'Deployment & Operations',
      link: {
        type: 'generated-index',
        title: 'Deployment & Operations',
        description:
          'Production deployment, Kubernetes, Server HA, SSE, controller-specific runbooks, and operations checks.',
      },
      items: [
        'deployment/production',
        'deployment/single-binary',
        'deployment/docker-compose',
        'deployment/kubernetes',
        'deployment/server-ha',
        'deployment/kubernetes-controller-runbook',
        'deployment/management-trigger-smoke-runbook',
        'deployment/sse-realtime',
      ],
    },
    {
      type: 'category',
      label: 'SDKs & API Integrations',
      items: [
        'integrations/sdk-and-api',
        'integrations/overview',
        'sdks/rust',
        'sdks/go',
        'sdks/java-spring-boot',
        'sdks/python',
        'sdks/nodejs',
      ],
    },
    {
      type: 'category',
      label: 'Develop and extend',
      items: [
        'development/overview',
        'development/script-support',
        'development/plugin-development',
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      items: [
        'reference/configuration',
        'reference/configuration-cookbook',
        'reference/management-openapi',
        'reference/notification-center',
        'reference/worker-tunnel-protobuf',
        'reference/troubleshooting',
      ],
    },
  ],
};

export default sidebars;
