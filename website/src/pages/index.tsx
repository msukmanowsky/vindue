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
        <img className={styles.heroLogo} src="/img/logo.svg" alt="" aria-hidden="true" />
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
        </div>
        <p className={styles.heroNote}>
          Free &amp; open source (MIT) · Apple Silicon + Intel · menu-bar app,
          no Dock icon · no telemetry, everything stays local
        </p>
        <img
          className={styles.heroDemo}
          src="/img/vindue-demo.gif"
          alt="Vindue demo: a hotkey opens a grid panel over every display, and dragging across cells tiles the frontmost window there"
        />
      </div>
    </header>
  );
}

function AutomationBand() {
  return (
    <section className={styles.automation}>
      <div className="container">
        <Heading as="h2" className={styles.automationTitle}>
          Built for AI and automation
        </Heading>
        <p className={styles.automationBlurb}>
          Every panel action is also an API call: Vindue serves a loopback
          REST API and an MCP endpoint on <code>127.0.0.1</code>, so scripts,
          Claude, or any MCP client can tile windows, manage shortcuts, and
          read state. No auth to manage — the server rejects anything a
          browser could send, so web pages can never reach it.
        </p>
        <div className="row">
          <div className="col col--6">
            <CodeBlock language="bash" title="REST">
              {`curl -s -X POST 127.0.0.1:47725/api/v1/tile \\
  -d '{"preset":"left_half","app":"Safari"}'`}
            </CodeBlock>
          </div>
          <div className="col col--6">
            <CodeBlock language="bash" title="MCP">
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
  return (
    <Layout
      title="Grid window tiling for macOS"
      description="Vindue is an open-source grid window tiler for macOS with a built-in HTTP API and MCP server, so scripts and AI clients can drive window management.">
      <HomepageHeader />
      <main>
        <HomepageFeatures />
        <AutomationBand />
      </main>
    </Layout>
  );
}
