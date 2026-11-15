import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./legacy/AppLegacy";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
