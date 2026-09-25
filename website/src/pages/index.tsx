import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import HomepageFeatures from '@site/src/components/HomepageFeatures';
import Heading from '@theme/Heading';
import CodeBlock from '@theme/CodeBlock';

import styles from './index.module.css';

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={clsx('hero', styles.heroBanner)}>
      <div className="container">
        <Heading as="h1" className="hero__title">
          {siteConfig.title}
        </Heading>
        <p className="hero__subtitle">{siteConfig.tagline}</p>
        <div className={styles.buttons}>
          <Link
            className="button button--primary button--lg"
            href="https://github.com/msukmanowsky/vindue/releases/latest/download/Vindue_universal.dmg">
            Download for macOS
          </Link>
          <Link
            className="button button--secondary button--lg margin-left--md"
            to="/docs/getting-started/intro">
            Read the docs
          </Link>
        </div>
        <p className={styles.heroNote}>
          Free &amp; open source (MIT) · Apple Silicon + Intel · menu-bar app,
          no Dock icon · no telemetry, everything stays local
        </p>
        {/* TODO(launch assets): demo GIF — hotkey → panel → drag → snap */}
      </div>
    </header>
  );
}

function AutomationTeaser() {
  return (
    <section className={styles.automation}>
      <div className="container">
        <Heading as="h2" className={styles.automationTitle}>
          One loopback server, two protocols
        </Heading>
        <p className={styles.automationBlurb}>
          Vindue serves a control API on <code>127.0.0.1</code> — REST for
          scripts and humans, and an MCP endpoint so AI clients can drive your
          windows. No auth to manage: the server rejects anything a browser
          could send, so web pages can never reach it.
        </p>
        <div className="row">
          <div className="col col--6">
            <Heading as="h3">Script it</Heading>
            <CodeBlock language="bash">
              {`curl -s -X POST 127.0.0.1:47725/api/v1/tile \\
  -d '{"preset":"left_half","app":"Safari"}'`}
            </CodeBlock>
          </div>
          <div className="col col--6">
            <Heading as="h3">Or let your AI drive</Heading>
            <CodeBlock language="bash">
              {`claude mcp add --transport http vindue \\
  http://127.0.0.1:47725/mcp`}
            </CodeBlock>
          </div>
        </div>
      </div>
    </section>
  );
}

export default function Home(): React.ReactNode {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout
      title="Grid window tiling for macOS"
      description="Vindue is an open-source grid window tiler for macOS with a built-in HTTP API and MCP server, so scripts and AI clients can drive window management.">
      <HomepageHeader />
      <main>
        <AutomationTeaser />
        <HomepageFeatures />
      </main>
    </Layout>
  );
}
