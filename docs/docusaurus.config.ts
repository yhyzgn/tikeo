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
const absoluteUrl = (path: string): string => `${siteUrl}${baseUrl}${path.replace(/^\//, '')}`;

const seo = {
  siteName: 'Tikeo Documentation',
  description:
    'Tikeo documentation for distributed task scheduling, workflow orchestration, outbound Worker Tunnel operations, SDK integration, Docker deployment, Kubernetes, notifications, RBAC, OpenTelemetry, and governed scripts.',
  keywords: [
    'Tikeo',
    'task orchestration',
    'distributed task scheduler',
    'job scheduler',
    'workflow orchestration',
    'workflow engine',
    'Worker Tunnel',
    'outbound workers',
    'multi-language SDK',
    'Rust scheduler',
    'Kubernetes operator',
    'Docker Compose deployment',
    'OpenTelemetry',
    'script sandbox',
    'RBAC',
    'notification center',
    'XXL-Job alternative',
    'PowerJob alternative',
  ],
};

const organizationJsonLd = {
  '@context': 'https://schema.org',
  '@type': 'Organization',
  name: 'Tikeo',
  url: siteUrl,
  logo: absoluteUrl('/img/tikeo-logo.svg'),
  sameAs: ['https://github.com/yhyzgn/tikeo'],
};

const softwareJsonLd = {
  '@context': 'https://schema.org',
  '@type': 'SoftwareApplication',
  name: 'Tikeo',
  applicationCategory: 'DeveloperApplication',
  operatingSystem: 'Linux, macOS, Windows, Docker, Kubernetes',
  description: seo.description,
  image: absoluteUrl('/img/tikeo-og.png'),
  url: siteUrl,
  codeRepository: 'https://github.com/yhyzgn/tikeo',
  license: 'https://github.com/yhyzgn/tikeo/blob/main/LICENSE',
  programmingLanguage: ['Rust', 'TypeScript', 'Java', 'Go', 'Python', 'Node.js'],
  offers: {
    '@type': 'Offer',
    price: '0',
    priceCurrency: 'USD',
  },
};

const websiteJsonLd = {
  '@context': 'https://schema.org',
  '@type': 'WebSite',
  name: seo.siteName,
  url: siteUrl,
  inLanguage: ['en', 'zh-CN'],
  description: seo.description,
  potentialAction: {
    '@type': 'SearchAction',
    target: `${siteUrl}${baseUrl}search?q={search_term_string}`,
    'query-input': 'required name=search_term_string',
  },
};

const config: Config = {
  title: 'Tikeo',
  tagline: 'Rust-native orchestration for jobs, workflows, workers, and governed scripts.',
  titleDelimiter: '·',
  favicon: 'img/tikeo-logo.svg',

  future: {
    v4: true,
  },

  // Override TIKEO_DOCS_URL/TIKEO_DOCS_BASE_URL for GitHub Pages project hosting or custom domains.
  // The default is a standalone docs domain rooted at / so /zh-CN/ works without extra hosting rewrites.
  url: siteUrl,
  baseUrl,

  headTags: [
    {tagName: 'meta', attributes: {name: 'description', content: seo.description}},
    {tagName: 'meta', attributes: {name: 'keywords', content: seo.keywords.join(', ')}},
    {tagName: 'meta', attributes: {name: 'author', content: 'Tikeo maintainers'}},
    {tagName: 'meta', attributes: {name: 'application-name', content: 'Tikeo'}},
    {tagName: 'meta', attributes: {name: 'robots', content: 'index,follow,max-image-preview:large,max-snippet:-1,max-video-preview:-1'}},
    {tagName: 'meta', attributes: {name: 'googlebot', content: 'index,follow,max-image-preview:large,max-snippet:-1,max-video-preview:-1'}},
    {tagName: 'meta', attributes: {name: 'theme-color', content: '#3157d5'}},
    {tagName: 'meta', attributes: {property: 'og:site_name', content: seo.siteName}},
    {tagName: 'meta', attributes: {property: 'og:type', content: 'website'}},
    {tagName: 'meta', attributes: {property: 'og:title', content: 'Tikeo documentation'}},
    {tagName: 'meta', attributes: {property: 'og:description', content: seo.description}},
    {tagName: 'meta', attributes: {property: 'og:image', content: absoluteUrl('/img/tikeo-og.png')}},
    {tagName: 'meta', attributes: {property: 'og:image:alt', content: 'Tikeo task orchestration documentation preview'}},
    {tagName: 'meta', attributes: {property: 'og:url', content: siteUrl}},
    {tagName: 'meta', attributes: {name: 'twitter:card', content: 'summary_large_image'}},
    {tagName: 'meta', attributes: {name: 'twitter:title', content: 'Tikeo documentation'}},
    {tagName: 'meta', attributes: {name: 'twitter:description', content: seo.description}},
    {tagName: 'meta', attributes: {name: 'twitter:image', content: absoluteUrl('/img/tikeo-og.png')}},
    {tagName: 'link', attributes: {rel: 'manifest', href: absoluteUrl('/site.webmanifest')}},
    {tagName: 'link', attributes: {rel: 'search', type: 'application/opensearchdescription+xml', title: 'Tikeo Docs', href: absoluteUrl('/opensearch.xml')}},
    {tagName: 'link', attributes: {rel: 'sitemap', type: 'application/xml', href: absoluteUrl('/sitemap.xml')}},
    {tagName: 'script', attributes: {type: 'application/ld+json'}, innerHTML: JSON.stringify(organizationJsonLd)},
    {tagName: 'script', attributes: {type: 'application/ld+json'}, innerHTML: JSON.stringify(softwareJsonLd)},
    {tagName: 'script', attributes: {type: 'application/ld+json'}, innerHTML: JSON.stringify(websiteJsonLd)},
  ],

  organizationName: 'yhyzgn',
  projectName: 'tikeo',

  onBrokenLinks: 'throw',
  markdown: {
    mermaid: true,
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

  themes: ['@docusaurus/theme-mermaid'],

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
      additionalLanguages: [
        'bash',
        'docker',
        'go',
        'java',
        'json',
        'kotlin',
        'nginx',
        'properties',
        'protobuf',
        'python',
        'rust',
        'toml',
        'typescript',
        'yaml',
      ],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
