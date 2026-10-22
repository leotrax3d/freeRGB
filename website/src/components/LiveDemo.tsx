export function LiveDemo() {
  const url = `${import.meta.env.BASE_URL}demo/`
  return <section id="demo" className="catalog view-section">
    <header className="view-header"><div className="view-symbol">↗</div><div>
      <h2>Try the real interface</h2>
      <p>The desktop UI, built from the same source. Explore lighting, profiles, layout and simulated device setup without installing anything.</p>
    </div></header>
    <div className="demo-launch">
      <h3>Your desk, simulated.</h3>
      <p>Launch the complete application in its own tab. Try virtual keyboards, fans and light strips. Downloads and readings are simulated; nothing connects to your hardware or local service.</p>
      <a className="button primary" href={url} target="_blank" rel="noreferrer">Launch live demo ↗</a>
      <p className="section-note">Best on a desktop display. Audio capture, screen capture and macro execution are unavailable. Changes reset when reloaded.</p>
    </div>
    <p className="section-note">Ready to use your own devices? <a href="#install">Install freeRGB →</a></p>
  </section>
}
