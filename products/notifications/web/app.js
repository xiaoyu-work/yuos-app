(() => {
  "use strict";
  const language = navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
  const t = (key, values = {}) => notificationStrings[language][key].replace(/\{(\w+)\}/g, (_, name) => values[name]);
  const $ = (selector) => document.querySelector(selector);
  const node = YuOsUi.node;
  document.documentElement.lang = language;
  document.title = t("app");
  for (const el of document.querySelectorAll("[data-text]")) el.textContent = t(el.dataset.text);
  for (const el of document.querySelectorAll("[data-placeholder]")) el.placeholder = t(el.dataset.placeholder);
  for (const el of document.querySelectorAll("[data-label]")) {
    el.setAttribute("aria-label", t(el.dataset.label));
    el.title = t(el.dataset.label);
  }

  YuOsApp.ready.then((app) => {
    const has = (command) => Boolean(YuOsApp.tool(app, command));
    const native = app.id === "yuos-notifications";
    const canCompose = has(native ? "post" : "send");
    const access = { history: !native && has("list"), posted: native && has("post"), dismiss: native && has("close"), diagnostics: true };
    const labels = { history: "history", posted: "posted", dismiss: "dismiss", diagnostics: "raw" };
    const navigation = [...document.querySelectorAll("[data-view]")];
    const durableId = /^notif-[a-fA-F0-9]{32}$/;
    const bareUtc = /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d$/;
    const shortTime = new Intl.DateTimeFormat(language, {
      month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit", hourCycle: "h23", timeZone: "UTC",
    });
    const dialog = $("#confirm");
    const receipts = [];
    let records = [];
    let total = 0;
    let loaded = false;
    let busy = false;
    let closeAttempted = false;
    let unreadOnly = false;
    let inspectorOpen = true;
    let selectedId = null;
    let view;
    let countStatus = "";
    let outcome = "";

    function icon(name, className) {
      const namespace = "http://www.w3.org/2000/svg";
      const svg = document.createElementNS(namespace, "svg");
      const use = document.createElementNS(namespace, "use");
      svg.setAttribute("aria-hidden", "true");
      if (className) svg.setAttribute("class", className);
      use.setAttribute("href", `#i-${name}`);
      svg.append(use);
      return svg;
    }
    function showNotice(text, isError = false, target = $("#notice")) {
      target.textContent = text;
      target.hidden = !text;
      target.classList.toggle("error", isError);
    }
    function update() {
      const inspect = canCompose && view !== "diagnostics" && inspectorOpen;
      $("#inspector").hidden = !inspect;
      $("#inspector-toggle").setAttribute("aria-expanded", String(inspect));
      $(".panels").classList.toggle("full", !inspect);
      $("#refresh").disabled = busy || !access.history;
      $("#limit").disabled = busy || !access.history;
      $("#send-button").disabled = busy || !canCompose;
      $("#review").disabled = busy || !access.dismiss;
      $("#confirm-button").disabled = busy || closeAttempted;
      $("#cancel").disabled = dialog.open && busy;
      for (const button of document.querySelectorAll(".row-action")) button.disabled = busy;
      $("#characters").textContent = native
        ? t("counts", { headline: [...$("#summary").value].length, body: [...$("#body").value].length })
        : `${[...$("#message").value].length} / 4000`;
    }
    async function perform(command, args, apply, failed, target = $("#notice")) {
      const tool = YuOsApp.tool(app, command);
      if (!tool || busy) return;
      const writeInputs = ["post", "send"].includes(command)
        ? [...$("#send").querySelectorAll("input:not(:disabled), textarea:not(:disabled)")] : [];
      busy = true;
      for (const input of writeInputs) input.disabled = true;
      update();
      showNotice(t("working"), false, target);
      try {
        const result = await YuOsApp.call(tool.name, args);
        YuOsUi.output($("#raw"), result);
        $("#raw").hidden = false;
        $("#raw-empty").hidden = true;
        if (result?.error) throw new Error(String(result.error));
        apply(result);
        showNotice("", false, target);
      } catch (error) {
        showNotice(error.message, true, target);
        if (failed) failed();
      } finally {
        busy = false;
        for (const input of writeInputs) input.disabled = false;
        update();
      }
    }
    function utc(value) {
      if (!value) return t("noTime");
      return `${value}${bareUtc.test(value) ? " UTC" : ""}`;
    }
    function historyTime(value) {
      if (!value) return t("noTime");
      const parsed = Date.parse(bareUtc.test(value) ? `${value}Z` : value);
      return Number.isFinite(parsed) ? shortTime.format(parsed) : value;
    }
    function renderFooter(visibleCount) {
      let detail = "";
      if (view === "history") {
        detail = `${loaded ? `${t("matchCount", { count: visibleCount })} · ` : ""}${t("deliveryHint")}`;
      }
      $("#outcome").textContent = outcome || detail;
      $("#list-state").textContent = view === "history" ? countStatus : "";
    }
    function renderRecords() {
      $("#history-count").textContent = loaded ? records.length : "";
      $("#posted-count").textContent = access.posted ? receipts.length : "";
      if (view !== "history" && view !== "posted") {
        $("#total").textContent = "";
        renderFooter(0);
        return;
      }
      const posted = view === "posted";
      $("#actions-heading").hidden = !posted || !access.dismiss;
      $("#time-heading").textContent = t(posted ? "time" : "utcTime");
      const filter = $("#filter").value.trim().toLowerCase();
      const source = posted ? receipts : records;
      const visible = source.filter((entry) =>
        (posted || !unreadOnly || entry.read === false) &&
        `${entry.id} ${entry.message || entry.summary || ""} ${entry.body || ""}`.toLowerCase().includes(filter));
      $("#total").textContent = posted ? t("postedCount", { count: receipts.length })
        : loaded ? t("loadedCount", { count: records.length, total }) : "";
      const rows = [];
      for (const [index, entry] of visible.entries()) {
        const active = entry.id === selectedId;
        const rawMessage = (posted ? entry.body : entry.message) || "";
        const kind = t(entry.urgent ? "urgentNotification" : "notification");
        const [firstLine, ...rest] = rawMessage.trim().split(/\r?\n/);
        const title = posted ? entry.summary : firstLine || kind;
        const subtitle = posted ? rawMessage || t("noBody") : rest.join(" ").trim() || kind;
        const when = posted ? entry.received : utc(entry.timestamp);
        const state = posted ? t(entry.dismissed ? "dismissed" : entry.closeRequested ? "requested" : "returned")
          : t(entry.read === false ? "unread" : entry.read === true ? "read" : "unreported");
        const row = node("tr", null, { class: active ? "entry selected" : "entry" });
        const main = node("button", null, { type: "button", class: "row-main", "aria-expanded": active, "aria-controls": `details-${index}` });
        const message = node("span");
        message.append(node("b", title), node("small", subtitle));
        main.append(icon(posted ? "inbox" : entry.urgent ? "warn" : "bell", entry.urgent ? "warning" : ""), message);
        const firstCell = node("td");
        firstCell.append(main);
        const status = node("td", null, { title: state });
        if (!posted && entry.read === false) status.append(node("span", null, { class: "dot", "aria-hidden": "true" }));
        status.append(state);
        const idPreview = durableId.test(entry.id) ? `${entry.id.slice(0, 10)}…${entry.id.slice(-4)}` : entry.id;
        row.append(firstCell, node("td", posted ? when : historyTime(entry.timestamp), { title: when }), status,
          node("td", idPreview, { class: "mono", title: entry.id }));
        if (posted && access.dismiss) {
          const cell = node("td");
          if (!entry.dismissed && !entry.closeRequested) {
            const action = node("button", t("review"), { type: "button", class: "row-action" });
            action.onclick = () => openConfirmation(entry.id);
            action.disabled = busy;
            cell.append(action);
          }
          row.append(cell);
        }
        const detailRow = node("tr", null, { id: `details-${index}` });
        detailRow.hidden = !active;
        const cell = node("td", null, { colspan: row.children.length });
        const detail = node("div", null, { class: "entry-details" });
        detail.append(node("b", posted ? title : kind), node("p", rawMessage || t("noBody")),
          node("p", `${when} · ${state}${!posted && entry.state ? ` · ${entry.state}` : ""}`),
          node("p", entry.id, { class: "mono" }));
        cell.append(detail);
        detailRow.append(cell);
        main.onclick = () => {
          selectedId = active ? null : entry.id;
          renderRecords();
          document.querySelectorAll(".row-main")[index]?.focus();
        };
        rows.push(row, detailRow);
      }
      YuOsUi.replace($("#rows"), rows);
      $("#empty").hidden = visible.length > 0;
      $("#empty").textContent = t(source.length ? "noMatch" : posted ? "emptyPosts" : loaded ? "emptyHistory" : "noHistory");
      renderFooter(visible.length);
    }
    function showView(next) {
      if (!access[next]) return;
      if (view !== next) {
        $("#filter").value = "";
        selectedId = null;
      }
      view = next;
      const list = view === "history" || view === "posted";
      for (const button of navigation) {
        if (button.dataset.view === view) button.setAttribute("aria-current", "page");
        else button.removeAttribute("aria-current");
      }
      $("#title").textContent = t(labels[view]);
      $("#history-tools").hidden = view !== "history";
      $("#search").hidden = !list;
      $("#list").hidden = !list;
      $("#dismiss-page").hidden = view !== "dismiss";
      $("#raw-page").hidden = view !== "diagnostics";
      $("#compose").hidden = $("#inspector-toggle").hidden = !canCompose || view === "diagnostics";
      renderRecords();
      update();
    }
    function refresh() {
      if (busy || !access.history) return;
      const limit = $("#limit").valueAsNumber;
      if (!Number.isInteger(limit) || limit < 1 || limit > 100) {
        showNotice(t("limitError"), true);
        return;
      }
      countStatus = t("loading");
      outcome = "";
      renderRecords();
      void perform("list", { limit }, (value) => {
        if (!Array.isArray(value?.notifications) || !Number.isInteger(value.total) || value.total < 0) {
          throw new Error(t("invalidList"));
        }
        records = value.notifications;
        total = value.total;
        loaded = true;
        countStatus = t("refreshed", { time: new Date().toLocaleTimeString(language) });
        renderRecords();
      }, () => {
        countStatus = t(loaded ? "stale" : "unavailable");
        renderRecords();
      });
    }
    function openConfirmation(id) {
      if (busy || !access.dismiss || !durableId.test(id)) return;
      closeAttempted = false;
      $("#confirm-id").value = id;
      $("#cancel").textContent = t("cancel");
      showNotice("", false, $("#confirm-notice"));
      dialog.showModal();
      update();
    }
    function validateText(value, field, max, optional, multiline) {
      const invalid = multiline ? /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f-\u009f]/ : /[\u0000-\u001f\u007f-\u009f]/;
      if ((!optional && !value.trim()) || [...value].length > max || invalid.test(value)) {
        throw new Error(t("textError", { field: t(field), max }));
      }
    }
    function postArguments() {
      const summary = $("#summary").value;
      const body = $("#body").value;
      const sender = $("#sender").value || "Yu OS Agent";
      const expiry = $("#expiry").valueAsNumber;
      validateText(summary, "headline", 240, false, false);
      validateText(body, "body", 4000, true, true);
      validateText(sender, "sender", 128, false, false);
      if (body && !body.trim()) throw new Error(t("bodyEmpty"));
      if (!Number.isInteger(expiry) || expiry < -1 || expiry > 2147483647) throw new Error(t("expiryError"));
      const args = { summary, body, app_name: sender, icon: $("#icon").value, expire_ms: expiry, transient: $("#transient").checked };
      if ($("#dedupe").value) args.dedupe_key = $("#dedupe").value;
      return args;
    }

    $("#scope").textContent = t(native ? "nativeScope" : "historyScope");
    $("#compose-title").textContent = $("#send-button span").textContent = t(native ? "post" : "send");
    $("#message-label").hidden = $("#urgent-label").hidden = $("#message").disabled = $("#urgent").disabled = native;
    $("#message").required = !native;
    $("#summary-label").hidden = $("#body-label").hidden = $("#post-options").hidden = !native;
    $("#summary").required = native;
    for (const el of document.querySelectorAll("#summary, #body, #post-options input")) el.disabled = !native;
    for (const button of navigation) {
      const next = button.dataset.view;
      button.disabled = !access[next];
      if (button.disabled) {
        const entry = next === "history" ? "notify" : "yuos-notifications";
        button.title = entry === app.id ? t("missingTool") : t("otherEntry", { entry });
        button.setAttribute("aria-label", `${t(labels[next])}. ${button.title}`);
      }
      button.onclick = () => showView(next);
    }
    for (const filterButton of [$("#all"), $("#unread")]) {
      filterButton.onclick = () => {
        unreadOnly = filterButton.id === "unread";
        $("#all").setAttribute("aria-pressed", String(!unreadOnly));
        $("#unread").setAttribute("aria-pressed", String(unreadOnly));
        selectedId = null;
        renderRecords();
      };
    }
    $("#filter").oninput = () => {
      selectedId = null;
      renderRecords();
    };
    $("#refresh").onclick = refresh;
    $("#limit").onkeydown = (event) => { if (event.key === "Enter") refresh(); };
    $("#compose").onclick = () => {
      inspectorOpen = true;
      update();
      $(native ? "#summary" : "#message").focus();
    };
    $("#inspector-toggle").onclick = () => { inspectorOpen = !inspectorOpen; update(); };
    for (const el of document.querySelectorAll("#summary, #body, #message")) el.oninput = update;
    $("#send").onsubmit = (event) => {
      event.preventDefault();
      if (busy || !canCompose || !$("#send").reportValidity()) return;
      let args;
      try {
        if (native) args = postArguments();
        else {
          const message = $("#message").value;
          validateText(message, "message", 4000, false, true);
          args = { message, urgent: $("#urgent").checked };
        }
      } catch (error) {
        showNotice(error.message, true);
        return;
      }
      outcome = t("sending");
      renderRecords();
      void perform(native ? "post" : "send", args, (value) => {
        if (typeof value?.id !== "string" || !durableId.test(value.id)) throw new Error(t("noId"));
        $("#receipt").textContent = `${t("id")}: ${value.id}${value.timestamp ? `\n${t("created")}: ${utc(value.timestamp)}` : ""}`;
        outcome = t("sent");
        if (native) {
          const duplicate = receipts.findIndex((entry) => entry.id === value.id);
          if (duplicate === -1) {
            receipts.unshift({ id: value.id, summary: args.summary, body: args.body, received: new Date().toLocaleTimeString(language) });
            receipts.splice(50);
          }
          if ($("#summary").value === args.summary && $("#body").value === args.body) {
            $("#summary").value = $("#body").value = "";
          }
        } else {
          if ($("#message").value === args.message) $("#message").value = "";
          countStatus = t("refreshAfterSend");
        }
        renderRecords();
      });
    };
    $("#close-form").onsubmit = (event) => {
      event.preventDefault();
      if ($("#close-form").reportValidity()) openConfirmation($("#close-id").value);
    };
    $("#cancel").onclick = () => dialog.close();
    dialog.oncancel = (event) => { if (busy) event.preventDefault(); };
    $("#confirm-form").onsubmit = (event) => {
      event.preventDefault();
      const id = $("#confirm-id").value;
      if (busy || closeAttempted || !access.dismiss || !durableId.test(id)) return;
      closeAttempted = true;
      const entry = receipts.find((receipt) => receipt.id === id);
      if (entry && !entry.dismissed) entry.closeRequested = true;
      outcome = t("closePending");
      renderFooter(0);
      void perform("close", { id }, (value) => {
        if (value?.ok !== true || value.id !== id || value.state !== "dismissed") throw new Error(t("closeUnconfirmed"));
        if (entry) { entry.dismissed = true; entry.closeRequested = false; }
        outcome = t("closed", { id });
        dialog.close();
        renderRecords();
      }, () => {
        $("#cancel").textContent = t("done");
        renderRecords();
      }, $("#confirm-notice"));
    };

    showView(access.history ? "history" : access.posted ? "posted" : access.dismiss ? "dismiss" : "diagnostics");
    $("#connecting").hidden = true;
    $("#workspace").hidden = false;
    if (access.history) refresh();
  });
})();
