// Highlight strip window: a solid-color rectangle placed along one edge of
// the target window by the Rust side. Opaque on purpose (no transparency =>
// no private APIs on macOS).
import "./styles.css";

export default function Strip() {
  return <div className="strip" />;
}
