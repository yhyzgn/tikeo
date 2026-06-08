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
      label: 'SDKs',
      items: [
        'sdks/rust',
        'sdks/go',
        'sdks/java-spring-boot',
      ],
    },
    {
      type: 'category',
      label: 'Deployment',
      items: [
        'deployment/docker-compose',
        'deployment/kubernetes',
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
        'reference/troubleshooting',
      ],
    },
  ],
};

export default sidebars;
