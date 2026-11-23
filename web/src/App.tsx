import { useMemo, useState } from "react";
import Access from "./Access.tsx";
import { BarContext } from "./Bar.tsx";
import HealthStatus from "./HealthStatus.tsx";
import "./App.css";

function App() {
  const [start, setStart] = useState<HTMLElement | null>(null);
  const [end, setEnd] = useState<HTMLElement | null>(null);
  const slots = useMemo(() => ({ start, end }), [start, end]);

  return (
    <BarContext value={slots}>
      <header className="app-bar">
        <div className="app-bar-side">
          <h1 className="app-mark">Sidereal</h1>
          <div className="app-bar-slot" ref={setStart} />
        </div>
        <div className="app-bar-side">
          <HealthStatus />
          <div className="app-bar-slot" ref={setEnd} />
        </div>
      </header>
      <main className="app-main">
        <section aria-label="Access">
          <Access />
        </section>
      </main>
    </BarContext>
  );
}

export default App;
