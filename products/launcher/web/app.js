YuOsApp.ready.then((app) => {
  "use strict";
  const ui = YuOsUi;
  const $ = (selector) => document.querySelector(selector);
  const has = (command) => Boolean(YuOsApp.tool(app, command));
  const native = app.id === "yuos-launcher";
  const lang = /^zh\b/i.test(navigator.language || "") ? "zh" : "en";
  const MAIN_CATEGORIES = ["AudioVideo", "Audio", "Video", "Development", "Education", "Game", "Graphics",
    "Network", "Office", "Science", "Settings", "System", "Utility"];
  const T = {
    en: {
      title: "Launcher", placeholder: "Search apps: name, category or keyword", all: "All Applications",
      recent: "Recent Launches", options: "Options", clear: "Clear search", visibility: "All-app visibility options",
      includeHidden: "Include hidden desktop entries", includeNoDisplay: "Include entries marked NoDisplay",
      visibilityNote: "These options apply to All applications, not ranked search or recent history.",
      limit: "Search limit", showRaw: "Show backend response", rawTitle: "Latest backend response",
      limitHint: (n) => `Up to ${n} results`, status: (count, chosen) => `${chosen} of ${count} selected`,
      count: (n) => String(n), recentRow: "Recently used", other: "Other", launchDots: "Launch…",
      check: "Check running processes", info: "Info", select: "Select", escape: "Clear",
      disclaimer: "Selecting never launches. Launch sends the exact desktop AppID and optional file/URI targets to the OS desktop service, not another App's business or MCP interface. Permission and session checks remain authoritative.",
      launchLead: "Review this exact AppID and optional targets. Only absolute non-file URIs and existing authorized local files are supported; raw Exec arguments are not accepted.",
      appId: "Desktop AppID", uri: "Optional absolute non-file URIs (one per line)",
      path: "Optional authorized absolute local files (one per line)",
      extras: "Optional ordered absolute local files / non-file URIs (one per line)",
      emptyTargets: "Leave empty to launch the app only",
      retry: "An unknown result must not be blindly retried — check recent launches or running state first.",
      cancel: "Cancel", launchVia: "Launch through the OS", launchWord: "Launch", withWord: "with",
      targets: "optional targets", run: "Run", genericName: "Generic name", categoriesLabel: "Categories",
      keywords: "Keywords", visibilityLabel: "Visibility", terminal: "Terminal entry", lastLaunch: "Last launch",
      recentCount: "Recent count", hiddenValue: "Hidden", notReported: "Not reported by recent history",
      visible: "Visible", yes: "Yes", no: "No", noDescription: "No description provided.",
      recentDescription: "Recent launch record. The backend verifies that the AppID is still installed when launching.",
      selectApp: "Select an application", loading: "Loading applications...",
      listed: (n) => `Listed ${n} installed applications.`, matched: (n) => `${n} search matches.`,
      recentListed: (n) => `${n} recent launches by this entry.`,
      emptyList: "No applications match this request.", emptyRecent: "No recent launches in this entry's own journal.",
      loadFailed: "Could not load applications. See the service error below; no empty catalog is inferred.",
      noList: "The backend returned no application list.",
      running: (pids) => `Observed matching PIDs: ${pids}. Process-name observation is not an application lifetime guarantee.`,
      notRunning: "No matching process was observed.", noObservation: "No process observation was returned.",
      launched: (name, at) => `OS reported launch of ${name}${at ? ` at ${at}` : ""}.`,
      unconfirmed: "The launch outcome was not confirmed. Check recent history or running state before trying again.",
      working: "Working...", times: (n) => `${n}×`,
      categories: { AudioVideo: "Audio & Video" },
    },
    zh: {
      title: "启动器", placeholder: "搜索应用：名称、类别或关键词", all: "所有应用", recent: "最近启动",
      options: "选项", clear: "清除搜索", visibility: "显示选项", includeHidden: "包含隐藏条目",
      includeNoDisplay: "包含 NoDisplay 条目", visibilityNote: "仅作用于“所有应用”，不影响搜索和最近启动。",
      limit: "搜索上限", showRaw: "显示后端响应", rawTitle: "最新后端响应",
      limitHint: (n) => `最多 ${n} 个结果`, status: (count, chosen) => `${count} 个条目，已选择 ${chosen} 个`,
      count: (n) => `${n} 个`, recentRow: "最近使用", other: "其他", launchDots: "启动…",
      check: "检查运行进程", info: "简介", select: "选择", escape: "清除",
      disclaimer: "选中不会启动。启动只把确切的桌面 AppID 和可选的文件/URI 目标交给系统桌面服务，不会调用其他应用的业务或 MCP 接口；权限和会话检查仍以系统为准。",
      launchLead: "核对确切的 AppID 和可选目标。仅支持绝对的非文件 URI 和已授权的现有本地文件；不接受原始 Exec 参数。",
      appId: "桌面 AppID", uri: "可选：绝对的非文件 URI（每行一个）", path: "可选：已授权的本地文件绝对路径（每行一个）",
      extras: "可选：按顺序的本地文件绝对路径 / 非文件 URI（每行一个）", emptyTargets: "留空表示仅启动应用",
      retry: "结果未知时不要盲目重试，请先查看最近启动或运行状态。", cancel: "取消", launchVia: "通过系统启动",
      launchWord: "启动", withWord: "并打开", targets: "可选目标", run: "运行", genericName: "通用名称",
      categoriesLabel: "类别", keywords: "关键词", visibilityLabel: "可见性", terminal: "终端条目",
      lastLaunch: "上次启动", recentCount: "启动次数", hiddenValue: "隐藏", notReported: "最近历史未报告",
      visible: "可见", yes: "是", no: "否", noDescription: "未提供说明。",
      recentDescription: "最近启动记录。启动时后端会核实该 AppID 仍已安装。", selectApp: "选择一个应用",
      loading: "正在加载应用…", listed: (n) => `已列出 ${n} 个已安装应用。`, matched: (n) => `找到 ${n} 个匹配项。`,
      recentListed: (n) => `此入口有 ${n} 条最近启动记录。`, emptyList: "没有符合此请求的应用。",
      emptyRecent: "此入口自己的记录中没有最近启动。", loadFailed: "无法加载应用。请查看底栏中的服务错误；不会推断为空目录。",
      noList: "后端未返回应用列表。",
      running: (pids) => `观察到匹配的 PID：${pids}。进程名观察不代表应用生命周期保证。`,
      notRunning: "未观察到匹配的进程。", noObservation: "未返回进程观察结果。",
      launched: (name, at) => `系统已报告${at ? `于 ${at} ` : ""}启动 ${name}。`,
      unconfirmed: "启动结果未确认。请先查看最近启动或运行状态，再决定是否重试。", working: "正在处理…",
      times: (n) => `${n} 次`,
      categories: { AudioVideo: "影音", Audio: "音频", Video: "视频", Development: "开发", Education: "教育",
        Game: "游戏", Graphics: "图形", Network: "网络", Office: "办公", Science: "科学", Settings: "设置",
        System: "系统", Utility: "工具" },
    },
  }[lang];
  const state = {
    mode: "list", explicit: false, entries: [], total: null, recentRow: [], filter: "", focus: 0, pinned: null,
    info: false, launching: null, raw: false, menu: false, hidden: false, noDisplay: false, loading: false,
    error: false, notice: null, outcome: null, running: null, checking: false, submitting: false,
  };
  let generation = 0;
  let selection = 0;
  let searchTimer = 0;

  document.documentElement.lang = lang;
  document.title = T.title;
  $("#panel").setAttribute("aria-label", typeof app.name === "string" ? app.name : app.id);
  for (const element of document.querySelectorAll("[data-t]")) element.textContent = T[element.dataset.t];
  for (const element of document.querySelectorAll("[data-t-placeholder]")) element.placeholder = T[element.dataset.tPlaceholder];
  for (const element of document.querySelectorAll("[data-t-label]")) element.setAttribute("aria-label", T[element.dataset.tLabel]);

  function call(command, args = {}) {
    const tool = YuOsApp.tool(app, command);
    if (!tool) throw new Error(`This entry does not declare ${command}.`);
    return YuOsApp.call(tool.name, args);
  }
  function checked(value) {
    ui.output($("#raw"), value);
    if (value && value.error) throw new Error(String(value.error));
    return value;
  }
  function errorText(error) {
    return error && error.message ? error.message : String(error);
  }
  function lines(value) {
    return value.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  }
  function when(value) {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? String(value) :
      date.toLocaleString(lang === "zh" ? "zh-CN" : undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  }
  function svg(name) {
    const ns = "http://www.w3.org/2000/svg";
    const element = document.createElementNS(ns, "svg");
    element.setAttribute("class", "i");
    const use = document.createElementNS(ns, "use");
    use.setAttribute("href", `#i-${name}`);
    element.append(use);
    return element;
  }
  function glyph(entry, size) {
    const label = String(entry.name || entry.app_id || "?").trim();
    let hue = 0;
    for (const character of String(entry.app_id || label)) hue = (hue * 31 + character.codePointAt(0)) % 360;
    const element = ui.node("span", (Array.from(label)[0] || "?").toUpperCase(), { class: `glyph s${size}`, "aria-hidden": "true" });
    element.style.setProperty("--h", String(hue));
    return element;
  }
  function kb(key, label) {
    const element = ui.node("span");
    element.append(ui.node("kbd", key), document.createTextNode(label));
    return element;
  }
  function note(target, message, kind) {
    target.replaceChildren();
    target.classList.toggle("error", kind === "error");
    if (!message) return;
    if (kind === "ok") target.append(ui.node("span", null, { class: "dot" }));
    target.append(document.createTextNode(message));
  }
  function setChecked(element, on) {
    element.setAttribute("aria-checked", String(on));
    element.querySelector(".ck").textContent = on ? "✓" : "";
  }
  function empty(iconName, message) {
    const element = ui.node("div", null, { class: "empty" });
    element.append(svg(iconName), ui.node("p", message));
    return element;
  }
  function limit() {
    const value = Number($("#limit").value);
    return Number.isInteger(value) && value >= 1 && value <= 50 ? value : 10;
  }
  function categoryOf(entry) {
    const tokens = String(entry.categories || "").split(";");
    return MAIN_CATEGORIES.find((name) => tokens.includes(name)) || "";
  }
  function categoryLabel(key) {
    return key ? T.categories[key] || key : T.other;
  }
  function isSplit() {
    return state.mode !== "list" || state.info;
  }
  function groups() {
    const entries = state.filter ? state.entries.filter((entry) => categoryOf(entry) === state.filter) : state.entries;
    if (state.mode === "recent") return [{ title: T.recent, entries }];
    if (!isSplit()) return [{ title: T.all, entries }];
    const grouped = new Map();
    for (const entry of entries) {
      const key = categoryOf(entry);
      if (!grouped.has(key)) grouped.set(key, []);
      grouped.get(key).push(entry);
    }
    return Array.from(grouped, ([key, list]) => ({ title: categoryLabel(key), entries: list }));
  }
  function visible() {
    return groups().flatMap((group) => group.entries);
  }
  function selected() {
    return state.pinned || visible()[state.focus] || null;
  }

  function row(entry, position) {
    const button = ui.node("button", null, { type: "button", class: "row", role: "option", "data-index": position });
    const text = ui.node("div", null, { class: "t" });
    const sub = ui.node("div", null, { class: "sub" });
    sub.append(ui.node("span", entry.app_id, { class: "mono" }));
    const category = categoryOf(entry);
    for (const part of [category ? categoryLabel(category) : "", entry.terminal ? T.terminal : ""]) {
      if (part) sub.append(document.createTextNode(` · ${part}`));
    }
    text.append(ui.node("div", entry.name || entry.app_id, { class: "nm" }), sub);
    button.append(glyph(entry, 30), text);
    if (state.mode === "recent") {
      const right = ui.node("div", null, { class: "rt" });
      right.append(ui.node("div", entry.last_launched_at ? when(entry.last_launched_at) : ""),
        ui.node("div", entry.count === undefined ? "" : T.times(entry.count)));
      button.append(right);
    }
    button.onclick = () => focusAt(position);
    button.ondblclick = () => openLaunch(entry);
    return button;
  }
  function tile(entry, attributes, onSelect) {
    const button = ui.node("button", null, { type: "button", class: "tile", role: "option", title: entry.name || entry.app_id, ...attributes });
    button.append(glyph(entry, 64), ui.node("span", entry.name || entry.app_id, { class: "label" }));
    button.onclick = onSelect;
    button.ondblclick = () => openLaunch(entry);
    return button;
  }
  function renderResults() {
    const target = $("#results");
    const split = isSplit();
    if (state.loading && !state.entries.length) return ui.replace(target, [empty("search", T.loading)]);
    if (state.error) return ui.replace(target, [empty("warn", T.loadFailed)]);
    const nodes = [];
    if (!split && state.recentRow.length && !state.filter) {
      nodes.push(ui.node("div", T.recentRow, { class: "sec" }));
      const recentGrid = ui.node("div", null, { class: "grid" });
      for (const record of state.recentRow) recentGrid.append(tile(record, { "data-pin": record.app_id }, () => pin(record)));
      nodes.push(recentGrid);
    }
    let position = 0;
    for (const group of groups()) {
      if (!group.entries.length) continue;
      nodes.push(ui.node("div", `${group.title} · ${T.count(group.entries.length)}`, { class: "sec" }));
      const container = ui.node("div", null, split ? {} : { class: "grid", id: "main-grid" });
      for (const entry of group.entries) {
        const index = position++;
        container.append(split ? row(entry, index) : tile(entry, { "data-index": index }, () => focusAt(index)));
      }
      nodes.push(container);
    }
    if (!position) nodes.push(empty("search", state.mode === "recent" ? T.emptyRecent : T.emptyList));
    ui.replace(target, nodes);
    markSelection();
  }
  function markSelection() {
    for (const element of $("#results").querySelectorAll("[data-index], [data-pin]")) {
      const on = element.dataset.pin !== undefined ? Boolean(state.pinned) && state.pinned.app_id === element.dataset.pin :
        !state.pinned && Number(element.dataset.index) === state.focus;
      element.classList.toggle("sel", on);
      element.setAttribute("aria-selected", String(on));
    }
    const current = $("#results .sel");
    if (current) current.scrollIntoView({ block: "nearest" });
  }
  function renderPreview() {
    const split = isSplit();
    $("#preview").hidden = !split;
    if (!split) return;
    const entry = selected();
    const recent = Boolean(entry) && !("categories" in entry);
    $("#preview-icon").replaceChildren(...(entry ? [glyph(entry, 56)] : []));
    $("#selected-name").textContent = entry ? entry.name || entry.app_id : T.selectApp;
    $("#selected-id").textContent = entry ? entry.app_id : "";
    $("#description").textContent = entry ? entry.comment || (recent ? T.recentDescription : T.noDescription) : "";
    const metadata = [];
    if (entry) {
      for (const [label, value] of [
        [T.genericName, entry.generic_name], [T.categoriesLabel, entry.categories], [T.keywords, entry.keywords],
        [T.visibilityLabel, entry.hidden ? T.hiddenValue : entry.no_display ? "NoDisplay" : recent ? T.notReported : T.visible],
        [T.terminal, entry.terminal === undefined ? "" : entry.terminal ? T.yes : T.no],
        [T.lastLaunch, entry.last_launched_at ? when(entry.last_launched_at) : ""], [T.recentCount, entry.count],
      ]) {
        if (value === undefined || value === null || value === "") continue;
        metadata.push(ui.node("dt", label), ui.node("dd", value));
      }
    }
    ui.replace($("#metadata"), metadata);
    $("#launch").hidden = !entry;
    $("#launch").disabled = !has("open");
    $("#running").hidden = !entry || !has("is-running");
    $("#running").disabled = state.checking;
    note($("#running-state"), state.running && state.running.text, state.running && state.running.ok ? "ok" : "");
    if (state.running && !state.running.ok) $("#running-state").prepend(ui.node("span", null, { class: "dot idle" }));
  }
  function renderFooter() {
    const launch = state.launching;
    $("#status").textContent = launch ? "" : T.status(visible().length, selected() ? 1 : 0);
    note($("#notice"), state.notice && state.notice.text, state.notice && state.notice.error ? "error" : "");
    note($("#outcome"), state.outcome && state.outcome.text, state.outcome ? state.outcome.ok ? "ok" : "error" : "");
    const keys = launch ? [] : [kb("↑↓", T.select), kb("↩", T.launchDots), kb("esc", T.escape)];
    if (!launch && state.mode === "list") keys.push(kb("Ctrl I", T.info));
    ui.replace($("#keys"), keys);
    $("#raw-panel").hidden = !state.raw;
    $("#raw-toggle").setAttribute("aria-pressed", String(state.raw));
  }
  function renderHead() {
    const launch = state.launching;
    const query = $("#query").value;
    const token = !launch && !query && state.explicit ? state.mode === "recent" ? T.recent : T.all : "";
    $("#qwrap").dataset.value = query;
    $("#query-field").classList.toggle("blank", !query);
    $("#query-field").hidden = Boolean(launch);
    $("#query").placeholder = token ? "" : T.placeholder;
    $("#mode-token").hidden = !token;
    $("#mode-token").textContent = token;
    $("#search-mode").replaceChildren(launch ? glyph(launch, 26) :
      svg(token ? state.mode === "recent" ? "clock" : "grid" : "search"));
    const top = state.mode === "find" && query.trim() ? visible()[0] : null;
    $("#completion").textContent = top ? `— ${top.name || top.app_id}` : "";
    $("#clear").hidden = !query || Boolean(launch);
    $("#more").hidden = Boolean(launch);
    $("#more").setAttribute("aria-expanded", String(state.menu && !launch));
    const head = $("#launch-head");
    head.hidden = !launch;
    head.replaceChildren(...(launch ? [
      ui.node("span", T.launchWord), ui.node("span", launch.name || launch.app_id, { class: "token blue" }),
      ui.node("span", T.withWord), ui.node("span", T.targets, { class: "token ghost" }),
      ui.node("span", `— ${T.run}`, { class: "run-lbl" }),
    ] : []));
  }
  function renderChips() {
    $("#chips").hidden = Boolean(state.launching);
    $("#all").classList.toggle("on", state.mode === "list");
    $("#recent").classList.toggle("on", state.mode === "recent");
    $("#all").disabled = !has("list");
    $("#recent").disabled = !has("recent");
    $("#all-count").textContent = state.total === null ? "" : String(state.total);
    $("#limit-hint").textContent = state.mode === "find" ? T.limitHint(limit()) : "";
    const counts = new Map();
    for (const entry of state.entries) {
      const key = categoryOf(entry);
      if (key) counts.set(key, (counts.get(key) || 0) + 1);
    }
    const keys = Array.from(counts.keys()).sort((a, b) => counts.get(b) - counts.get(a)).slice(0, 5);
    const nodes = [];
    if (keys.length > 1) {
      nodes.push(ui.node("span", null, { class: "chip-sep" }));
      for (const key of keys) {
        const chip = ui.node("button", null, { type: "button", class: `chip${state.filter === key ? " on" : ""}`, "aria-pressed": String(state.filter === key) });
        chip.append(ui.node("span", categoryLabel(key)), ui.node("span", String(counts.get(key)), { class: "n" }));
        chip.onclick = () => {
          state.filter = state.filter === key ? "" : key;
          state.focus = 0;
          state.pinned = null;
          changedSelection();
          render();
        };
        nodes.push(chip);
      }
    }
    ui.replace($("#categories"), nodes);
  }
  function render() {
    const launch = state.launching;
    renderHead();
    renderChips();
    $("#body").hidden = Boolean(launch);
    $("#body").classList.toggle("split", isSplit());
    $("#launch-form").hidden = !launch;
    $("#launch-submit").disabled = state.submitting;
    $("#menu").hidden = !state.menu || Boolean(launch);
    setChecked($("#hidden"), state.hidden);
    setChecked($("#no-display"), state.noDisplay);
    setChecked($("#raw-option"), state.raw);
    $("#no-display").hidden = native;
    renderResults();
    renderPreview();
    renderFooter();
  }

  function changedSelection() {
    selection++;
    state.running = null;
  }
  function focusAt(index) {
    state.pinned = null;
    state.focus = index;
    changedSelection();
    markSelection();
    renderPreview();
    renderFooter();
  }
  function pin(record) {
    const match = state.entries.find((entry) => entry.app_id === record.app_id);
    const index = match ? visible().indexOf(match) : -1;
    if (index >= 0) return focusAt(index);
    state.pinned = record;
    changedSelection();
    markSelection();
    renderPreview();
    renderFooter();
  }
  function move(delta) {
    const count = visible().length;
    if (!count) return;
    focusAt(Math.max(0, Math.min(count - 1, (state.pinned ? 0 : state.focus) + delta)));
  }
  function columns() {
    const grid = $("#main-grid");
    return grid ? getComputedStyle(grid).gridTemplateColumns.split(" ").filter(Boolean).length || 1 : 1;
  }

  async function load(mode, explicit) {
    const version = ++generation;
    if (mode !== "find" || state.mode !== "find") state.entries = [];
    Object.assign(state, { mode, explicit: Boolean(explicit), filter: "", focus: 0, pinned: null, loading: true, error: false,
      notice: { text: T.loading } });
    changedSelection();
    render();
    const args = {};
    if (mode === "find") Object.assign(args, { query: $("#query").value, limit: limit() });
    if (mode === "recent") args.limit = 20;
    if (mode === "list") {
      args.include_hidden = state.hidden;
      if (!native) args.include_no_display = state.noDisplay;
    }
    try {
      const value = checked(await call(mode, args));
      if (version !== generation) return;
      const entries = mode === "recent" ? value.recent : mode === "find" ? value.matches : value.apps;
      if (!Array.isArray(entries)) throw new Error(T.noList);
      state.entries = entries;
      if (mode === "list") state.total = entries.length;
      const count = entries.length;
      state.notice = { text: mode === "recent" ? T.recentListed(count) : mode === "find" ? T.matched(count) : T.listed(count) };
    } catch (error) {
      if (version !== generation) return;
      state.entries = [];
      state.error = true;
      state.notice = { text: errorText(error), error: true };
    }
    state.loading = false;
    render();
  }
  function loadRecentRow() {
    if (!has("recent")) return;
    call("recent", { limit: 7 }).then((value) => {
      state.recentRow = value && Array.isArray(value.recent) ? value.recent : [];
      render();
    }, () => {});
  }
  function browse(mode) {
    clearTimeout(searchTimer);
    $("#query").value = "";
    void load(mode, true);
    $("#query").focus();
  }

  function openLaunch(entry) {
    if (!entry || !has("open") || state.submitting) return;
    state.launching = entry;
    state.menu = false;
    $("#launch-app-id").value = entry.app_id;
    for (const id of ["#launch-uri", "#launch-path", "#launch-extras"]) $(id).value = "";
    $("#uri-field").hidden = native;
    $("#path-field").hidden = native;
    $("#extras-field").hidden = !native;
    note($("#launch-notice"), "");
    render();
    (native ? $("#launch-extras") : $("#launch-uri")).focus();
  }
  function closeLaunch() {
    if (state.submitting) return;
    state.launching = null;
    render();
    $("#query").focus();
  }
  $("#launch-form").onsubmit = (event) => {
    event.preventDefault();
    const target = state.launching;
    if (!target || state.submitting) return;
    const args = { app_id: target.app_id };
    // Empty target lists are omitted: a present `path` argument requests file grants.
    for (const [name, field] of native ? [["extras", "#launch-extras"]] : [["uri", "#launch-uri"], ["path", "#launch-path"]]) {
      const values = lines($(field).value);
      if (values.length) args[name] = values;
    }
    state.submitting = true;
    $("#launch-submit").disabled = true;
    note($("#launch-notice"), T.working);
    call("open", args).then(checked).then((value) => {
      state.submitting = false;
      state.outcome = value.launched === true && value.app_id === target.app_id ?
        { ok: true, text: T.launched(value.name || target.name || target.app_id, value.launched_at ? when(value.launched_at) : "") } :
        { ok: false, text: T.unconfirmed };
      state.launching = null;
      render();
      loadRecentRow();
      $("#query").focus();
    }).catch((error) => {
      state.submitting = false;
      $("#launch-submit").disabled = false;
      note($("#launch-notice"), errorText(error), "error");
    });
  };
  $("#launch-cancel").onclick = closeLaunch;
  $("#launch").onclick = () => openLaunch(selected());
  $("#running").onclick = () => {
    const entry = selected();
    if (!entry || state.checking) return;
    const version = selection;
    const id = entry.app_id;
    state.checking = true;
    $("#running").disabled = true;
    call("is-running", { app_id: id }).then(checked).then((value) => {
      if (version !== selection || value.app_id !== id) return;
      if (!Array.isArray(value.pids)) throw new Error(T.noObservation);
      state.running = value.running ? { ok: true, text: T.running(value.pids.join(", ")) } : { ok: false, text: T.notRunning };
    }).catch((error) => {
      if (version === selection) state.notice = { text: errorText(error), error: true };
    }).finally(() => {
      state.checking = false;
      renderPreview();
      renderFooter();
    });
  };

  $("#query").addEventListener("input", () => {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      if ($("#query").value.trim()) {
        if (has("find")) void load("find");
      } else if (state.mode === "find") void load("list");
    }, 160);
    renderHead();
  });
  $("#all").onclick = () => browse("list");
  $("#recent").onclick = () => browse("recent");
  $("#clear").onclick = () => browse("list");
  $("#more").onclick = () => {
    state.menu = !state.menu;
    render();
  };
  document.addEventListener("click", (event) => {
    if (state.menu && !event.target.closest("#menu, #more")) {
      state.menu = false;
      render();
    }
  });
  function toggleVisibility(key) {
    state[key] = !state[key];
    if (state.mode === "list") void load("list", state.explicit);
    else render();
  }
  $("#hidden").onclick = () => toggleVisibility("hidden");
  $("#no-display").onclick = () => toggleVisibility("noDisplay");
  $("#raw-option").onclick = $("#raw-toggle").onclick = () => {
    state.raw = !state.raw;
    render();
  };
  function setLimit(value) {
    $("#limit").value = String(Math.min(50, Math.max(1, value)));
    if (state.mode === "find") void load("find");
    else render();
  }
  $("#limit-down").onclick = () => setLimit(limit() - 1);
  $("#limit-up").onclick = () => setLimit(limit() + 1);
  $("#limit").onchange = () => setLimit(limit());

  document.addEventListener("keydown", (event) => {
    const ctrl = event.ctrlKey || event.metaKey;
    const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
    if (key === "Escape") {
      event.preventDefault();
      if (state.menu) {
        state.menu = false;
        render();
      } else if (state.launching) closeLaunch();
      else if ($("#query").value) browse("list");
      return;
    }
    if (state.launching) {
      if (ctrl && key === "Enter") {
        event.preventDefault();
        $("#launch-form").dispatchEvent(new Event("submit", { cancelable: true }));
      }
      return;
    }
    if (ctrl && key === "1" && has("list")) return event.preventDefault(), browse("list");
    if (ctrl && key === "2" && has("recent")) return event.preventDefault(), browse("recent");
    if (ctrl && key === "i" && state.mode === "list") {
      event.preventDefault();
      state.info = !state.info;
      render();
      return;
    }
    if (ctrl && key === "p") {
      event.preventDefault();
      if (isSplit() && !$("#running").hidden) $("#running").click();
      return;
    }
    if (event.target !== $("#query") && /^(INPUT|TEXTAREA)$/.test(event.target.tagName)) return;
    const grid = !isSplit();
    const delta = key === "ArrowDown" ? grid ? columns() : 1 : key === "ArrowUp" ? grid ? -columns() : -1 :
      grid && key === "ArrowRight" ? 1 : grid && key === "ArrowLeft" ? -1 : 0;
    if (delta) {
      event.preventDefault();
      move(delta);
    } else if (key === "Enter" && (event.target === $("#query") || event.target.closest("#results"))) {
      event.preventDefault();
      openLaunch(selected());
    }
  });

  render();
  $("#query").focus();
  if (has("list")) void load("list");
  loadRecentRow();
});
