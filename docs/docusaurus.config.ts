import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

declare const process: {
  env: Record<string, string | undefined>;
};

const normalizeBaseUrl = (value: string): string => {
  const withLeadingSlash = value.startsWith('/') ? value : `/${value}`;
  return withLeadingSlash.endsWith('/') ? withLeadingSlash : `${withLeadingSlash}/`;
};

const siteUrl = process.env.TIKEO_DOCS_URL ?? 'https://tikeo.dev';
const baseUrl = normalizeBaseUrl(process.env.TIKEO_DOCS_BASE_URL ?? '/');

const config: Config = {
  title: 'Tikeo',
  tagline: 'Rust-native orchestration for jobs, workflows, workers, and governed scripts.',
  favicon: 'img/tikeo-logo.svg',

  future: {
    v4: true,
  },

  // Override TIKEO_DOCS_URL/TIKEO_DOCS_BASE_URL for GitHub Pages project hosting or custom domains.
  // The default is a standalone docs domain rooted at / so /zh-CN/ works without extra hosting rewrites.
  url: siteUrl,
  baseUrl,

  headTags: [
    {
      tagName: 'meta',
      attributes: {name: 'description', content: 'Tikeo documentation for Rust-native distributed task scheduling, Worker Tunnel operations, SDKs, deployment, and governance.'},
    },
    {
      tagName: 'meta',
      attributes: {property: 'og:title', content: 'Tikeo documentation'},
    },
    {
      tagName: 'meta',
      attributes: {property: 'og:image', content: `${siteUrl}${baseUrl}img/tikeo-og.png`},
    },
    {
      tagName: 'meta',
      attributes: {name: 'twitter:card', content: 'summary_large_image'},
    },
  ],

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
          editUrl: 'https://github.com/yhyzgn/tikeo/tree/main/docs/',
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
        sitemap: {
          changefreq: 'weekly',
          priority: 0.7,
          filename: 'sitemap.xml',
        },
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    image: 'img/tikeo-og.png',
    colorMode: {
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'Tikeo',
      logo: {
        alt: 'Tikeo logo',
        src: 'img/tikeo-logo.svg',
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
        {to: '/search', label: 'Search', position: 'left'},
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
