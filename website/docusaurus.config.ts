import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Vindue',
  tagline: 'Grid window tiling for macOS — drivable by scripts and AI',
  favicon: 'img/icon-64.png',

  future: {
    v4: true,
  },

  // GitHub Pages project site. When the custom domain lands: url →
  // 'https://vindue.app', baseUrl → '/', and add website/static/CNAME
  // (a file containing the domain).
  url: 'https://msukmanowsky.github.io',
  baseUrl: '/vindue/',

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
    colorMode: {
      defaultMode: 'dark',
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'Vindue',
      logo: {
        alt: 'Vindue icon',
        src: 'img/icon-64.png',
        width: 24,
        height: 24,
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docs',
          position: 'left',
          label: 'Docs',
        },
        {
          type: 'doc',
          docId: 'reference/roadmap',
          position: 'left',
          label: 'Roadmap',
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
            {label: 'MCP (AI control)', to: '/docs/reference/mcp'},
            {label: 'Roadmap', to: '/docs/reference/roadmap'},
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
