import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App.tsx";
import "./styles/tokens.css";
import "./styles/fonts.css";
import "./styles/base.css";

const root = document.getElementById("root");
if (!root) throw new Error('index.html has no element with id "root"');

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
