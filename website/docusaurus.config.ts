import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Vindue',
  tagline: 'Grid window tiling for macOS — drivable by scripts and AI',
  favicon: 'img/favicon.png',

  future: {
    v4: true,
  },

  // Custom domain on GitHub Pages (DNS: apex A/AAAA → GitHub's IPs, www →
  // CNAME msukmanowsky.github.io; the domain itself is declared in repo
  // Settings → Pages — for Actions-published sites no CNAME file is used).
  // GitHub 301s the old msukmanowsky.github.io/vindue/* paths here, prefix
  // stripped.
  url: 'https://vindue.app',
  baseUrl: '/',

  organizationName: 'msukmanowsky',
  projectName: 'vindue',

  onBrokenLinks: 'throw',
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl: 'https://github.com/msukmanowsky/vindue/tree/main/website/',
          // *.tag.mdx are plugin-generated tag summaries (committed — the
          // drift gate covers them) but near-empty as pages; keep them out of
          // the built site so the sidebar lists only real content.
          exclude: [
            '**/_*.{js,jsx,ts,tsx,md,mdx}',
            '**/_*/**',
            '**/*.test.{js,ts}',
            '**/__tests__/**',
            '**/*.tag.mdx',
          ],
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  // Endpoint pages are GENERATED from the OpenAPI spec that the Rust code
  // produces (`npm run docs:all` at the repo root → spec + MDX; never
  // `gen-api-docs all` — upstream race bug, see AGENTS.md). Both generated
  // copies are committed and pinned by CI drift gates — see CONTRIBUTING.md.
  themes: ['docusaurus-theme-openapi-docs'],
  plugins: [
    [
      '@docusaurus/plugin-client-redirects',
      {
        // Pages promoted from Reference to top-level sidebar members.
        redirects: [
          {from: '/docs/reference/roadmap', to: '/docs/roadmap'},
          {from: '/docs/reference/contributing', to: '/docs/contributing'},
          {from: '/docs/reference/troubleshooting', to: '/docs/troubleshooting'},
          {from: '/docs/reference/architecture', to: '/docs/architecture'},
        ],
      },
    ],
    [
      'docusaurus-plugin-openapi-docs',
      {
        id: 'api',
        docsPluginId: 'default',
        config: {
          vindue: {
            specPath: 'docs/reference/generated/openapi.json',
            outputDir: 'docs/reference/http-api',
            sidebarOptions: {
              groupPathsBy: 'tag',
              categoryLinkSource: 'tag',
            },
          },
        },
      },
    ],
  ],

  themeConfig: {
    image: 'img/og.png',
    colorMode: {
      defaultMode: 'dark',
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'Vindue',
      logo: {
        alt: 'Vindue icon',
        src: 'img/logo.svg',
        width: 26,
        height: 26,
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docs',
          position: 'left',
          label: 'Docs',
        },
        {
          href: 'https://github.com/sponsors/msukmanowsky',
          label: 'Donate',
          position: 'right',
        },
        {
          href: 'https://github.com/msukmanowsky/vindue',
          label: 'GitHub',
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
            {label: 'Getting started', to: '/docs/getting-started/intro'},
            {label: 'HTTP API', to: '/docs/reference/http-api'},
            {label: 'MCP', to: '/docs/reference/mcp'},
            {label: 'Roadmap', to: '/docs/roadmap'},
          ],
        },
        {
          title: 'Project',
          items: [
            {
              label: 'GitHub',
              href: 'https://github.com/msukmanowsky/vindue',
            },
            {
              label: 'Releases',
              href: 'https://github.com/msukmanowsky/vindue/releases',
            },
            {
              label: 'Changelog',
              href: 'https://github.com/msukmanowsky/vindue/blob/main/CHANGELOG.md',
            },
          ],
        },
        {
          title: 'Support',
          items: [
            {
              label: 'Donate (GitHub Sponsors)',
              href: 'https://github.com/sponsors/msukmanowsky',
            },
            {
              label: 'Report a bug',
              href: 'https://github.com/msukmanowsky/vindue/issues/new/choose',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Mike Sukmanowsky · MIT licensed`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['bash', 'json'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
