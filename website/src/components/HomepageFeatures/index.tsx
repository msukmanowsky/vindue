import Heading from '@theme/Heading';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  text: string;
};

const FeatureList: FeatureItem[] = [
  {
    title: 'Drag-to-tile grid',
    text: 'A hotkey (or tray click) throws a translucent grid over your screen. Drag across cells — the frontmost window lands exactly there. Esc dismisses; clicking away dismisses. It stays out of your way.',
  },
  {
    title: 'Every display at once',
    text: 'Panels open on all displays simultaneously, each grid mirroring that display\u2019s aspect ratio. Drag on any panel to place the window there.',
  },
  {
    title: 'Keyboard shortcuts with display memory',
    text: 'Save any grid region to a key. Assignments can pin to a specific display — press 3 and the window flies to the left half of your Dell, even while you\u2019re looking at the MacBook.',
  },
  {
    title: 'Private, local, MIT',
    text: 'No account, no telemetry, no cloud. Config is a JSON file you own. Built with Tauri 2 + Rust — idle memory stays tiny. The whole source is MIT licensed.',
  },
];

function Feature({title, text}: FeatureItem) {
  return (
    <div className={styles.feature}>
      <Heading as="h3">{title}</Heading>
      <p>{text}</p>
    </div>
  );
}

export default function HomepageFeatures(): React.ReactNode {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <div key={idx} className="col col--6">
              <Feature {...props} />
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
