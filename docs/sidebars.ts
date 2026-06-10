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
        'user-guide/scripts',
        'user-guide/audit',
        'user-guide/settings',
      ],
    },
    {
      type: 'category',
      label: 'SDKs',
      items: [
        'sdks/rust',
        'sdks/go',
        'sdks/java-spring-boot',
        'sdks/python',
        'sdks/nodejs',
      ],
    },
    {
      type: 'category',
      label: 'Deployment',
      items: [
        'deployment/single-binary',
        'deployment/docker-compose',
        'deployment/kubernetes',
        'deployment/sse-realtime',
      ],
    },
    {
      type: 'category',
      label: 'Integrations',
      items: [
        'integrations/overview',
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      items: [
        'reference/configuration',
        'reference/management-openapi',
        'reference/worker-tunnel-protobuf',
        'reference/troubleshooting',
      ],
    },
  ],
};

export default sidebars;
