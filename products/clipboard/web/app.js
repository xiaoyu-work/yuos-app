YuOsApp.ready.then((app) => {
  "use strict";
  const ui = YuOsUi;
  const $ = (id) => document.getElementById(id);
  const locale = globalThis.navigator?.language || "en";
  const chinese = /^zh(?:-|$)/i.test(locale);
  const tr = (en, zh) => chinese ? zh : en;
  const previewLabel = tr("Preview is plain text. Hiding it does not clear the system selection.", "预览仅显示纯文本。隐藏预览不会清除系统选区。");
  if (chinese) {
    document.documentElement.lang = "zh";
    for (const node of document.querySelectorAll("[data-zh]")) node.textContent = node.dataset.zh;
    for (const node of document.querySelectorAll("[data-zh-placeholder]")) node.placeholder = node.dataset.zhPlaceholder;
  }
  let busy = false;
  let reviewing = false;
  let selectionVersion = 0;
  ui.showApp(app);
  if (app.id === "panel-clipboard") {
    $("selection-workspace").hidden = true;
    $("native-history").hidden = false;
    $("history-nav").hidden = false;
    $("history-actions").hidden = false;
    $("title").textContent = tr("CopyQ history", "CopyQ 历史");
    $("history-refresh").disabled = true;
    historyWorkspace();
    return;
  }
  if (app.id !== "clipboard-manager") {
    $("selection-workspace").hidden = true;
    $("notice").textContent = tr("This identity has no supported clipboard interface.", "此应用身份不提供剪贴板界面。");
    return;
  }
  $("selection-workspace").hidden = false;
  $("selection-nav").hidden = false;
  $("selection-actions").hidden = false;
  $("title").textContent = selectionName(selection());

  function historyWorkspace() {
    const relative = new Intl.RelativeTimeFormat(locale, { style: "short" });
    let selected = null;
    let loading = false;
    function select(entry) {
      selected = entry;
      $("history-preview").textContent = entry ? entry.preview : tr("No history item selected.", "未选择历史条目。");
      $("history-time").textContent = entry?.copied_at || tr("Copy time not reported.", "未报告复制时间。");
      $("history-identity").textContent = entry ? entry.identity : "";
      $("history-restore").disabled = loading || !entry;
      $("history-remove").disabled = loading || !entry;
      for (const control of $("history-items").querySelectorAll("button")) {
        control.setAttribute("aria-pressed", String(control.dataset.identity === (entry && entry.identity)));
      }
    }
    function hide(message = tr("History previews are hidden locally. Load history explicitly to view it again.", "本地预览已隐藏。请重新载入历史以查看。")) {
      ui.replace($("history-items"), [ui.node("p", message, { class: "list-message" })]);
      select(null);
      $("history-refresh").disabled = true;
    }
    function age(time) {
      const number = /^\d+$/.test(time) ? Number(time) : NaN;
      const date = Number.isFinite(number) ? (number > 1e10 ? number : number * 1000) : Date.parse(time);
      if (!Number.isFinite(date)) return time;
      const minutes = Math.max(0, Math.floor((Date.now() - date) / 60000));
      if (minutes < 60) return relative.format(-minutes, "minute");
      if (minutes < 1440) return relative.format(-Math.floor(minutes / 60), "hour");
      return relative.format(-Math.floor(minutes / 1440), "day");
    }
    function show(value) {
      if (!value || !Array.isArray(value.entries) || value.entries.length > 20 ||
          value.entries.some((entry) => !entry || typeof entry.identity !== "string" ||
            !/^[A-Za-z0-9+/]{43}=$/.test(entry.identity) || typeof entry.preview !== "string" ||
            Array.from(entry.preview).length > 181 || entry.preview.includes("\0") ||
            entry.copied_at !== null && typeof entry.copied_at !== "string")) {
        hide(tr("The service returned an invalid history response. No entries are displayed.", "服务返回的历史格式无效，未显示任何条目。"));
        throw new Error(tr("Invalid OS clipboard history result.", "系统剪贴板历史返回结果无效。"));
      }
      select(null);
      const controls = value.entries.map((entry, index) => {
        const control = ui.node("button", `${index + 1}. ${entry.preview || tr("(Whitespace-only text)", "（仅空白文本）")}`, {
          class: "history-row", type: "button", "aria-pressed": "false",
        });
        control.dataset.identity = entry.identity;
        control.onclick = () => select(entry);
        if (entry.copied_at) control.append(ui.node("span", age(entry.copied_at), { class: "age" }));
        return control;
      });
      ui.replace($("history-items"), controls.length ? controls : [
        ui.node("p", tr("CopyQ returned no eligible text history items.", "CopyQ 历史中没有可显示的文本条目。"), { class: "list-message" }),
      ]);
      $("history-state").textContent = tr(`${controls.length} text previews · up to 20, 180 characters each`, `${controls.length} 条文本预览 · 最多 20 条，每条最多 180 字`);
    }
    async function load(control) {
      if (loading) return;
      loading = true;
      hide(tr("Reading CopyQ history…", "正在读取 CopyQ 历史…"));
      $("history-hide").disabled = true;
      $("history-clear").disabled = true;
      $("history-load").disabled = true;
      try {
        const value = await ui.run(control, () => {
          const tool = YuOsApp.tool(app, "history.list");
          if (!tool) throw new Error(tr("This identity does not declare history.list.", "此应用身份未声明 history.list。"));
          return YuOsApp.call(tool.name, {});
        }, show, $("notice"));
        if (value === undefined) {
          hide(tr("History could not be loaded. The service error is shown above.", "无法载入历史。服务错误见上方。"));
          $("history-state").textContent = tr("History unavailable or invalid.", "历史不可用或结果无效。");
        }
      } finally {
        loading = false;
        $("history-hide").disabled = false;
        $("history-clear").disabled = false;
        $("history-load").disabled = false;
        $("history-refresh").disabled = false;
        select(selected);
      }
    }
    function review(command) {
      if (loading || command !== "history.clear" && !selected) return;
      const identity = selected && selected.identity;
      try {
        ui.command(app, command, command === "history.clear" ? {} : { identity }, (value) => {
          hide(tr("History changed or may now be stale. Load it explicitly before another item action.", "历史已改变或可能过期。再次操作前请先载入。"));
          $("history-state").textContent = tr("Action response received; no automatic retry or refresh.", "已收到操作响应，不会自动重试或刷新。");
          ui.output($("history-result"), value);
          $("notice").textContent = tr("Inspect history again before another change. No undo is promised.", "再次更改前请先查看历史。操作无法保证撤销。");
        }, {
          title: command === "history.clear" ? tr("Clear CopyQ history?", "清空 CopyQ 历史？") :
            command === "history.restore" ? tr("Restore selected history item?", "恢复所选历史条目？") : tr("Delete selected history item?", "删除所选历史条目？"),
          description: command === "history.restore" ?
            tr("This replaces the live clipboard. The OS separately requires clipboard.write:selection as well as history read/write. It does not read the current selection.", "这会替换当前剪贴板。除了历史读写权限，系统还单独要求 clipboard.write:selection。不会读取当前选区。") :
            command === "history.clear" ?
              tr("This permanently removes every item from CopyQ's configured clipboard history tab. Custom tabs and the live selection are not changed.", "这会永久删除 CopyQ 配置的剪贴板历史标签页中的所有条目，不影响自定义标签页或当前选区。") :
              tr("This deletes only the selected unchanged history item. If it changed or disappeared, refresh instead of guessing its position.", "只会删除仍未变化的所选历史条目。如果条目已变更或消失，请刷新后再操作。"),
          submitLabel: command === "history.clear" ? tr("Clear history", "清空历史") :
            command === "history.restore" ? tr("Restore to clipboard", "恢复到剪贴板") : tr("Delete history item", "删除历史条目"),
          fields: {
            identity: { label: tr("Selected item identity", "所选条目标识"), readOnly: true },
            confirm: { label: tr("I confirm clearing this existing clipboard history.", "我确认清空此剪贴板历史。") },
          },
        });
        const dialog = document.querySelector?.("dialog[open]");
        if (dialog) {
          dialog.toggleAttribute("data-destructive", command !== "history.restore");
          dialog.querySelector(".toolbar button[type=button]").textContent = tr("Cancel", "取消");
        }
      } catch (error) { $("notice").textContent = error.message; }
    }
    $("history-load").onclick = (event) => { void load(event.currentTarget); };
    $("history-refresh").onclick = (event) => { void load(event.currentTarget); };
    $("history-hide").onclick = () => { hide(); $("history-state").textContent = tr("Only local previews were hidden. System history and selection are unchanged.", "只隐藏了本地预览，系统历史和选区均未更改。"); };
    $("history-restore").onclick = () => review("history.restore");
    $("history-remove").onclick = () => review("history.remove");
    $("history-clear").onclick = () => review("history.clear");
  }

  function detail(value) {
    if (value === null || value === undefined) return ui.node("p", tr("Not reported", "未报告"), { class: "muted" });
    if (typeof value !== "object") return ui.node("p", String(value), { class: "prose" });
    const box = ui.node("div", null, { class: "stack" });
    if (Array.isArray(value)) {
      if (!value.length) box.append(ui.node("p", tr("No records returned.", "未返回记录。"), { class: "muted" }));
      for (const item of value) box.append(detail(item));
    } else {
      const grid = ui.node("dl", null, { class: "detail-grid" });
      for (const [key, item] of Object.entries(value)) {
        grid.append(ui.node("dt", key.replaceAll("_", " ")));
        const cell = ui.node("dd"); cell.append(detail(item)); grid.append(cell);
      }
      box.append(grid.children.length ? grid : ui.node("p", tr("No additional fields returned.", "未返回其他字段。"), { class: "muted" }));
    }
    return box;
  }
  function selection(withMime = false) {
    const args = { primary: $("selection-primary").checked };
    const mime = $("mime").value.trim();
    if (withMime && mime) {
      if (mime.length > 255 || !mime.includes("/") || mime.startsWith("-") || /[\s\u0000-\u001f]/.test(mime)) throw new Error(tr("Enter a MIME type without whitespace or controls, or leave it blank for the OS default.", "请输入不含空白或控制字符的 MIME 类型，或留空以使用系统默认值。"));
      args.mime = mime;
    }
    return args;
  }
  function selectionName(args) { return args.primary ? tr("Primary selection", "主选区") : tr("Clipboard selection", "剪贴板选区"); }
  function evidence(title, value) {
    $("result-title").textContent = title;
    ui.replace($("result"), [detail(value)]);
  }
  async function request(control, command, args, done, status = $("notice"), guardSelection = true) {
    if (busy) return;
    busy = true;
    const version = selectionVersion;
    try {
      await ui.run(control, () => {
        const tool = YuOsApp.tool(app, command);
        if (!tool) throw new Error(tr(`This entry does not declare ${command}.`, `此应用入口未声明 ${command}。`));
        return YuOsApp.call(tool.name, args);
      }, (value) => { if (!guardSelection || version === selectionVersion) done(value); }, status);
    } finally { busy = false; }
  }
  function clearPreview() {
    $("content").textContent = tr("No selection content is displayed.", "未显示选区内容。");
    $("content-label").textContent = previewLabel;
  }
  function reset() {
    selectionVersion++;
    clearPreview();
    ui.replace($("types"), [ui.node("p", tr("Formats have not been loaded for this selection.", "尚未载入此选区的格式。"), { class: "muted" })]);
    evidence(tr("Clipboard evidence", "剪贴板凭据"), tr("Selection or MIME changed. Read again explicitly; no clipboard access occurred.", "选区或 MIME 类型已更改。未访问剪贴板，请按需重新读取。"));
  }
  function changeSelection() {
    const { primary } = selection();
    $("title").textContent = selectionName({ primary });
    $(primary ? "nav-primary" : "nav-clipboard").setAttribute("aria-current", "true");
    $(primary ? "nav-clipboard" : "nav-primary").removeAttribute("aria-current");
    reset();
  }
  function review(command, args) {
    if (busy || reviewing) return;
    reviewing = true;
    const dialog = ui.node("dialog"); const form = ui.node("form");
    const check = ui.node("input", null, { type: "checkbox", required: "" });
    const label = ui.node("label", tr(`I confirm ${command === "clear" ? "clearing" : "replacing"} the ${selectionName(args).toLowerCase()}.`, `我确认${command === "clear" ? "清除" : "替换"}${selectionName(args)}。`)); label.append(check);
    const status = ui.node("p", "", { class: "notice", role: "status", "aria-live": "polite" });
    const cancel = ui.node("button", tr("Cancel", "取消"), { type: "button" });
    const execute = ui.node("button", command === "clear" ? tr("Clear selection", "清除选区") : tr("Write source file", "写入源文件"), { type: "submit", class: "danger" });
    const actions = ui.node("div", null, { class: "toolbar" }); actions.append(cancel, execute);
    form.append(ui.node("p", command === "clear" ?
      tr("This clears only the selected Wayland selection, not CopyQ history. No undo is promised.", "只清除所选 Wayland 选区，不影响 CopyQ 历史。无法保证撤销。") :
      tr("This replaces the selected Wayland selection from the exact authorized source file. It does not import history or use the browser clipboard.", "使用获准的指定源文件替换所选 Wayland 选区，不导入历史，也不使用浏览器剪贴板。")),
      detail({ [tr("selection", "选区")]: selectionName(args), [tr("source", "源文件")]: args.source || tr("Not applicable", "不适用"), MIME: args.mime || tr("OS default (argument omitted)", "系统默认（未传入参数）") }), label, status, actions);
    form.onsubmit = async (event) => {
      event.preventDefault();
      if (!check.checked || busy) return;
      cancel.disabled = true;
      await request(execute, command, command === "clear" ? { ...args, confirm: true } : args, (value) => {
        clearPreview();
        evidence(`${command === "clear" ? tr("Clear response", "清除响应") : tr("Write response", "写入响应")}: ${selectionName(args)}`, value);
        $("notice").textContent = tr("Response received. Read or inspect the selection explicitly before another change; no automatic retry.", "已收到响应。再次更改前请主动读取或检查选区，不会自动重试。");
        dialog.close();
      }, status, false);
      cancel.disabled = false; check.checked = false;
      if (dialog.open) status.append(ui.node("span", tr(" The effect may be unknown. Inspect the selection before retrying.", " 结果可能未知。请检查选区后再重试。")));
    };
    cancel.onclick = () => dialog.close();
    dialog.addEventListener("cancel", (event) => { if (busy) event.preventDefault(); });
    dialog.addEventListener("close", () => { reviewing = false; dialog.remove(); });
    dialog.append(ui.node("h2", command === "clear" ? tr("Clear selection?", "清除选区？") : tr("Review clipboard write", "检查剪贴板写入")), form);
    document.body.append(dialog); dialog.showModal();
  }
  $("selection-clipboard").onchange = changeSelection;
  $("selection-primary").onchange = changeSelection;
  $("nav-clipboard").onclick = () => $("selection-clipboard").click();
  $("nav-primary").onclick = () => $("selection-primary").click();
  $("mime").oninput = reset;
  $("hide-content").onclick = clearPreview;
  $("read").onclick = (event) => {
    try {
      const args = selection(true);
      void request(event.currentTarget, "read", args, (value) => {
        $("content-label").textContent = tr(`${selectionName(args)} content (explicit read; may now be stale)`, `${selectionName(args)}内容（显式读取，可能已过期）`);
        const empty = tr("(Empty text returned.)", "（返回空文本。）");
        if (value && typeof value.text === "string") $("content").textContent = value.text || empty;
        else if (typeof value === "string") $("content").textContent = value || empty;
        else {
          $("content").textContent = tr("The service did not return a text field. Binary or unavailable content is not decoded or rendered here.", "服务没有返回文本字段。这里不会解码或显示二进制或不可用内容。");
          evidence(tr("Read response metadata", "读取响应元数据"), value);
        }
      });
    } catch (error) { $("notice").textContent = error.message; }
  };
  $("status").onclick = (event) => {
    const args = selection();
    void request(event.currentTarget, "status", args, (value) => evidence(tr(`${selectionName(args)} status`, `${selectionName(args)}状态`), value));
  };
  $("load-types").onclick = (event) => {
    const args = selection();
    void request(event.currentTarget, "types", args, (value) => {
      const types = value && Array.isArray(value.types) ? value.types : null;
      if (!types) { evidence(tr("Format response", "格式响应"), value); return; }
      const buttons = [];
      for (const mime of types) {
        if (typeof mime !== "string") continue;
        const control = ui.node("button", mime, { type: "button", "aria-pressed": String($("mime").value === mime) });
        control.onclick = () => {
          $("mime").value = mime;
          clearPreview();
          selectionVersion++;
          for (const button of buttons) button.setAttribute("aria-pressed", String(button === control));
        };
        buttons.push(control);
      }
      ui.replace($("types"), buttons.length ? buttons : [ui.node("p", tr("No MIME formats returned for this selection.", "此选区未返回 MIME 格式。"), { class: "muted" })]);
    });
  };
  $("write-form").onsubmit = (event) => {
    event.preventDefault();
    try {
      const source = $("source").value.trim();
      if (!source.startsWith("/") || source.length > 4096 || /[\u0000-\u001f]/.test(source)) throw new Error(tr("Choose an absolute canonical non-symlink source path on the OS filesystem.", "请选择系统文件中的绝对规范路径，不能使用符号链接。"));
      review("write", { ...selection(true), source });
    } catch (error) { $("notice").textContent = error.message; }
  };
  $("clear").onclick = () => review("clear", selection());
}).catch((error) => {
  document.getElementById("notice").textContent = `Clipboard UI unavailable: ${error.message}`;
});
