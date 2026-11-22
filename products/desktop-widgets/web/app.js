YuOsApp.ready.then((app) => {
  "use strict";
  const ui = YuOsUi;
  const $ = (id) => document.getElementById(id);
  ui.showApp(app);
  if (app.id !== "widget-rail") {
    ui.replace($("workspace"), [ui.node("p", "This identity has no Widget Rail presentation.", { class: "empty" })]);
    return;
  }
  let pending = false;
  function system(result) {
    const value = result.summary;
    if (!value || typeof value !== "object") throw new Error("Invalid OS telemetry response.");
    const usage = (item) => item ? `${item.used_mb.toLocaleString()} / ${item.total_mb.toLocaleString()} MiB` : "Not reported";
    const rate = (number) => typeof number === "number" ? `${number.toLocaleString()} B/s` : "Not reported";
    const values = ui.node("dl", null, { class: "detail-grid" });
    for (const [label, text] of [
      ["CPU", typeof value.cpu_percent === "number" ? `${value.cpu_percent.toFixed(1)}%` : "Not reported"],
      ["Memory", usage(value.memory)], ["Storage", usage(value.storage)],
      ["Download", rate(value.network_down_bps)], ["Upload", rate(value.network_up_bps)],
    ]) values.append(ui.node("dt", label), ui.node("dd", text));
    return [values, ui.node("p", value.fallback ? "The OS reports fallback telemetry." : "Reported by the OS telemetry service.", { class: "small muted" })];
  }
  async function refresh() {
    if (pending) return;
    const state = $("system-state");
    const name = "widget-rail.system";
    if (!YuOsApp.tool(app, name)) {
      state.textContent = "Unavailable: this App does not declare this data source.";
      return;
    }
    pending = true;
    state.textContent = "Refreshing…";
    try {
      const result = await YuOsApp.call(name, {});
      ui.replace($("system"), system(result));
      state.textContent = "Updated";
    } catch (error) {
      ui.replace($("system"), []);
      state.textContent = `Unavailable: ${error.message}`;
    } finally {
      pending = false;
    }
  }
  $("refresh").onclick = refresh;
  void refresh();
  const timer = setInterval(() => {
    if (!document.hidden) void refresh();
  }, 5000);
  window.addEventListener("pagehide", () => clearInterval(timer), { once: true });
}).catch((error) => {
  document.getElementById("notice").textContent = `Widget presentation unavailable: ${error.message}`;
});
