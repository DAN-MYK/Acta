import App from "./App.svelte";
import "./lib/styles/tokens.css";
import "./styles.css";

const app = new App({
  target: document.getElementById("app")!
});

export default app;
