import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Tikeo',
  tagline: 'Rust-native orchestration for jobs, workflows, workers, and governed scripts.',
  favicon: 'img/tikeo-logo-breathe.gif',

  future: {
    v4: true,
  },

  // Replace this with the final public docs domain when deployment is chosen.
  url: 'https://tikeo.dev',
  baseUrl: '/',

  organizationName: 'yhyzgn',
  projectName: 'tikeo',

  onBrokenLinks: 'throw',
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en', 'zh-CN'],
    localeConfigs: {
      en: {label: 'English'},
      'zh-CN': {label: '简体中文'},
    },
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          routeBasePath: 'docs',
          editUrl: 'https://github.com/yhyzgn/tikeo/tree/main/website/',
        },
        blog: {
          showReadingTime: true,
          routeBasePath: 'releases',
          blogTitle: 'Tikeo Releases',
          blogDescription: 'Release notes and project updates for Tikeo.',
          onInlineTags: 'warn',
          onInlineAuthors: 'warn',
          onUntruncatedBlogPosts: 'warn',
        },
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    image: 'img/tikeo-architecture.en.svg',
    colorMode: {
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'Tikeo',
      logo: {
        alt: 'Tikeo logo',
        src: 'img/tikeo-logo-breathe.gif',
      },
      items: [
        {to: '/', label: 'Home', position: 'left'},
        {
          type: 'docSidebar',
          sidebarId: 'docsSidebar',
          position: 'left',
          label: 'Docs',
        },
        {to: '/docs/sdks/rust', label: 'SDKs', position: 'left'},
        {to: '/docs/integrations/overview', label: 'Integrations', position: 'left'},
        {to: '/releases', label: 'Blog / Releases', position: 'left'},
        {
          href: 'https://github.com/yhyzgn/tikeo',
          label: 'GitHub',
          position: 'right',
        },
        {
          type: 'localeDropdown',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            {label: 'Overview', to: '/docs/'},
            {label: 'Quickstart', to: '/docs/getting-started/quickstart'},
            {label: 'Configuration', to: '/docs/reference/configuration'},
          ],
        },
        {
          title: 'Build',
          items: [
            {label: 'SDKs', to: '/docs/sdks/rust'},
            {label: 'Deployment', to: '/docs/deployment/docker-compose'},
            {label: 'Troubleshooting', to: '/docs/reference/troubleshooting'},
          ],
        },
        {
          title: 'Project',
          items: [
            {label: 'GitHub', href: 'https://github.com/yhyzgn/tikeo'},
            {label: 'Security', href: 'https://github.com/yhyzgn/tikeo/security'},
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Tikeo maintainers. Built with Docusaurus.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
