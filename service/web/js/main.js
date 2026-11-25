const { render, html } = await import("./core.js");
const { App } = await import("./app.js");

render(html`<${App} />`, document.getElementById("app"));
