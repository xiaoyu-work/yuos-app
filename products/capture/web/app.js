(() => {
  "use strict";
  const messages = {
    en: {
      app: "Capture",
      capturePage: "New Screenshot",
      historyPage: "This Session",
      sidebarNote: "Responses are kept only on this page and clear when it closes.",
      captureMode: "Capture mode",
      screen: "Entire Screen",
      screenNote: "Non-interactive · authorized portal",
      region: "Region",
      window: "Window",
      clipboard: "Clipboard",
      nativeOnly: "Native portal only",
      options: "Options",
      saveTo: "Save to",
      saveToNote: "Existing authorized directory",
      defaultPictures: "Default: Pictures",
      modal: "Request a modal portal dialog",
      directoryNote: "Leave blank to use the manifest's owner-resolved Pictures folder. If the Host cannot resolve it, the request fails. An explicit path must be an existing absolute Linux directory. This page does not create directories or choose destinations outside its grant.",
      capture: "Capture to a New PNG",
      working: "Capturing…",
      idle: "No capture requested",
      outputNote: "The service returns a saved path or explicit cancellation, not image bytes. This view shows neither a live desktop nor a file preview. A display, portal, authenticated session and screen/output grants are required.",
      pending: "Capture requested once",
      pendingDetail: "Waiting for a confirmed saved path or cancellation. If the call fails or times out, the outcome may be unknown; nothing is retried automatically.",
      scopeNote: "This entry always sends interactive=false. Screen selection, clipboard access, delay, cursor options and video recording are not available here. Access to this page does not grant screen access.",
      result: "Result",
      detail: "Detail",
      localTime: "Local response time",
      received: (time) => `Response received locally at ${time}.`,
      saved: "PNG path returned by the service",
      cancelled: "Capture cancelled",
      cancelledDetail: "The service explicitly reported cancellation. No saved screenshot is claimed.",
      unknown: "No saved output confirmed",
      unknownDetail: "The response contains neither an explicit cancellation nor a confirmed saved path. Inspect the response and destination before retrying.",
      failed: "Capture failed or unconfirmed",
      failedDetail: "See the service error. No saved path or cancellation was confirmed; inspect the destination before trying again.",
      historyEmpty: "No completed capture responses.",
      historyNote: "This is not a persistent screenshot library. Cancellations and unknown output are not recorded as saved screenshots.",
      raw: "Latest backend response",
      rawEmpty: "No backend response received.",
      footer: "Screenshots are taken through the system-authorized portal",
      invalidDirectory: "Enter an existing authorized absolute Linux directory, or leave it blank for the Host-resolved default.",
      noTool: "This entry declares no capture service.",
    },
    zh: {
      app: "截图",
      capturePage: "新截图",
      historyPage: "本页记录",
      sidebarNote: "响应仅保存在本页，关闭页面后清空。",
      captureMode: "截图方式",
      screen: "整个屏幕",
      screenNote: "非交互 · 经授权的系统截图",
      region: "区域",
      window: "窗口",
      clipboard: "剪贴板",
      nativeOnly: "仅原生 portal",
      options: "选项",
      saveTo: "保存到",
      saveToNote: "已授权的现有目录",
      defaultPictures: "默认：图片",
      modal: "请求模态 portal 对话框",
      directoryNote: "留空时使用应用清单指定的「图片」目录，由系统解析；若无法解析，请求将失败。填写时须使用已存在的 Linux 绝对路径。本页不会创建目录，也不会选择授权范围外的位置。",
      capture: "截取为新 PNG",
      working: "正在截取…",
      idle: "尚未截图",
      outputNote: "服务只返回保存路径或明确的取消结果，不返回图像数据，因此这里不显示实时桌面或文件预览。截图需要显示器、系统 portal、已认证会话，以及屏幕和输出授权。",
      pending: "已发送一次截图请求",
      pendingDetail: "正在等待确认保存路径或收到取消结果。若调用失败或超时，结果可能未知，不会自动重试。",
      scopeNote: "此入口始终发送 interactive=false，不提供屏幕选区、剪贴板访问、延时、光标选项或录屏。能访问本页不代表拥有屏幕访问权限。",
      result: "结果",
      detail: "详情",
      localTime: "本地响应时间",
      received: (time) => `本地收到响应的时间：${time}。`,
      saved: "服务返回的 PNG 路径",
      cancelled: "截图已取消",
      cancelledDetail: "服务明确报告取消，未确认保存截图。",
      unknown: "未确认保存截图",
      unknownDetail: "服务未明确报告取消，也未确认保存路径。请先检查响应和目标目录，再决定是否重试。",
      failed: "截图失败或结果未确认",
      failedDetail: "请查看服务错误。未确认保存路径或取消；请先检查目标目录，再决定是否重试。",
      historyEmpty: "还没有已完成的截图响应。",
      historyNote: "这不是持久截图库。取消和结果未知的请求不会记为已保存的截图。",
      raw: "最近后端响应",
      rawEmpty: "尚未收到后端响应。",
      footer: "截图通过系统授权的 portal 完成",
      invalidDirectory: "请输入已经存在且获授权的 Linux 绝对路径，或留空使用系统解析的默认目录。",
      noTool: "此入口未声明截图服务。",
    },
  };
  const chinese = /^zh(?:-|$)/i.test(navigator.language);
  const text = chinese ? messages.zh : messages.en;
  const ui = YuOsUi;
  const $ = (selector) => document.querySelector(selector);
  const form = $("#capture-form");
  const capture = $("#capture");
  const directory = $("#directory");
  const modal = $("#modal");
  const resultIcon = $(".output-icon svg");
  const notice = $("#notice");
  const history = [];
  const icons = { idle: "camera", pending: "clock", saved: "check", cancelled: "cancel", unknown: "warning", failed: "warning" };
  const timeFormat = new Intl.DateTimeFormat(chinese ? "zh-CN" : "en", { timeStyle: "medium" });

  document.documentElement.lang = chinese ? "zh-Hans" : "en";
  document.title = text.app;
  document.querySelectorAll("[data-text]").forEach((element) => {
    element.textContent = text[element.dataset.text];
  });

  function showPage(page) {
    for (const name of ["capture", "history"]) {
      $(`#${name}-view`).hidden = name !== page;
      if (name === page) $(`#nav-${name}`).setAttribute("aria-current", "page");
      else $(`#nav-${name}`).removeAttribute("aria-current");
    }
    $("#page-title").textContent = page === "capture" ? text.capturePage : text.historyPage;
  }

  function showResult(kind, detail, received) {
    resultIcon.firstElementChild.setAttribute("href", `#icon-${icons[kind]}`);
    $("#output").dataset.kind = kind;
    $("#result-title").textContent = text[kind];
    $("#result-detail").textContent = detail;
    $("#result-time").textContent = received ? text.received(received) : "";
    $("#result-time").hidden = !received;
    $("#output-note").hidden = kind === "idle";
  }

  function record(kind, detail) {
    const received = timeFormat.format(new Date());
    history.unshift({ kind, detail, received });
    history.splice(20);
    const count = $("#history-count");
    count.textContent = history.length;
    count.hidden = false;
    $("#history-empty").hidden = true;
    $("#history-table").hidden = false;
    ui.replace($("#history"), history.map((entry) => {
      const row = ui.node("tr", null, { "data-kind": entry.kind });
      const result = ui.node("td");
      const label = ui.node("span", null, { class: "history-result" });
      const icon = resultIcon.cloneNode(true);
      icon.firstElementChild.setAttribute("href", `#icon-${icons[entry.kind]}`);
      label.append(icon, ui.node("span", text[entry.kind]));
      result.append(label);
      row.append(result, ui.node("td", entry.detail, { class: "history-detail" }), ui.node("td", entry.received));
      return row;
    }));
    showResult(kind, detail, received);
  }

  $("#nav-capture").onclick = () => showPage("capture");
  $("#nav-history").onclick = () => showPage("history");
  showPage("capture");
  showResult("idle", text.outputNote);

  function setBusy(busy) {
    for (const control of [capture, directory, modal]) control.disabled = busy;
    $("#progress").hidden = !busy;
  }

  YuOsApp.ready.then((app) => {
    const tool = YuOsApp.tool(app, "capture");
    $("#app-id").textContent = app.id;
    capture.disabled = !tool;
    if (!tool) notice.textContent = text.noTool;

    form.onsubmit = async (event) => {
      event.preventDefault();
      if (capture.disabled || !tool || !form.reportValidity()) return;
      const path = directory.value.trim();
      if (path && (!path.startsWith("/") || path.includes("\0"))) {
        notice.textContent = text.invalidDirectory;
        directory.setAttribute("aria-invalid", "true");
        directory.focus();
        return;
      }
      const args = { interactive: false, modal: modal.checked };
      if (path) args.save_dir = path;
      directory.removeAttribute("aria-invalid");
      notice.textContent = "";
      setBusy(true);
      showResult("pending", text.pendingDetail);
      try {
        const value = await YuOsApp.call(tool.name, args);
        ui.output($("#raw"), value);
        $("#raw-empty").hidden = true;
        if (value?.error) throw new Error(String(value.error));
        if (value?.cancelled === true) record("cancelled", text.cancelledDetail);
        else if (value?.cancelled === false && typeof value.path === "string" && value.path) record("saved", value.path);
        else record("unknown", text.unknownDetail);
      } catch (error) {
        notice.textContent = error.message;
        showResult("failed", text.failedDetail);
      } finally {
        setBusy(false);
      }
    };
    directory.oninput = () => {
      if (!directory.hasAttribute("aria-invalid")) return;
      directory.removeAttribute("aria-invalid");
      notice.textContent = "";
    };
  });
})();
