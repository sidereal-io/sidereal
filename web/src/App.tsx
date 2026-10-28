import Access from "./Access.tsx";
import HealthStatus from "./HealthStatus.tsx";

function App() {
  return (
    <main>
      <h1>Sidereal</h1>
      <HealthStatus />
      <section aria-label="Access">
        <Access />
      </section>
    </main>
  );
}

export default App;
