YuOsApp.ready.then((app) => {
  "use strict";
  const ui = YuOsUi; ui.showApp(app);
  const form = document.querySelector("#summarize");
  const text = form.elements.text;
  text.oninput = () => { document.querySelector("#input-state").textContent = `${text.value.length} characters`; };
  function output(value) {
    document.querySelector("#result").textContent = value.summary;
    const details = [];
    if (value.model) details.push(`Model: ${value.model}`);
    if (value.provider) details.push(`Provider: ${value.provider}`);
    if (value.usage) details.push(`App-reported usage: ${value.usage.units} units`);
    if (value.budget) {
      const { period, units_used, units_cap } = value.budget;
      const usage = units_cap === 0 ? `${units_used} units used (unlimited)` :
        `${units_used} of ${units_cap} units used`;
      details.push(`Shared App AI budget ${period}: ${usage}`);
    }
    document.querySelector("#usage").textContent = details.join(" - ");
  }
  form.onsubmit = (event) => {
    event.preventDefault();
    const submitted = text.value;
    if (!submitted.trim()) { document.querySelector("#notice").textContent = "Enter text to summarize."; return; }
    void ui.run(form.querySelector("button"), () => YuOsApp.call("run", { text: submitted }), (value) => {
      document.querySelector("#summary-source").textContent = `Summary of the submitted ${submitted.length} characters. Input edits are not resubmitted automatically.`;
      output(value);
    }, document.querySelector("#notice"));
  };
  ui.allTools(document.querySelector("#more"), app, output);
});
