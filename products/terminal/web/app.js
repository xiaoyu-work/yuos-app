YuOsApp.ready.then((app) => {
  "use strict";
  const ui = YuOsUi;
  const $ = (selector) => document.querySelector(selector);
  const has = (command) => Boolean(YuOsApp.tool(app, command));
  const native = app.id === "yuos-term";
  const zh = /^zh\b/i.test(navigator.language || "");
  const T = zh ? {
    app: "终端", tools: "工具", runner: "命令运行器", scripts: "脚本", jobs: "后台任务", history: "请求历史",
    runnerLede: "结构化、需授权的命令运行器，不是交互式 PTY。提交后才会执行，没有实时输入或输出流。Web 超时可能导致结果未知，切勿盲目重跑修改类操作。",
    runProgram: "运行程序", program: "程序", programPh: "要执行的程序", locate: "定位", arguments: "参数", addArg: "添加参数",
    argsHelp: "每个参数一栏，按顺序传递；空参数也会保留。", mode: "运行方式", capture: "捕获输出", background: "后台任务",
    timeout: "超时", seconds: "秒", shell: "Shell 语法", shellOpt: "使用 Shell 语法（需显式开启）",
    shellHelp: "用空格拼接程序和参数，允许替换、管道和重定向；不再保留参数边界。",
    run: "运行程序", start: "启动后台任务", runShell: "执行 Shell 命令", openNative: "打开原生终端…",
    output: "捕获的输出", stdout: "标准输出", stderr: "标准错误", raw: "最近一次后端响应",
    scriptsLede: "只有在你明确选择时才运行脚本，不会自动生成或运行任何脚本。", runScript: "显式运行脚本", source: "来源",
    inline: "内联代码", scriptFile: "已授权的脚本文件", interpreter: "解释器", code: "代码",
    scriptHelp: "内联代码需要单独的通配执行授权。", runThisScript: "运行此脚本", refresh: "刷新",
    historyHelp: "本窗口中已完成的请求。“重用”只会填入表单，不会直接运行。",
    argLabel: "程序参数", remove: "移除", noRun: "尚未运行命令", noOutput: "暂无输出。", empty: "（空）",
    pending: "等待中——结果未确认", sentOnce: "请求只发送一次。如失败或超时，请先检查状态再重试。",
    noCapture: "后端未返回捕获的输出，执行结果未确认。",
    timedOut: (code) => `已超时${code == null ? "" : `——退出码 ${code}`}`, exitUnknown: "退出状态未知", exit: (code) => `退出码 ${code}`,
    timeoutInfo: "超时由后端报告。", captureInfo: "已完成的有限捕获，不是实时终端。", truncated: "后端截断了输出。",
    timeoutOutcome: "后端报告超时，不会自动重试。", exit0: "程序以退出码 0 结束。",
    exitOther: (code) => `程序返回 ${code ?? "未知退出状态"}，不视为成功。`,
    request: "请求", result: "结果", reuse: "重用", startedPid: (pid) => `已启动 PID ${pid}`,
    copied: "请求已填入表单。请检查后再提交执行。", noRegistry: "后端未返回进程登记表。",
    command: "命令", started: "启动时间", stop: "停止…", notReported: "未报告", cmdNotReported: "未报告命令",
    noJobs: "此入口的后台登记表中没有运行中的任务。",
    stopTitle: (pid) => `停止已登记的 PID ${pid}？`,
    stopBody: "向这个 PID 发送 SIGTERM。登记记录不能证明进程身份——进程可能已经退出，PID 也可能已被复用。不提供任意 PID 输入，也不会自动重试。",
    stopConfirm: "我已核对该登记进程，确认发送信号。", cancel: "取消", stopGo: (pid) => `向 PID ${pid} 发送信号`,
    refreshFirst: "请先刷新任务，获取新的登记快照后再进行下一次操作。",
    stopRequested: (pid) => `已对 PID ${pid} 发送一次停止请求，结果尚未确认。`,
    stopReported: (pid) => `后端报告已向 PID ${pid} 发送停止信号。这不能独立证明进程已退出。`,
    stopUnconfirmed: "停止结果未确认。请先刷新登记表再发起新请求。",
    stopFailed: (pid) => `停止 PID ${pid} 失败或未确认。请刷新登记表；不会自动重发信号。`,
    backgroundRequest: "后台请求", captured: "捕获执行", noPid: "未确认后台 PID。考虑再次启动前请先刷新任务。",
    bgOut: "此入口的工具不提供后台输出。", bgErr: "没有可用的实时后台输出流。",
    bgOutcome: (pid) => `后端报告 PID ${pid}。刷新任务以查看其登记状态。`,
    failed: "失败或结果未确认", failedOutcome: "未确认执行完成。再次执行前请先检查状态；不会自动重试。",
    enterName: "请先输入可执行文件名。", notFound: "未找到可执行文件。",
    scriptRequest: (lang) => `显式 ${lang} 脚本请求`, scriptFailed: "脚本失败或结果未确认", scriptFailedOutcome: "脚本结果未确认，不会自动重试。",
    loading: "正在读取登记表…", registryUnavailable: "进程登记表不可用。不会推断为空登记表，也无法选择任务。",
    jobsHelp: "仅显示此入口登记的后台任务，不包括原生 PTY 或主机上的全部进程。刷新需手动触发；此处无法读取后台输出。",
    jobsNative: "此原生入口没有后台登记表、任务列表或信号工具。其有限命令运行器与原生 PTY 相互独立。",
    refreshHint: "刷新以查看已登记的任务。", jobsUnavailable: "此入口不提供任务控制。", noHistory: "本页面尚无已完成的请求。",
    nativeOpened: "已报告激活原生终端，未向其发送任何命令。", nativeUnconfirmed: "未确认原生终端已激活。",
    nativeTitle: "打开原生终端窗口", nativeSubmit: "打开原生窗口",
    nativeDesc: "通过此入口固定的系统服务显式激活桌面终端，不是内嵌 PTY。不会自动执行任何命令。",
    cwd: "可选：已授权的绝对工作目录",
  } : {
    app: "Terminal", tools: "Tools", runner: "Command Runner", scripts: "Scripts", jobs: "Background Jobs", history: "Request History",
    runnerLede: "A structured, authorized command runner — not an interactive PTY. Nothing executes until you submit. There is no live stdin or output stream; a Web timeout may leave the outcome unknown, so never blindly rerun a mutation.",
    runProgram: "Run a Program", program: "Program", programPh: "Program to execute", locate: "Locate", arguments: "Arguments", addArg: "Add Argument",
    argsHelp: "One field per argument, in order; empty arguments are kept.", mode: "Mode", capture: "Capture Output", background: "Background Job",
    timeout: "Timeout", seconds: "seconds", shell: "Shell Syntax", shellOpt: "Use shell syntax (explicit opt-in)",
    shellHelp: "Joins the program and arguments with spaces and allows substitutions, pipelines and redirects. Argument boundaries are not preserved.",
    run: "Run Program", start: "Start Background Job", runShell: "Execute Shell Command", openNative: "Open Native Terminal…",
    output: "Captured Output", stdout: "Standard Output", stderr: "Standard Error", raw: "Latest Backend Response",
    scriptsLede: "Scripts run only when you choose to. Nothing is generated or run automatically.", runScript: "Run a Script Explicitly", source: "Source",
    inline: "Inline Code", scriptFile: "Authorized Script File", interpreter: "Interpreter", code: "Code",
    scriptHelp: "Inline code requires its own wildcard execution authority.", runThisScript: "Run This Script", refresh: "Refresh",
    historyHelp: "Completed requests from this window. Reuse fills the form only — it never runs the request.",
    argLabel: "Program argument", remove: "Remove", noRun: "No command run", noOutput: "No output yet.", empty: "(empty)",
    pending: "Pending - no confirmed result", sentOnce: "Request sent only once. If it fails or times out, inspect state before retrying.",
    noCapture: "The backend returned no captured output. The execution outcome is not confirmed.",
    timedOut: (code) => `Timed out${code == null ? "" : ` - exit ${code}`}`, exitUnknown: "Exit status unknown", exit: (code) => `Exit ${code}`,
    timeoutInfo: "The timeout was reported by the backend.", captureInfo: "Completed bounded capture, not a live terminal.", truncated: "The backend truncated output.",
    timeoutOutcome: "Backend reported a timeout. No automatic retry.", exit0: "Program exited with code 0.",
    exitOther: (code) => `Program returned ${code ?? "an unknown exit status"}; this is not reported as success.`,
    request: "Request", result: "Result", reuse: "Reuse", startedPid: (pid) => `Started PID ${pid}`,
    copied: "Request copied to the form. Review and submit to execute.", noRegistry: "The backend returned no process registry.",
    command: "Command", started: "Started", stop: "Stop…", notReported: "not reported", cmdNotReported: "Command not reported",
    noJobs: "No live jobs in this entry's background registry.",
    stopTitle: (pid) => `Stop registered PID ${pid}?`,
    stopBody: "Sends SIGTERM to this exact PID. Registry presence is not a process-identity token; the process may exit or the PID may be reused. No arbitrary PID field or automatic retry is provided.",
    stopConfirm: "I reviewed the selected registered process and want to send this signal.", cancel: "Cancel", stopGo: (pid) => `Signal PID ${pid}`,
    refreshFirst: "Refresh jobs to obtain a new registry snapshot before another control.",
    stopRequested: (pid) => `Stop requested once for PID ${pid}. Its outcome is not yet confirmed.`,
    stopReported: (pid) => `Backend reported the stop signal for PID ${pid}. This does not independently verify process exit.`,
    stopUnconfirmed: "The stop outcome was not confirmed. Refresh the registry before another request.",
    stopFailed: (pid) => `Stop of PID ${pid} failed or is unconfirmed. Refresh the registry; no signal is retried automatically.`,
    backgroundRequest: "Background request", captured: "Captured execution", noPid: "No background PID was confirmed. Refresh jobs before considering another start.",
    bgOut: "Background output is not exposed by this entry's tools.", bgErr: "No live background stream is available.",
    bgOutcome: (pid) => `Backend reported PID ${pid}. Refresh jobs to observe its registry state.`,
    failed: "Failed or unconfirmed outcome", failedOutcome: "No completed execution result confirmed. Inspect state before another execution; nothing is retried automatically.",
    enterName: "Enter an executable name first.", notFound: "no executable found.",
    scriptRequest: (lang) => `Explicit ${lang} script request`, scriptFailed: "Failed or unconfirmed script outcome", scriptFailedOutcome: "Script result not confirmed. No automatic retry.",
    loading: "Loading registry...", registryUnavailable: "Process registry unavailable. No empty registry is inferred and no job can be selected.",
    jobsHelp: "Only this exec entry's registry is shown, not native PTYs or all host processes. Refresh is explicit; background output files are not readable through these tools.",
    jobsNative: "This native entry has no background registry, job-list or signal tool. Its bounded command runner is separate from native PTYs.",
    refreshHint: "Refresh to inspect registered jobs.", jobsUnavailable: "Job controls are unavailable for this entry.", noHistory: "No requests completed in this page.",
    nativeOpened: "Native terminal activation reported. No command was sent to it.", nativeUnconfirmed: "Native activation was not confirmed.",
    nativeTitle: "Open a native terminal window", nativeSubmit: "Open native window",
    nativeDesc: "Explicit desktop activation through this entry's fixed OS service, not an embedded PTY. No command is automatically executed.",
    cwd: "Optional authorized absolute working directory",
  };
  let busy = false;
  let jobs = [];
  let jobsGeneration = 0;
  let view = "runner";
  const history = [];
  document.documentElement.lang = zh ? "zh-CN" : "en";
  document.title = T.app;
  document.querySelectorAll("[data-t]").forEach((node) => { node.textContent = T[node.dataset.t]; });
  document.querySelectorAll("[data-tp]").forEach((node) => { node.placeholder = T[node.dataset.tp]; });
  $("#exit").textContent = T.noRun;
  $("#stdout").textContent = T.noOutput;
  $("#stderr").textContent = T.noOutput;
  ui.showApp(app);

  function call(command, args = {}) {
    const tool = YuOsApp.tool(app, command);
    if (!tool) throw new Error(`This entry does not declare ${command}.`);
    return YuOsApp.call(tool.name, args);
  }
  function show(next) {
    view = next;
    document.querySelectorAll(".it[data-view]").forEach((item) => item.classList.toggle("on", item.dataset.view === next));
    $("#view-title").textContent = T[next];
    $("#v-runner").hidden = next !== "runner";
    $("#v-scripts").hidden = next !== "scripts";
    $("#v-jobs").hidden = next !== "jobs";
    $("#v-history").hidden = next !== "history";
    $("#v-output").hidden = next !== "runner" && next !== "scripts";
  }
  function segment(select) {
    let seg = select.nextElementSibling;
    if (!seg || !seg.classList.contains("seg")) {
      seg = ui.node("div", null, { class: "seg", role: "group" });
      select.after(seg);
    }
    seg.replaceChildren(...Array.from(select.options).map((option) => {
      const button = ui.node("button", option.textContent, { type: "button" });
      button.dataset.value = option.value;
      button.onclick = () => {
        if (busy || select.value === option.value) return;
        select.value = option.value;
        select.dispatchEvent(new Event("change"));
      };
      return button;
    }));
  }
  function syncSegments() {
    document.querySelectorAll("select.seg-source").forEach((select) => {
      const seg = select.nextElementSibling;
      seg.querySelectorAll("button").forEach((button) => {
        button.classList.toggle("on", button.dataset.value === select.value);
        button.disabled = busy || select.disabled;
      });
    });
  }
  function update() {
    const background = $("#mode").value === "start";
    $("#run-label").textContent = background ? T.start : $("#shell").checked ? T.runShell : T.run;
    $("#timeout").disabled = busy || background;
    $("#shell").disabled = busy || background;
    $("#refresh-jobs").disabled = busy || !has("ps");
    $("#which").disabled = busy || !has("which");
    $("#run").disabled = busy || !has(background ? "start" : "run");
    $("#code").required = $("#script-source").value === "inline";
    $("#script-file").required = $("#script-source").value === "file";
    $("#code-label").hidden = $("#script-source").value !== "inline";
    $("#script-file-label").hidden = $("#script-source").value !== "file";
    syncSegments();
  }
  async function perform(action, render, failed) {
    if (busy) return;
    busy = true;
    const controls = Array.from(document.querySelectorAll("button, input, select, textarea"))
      .filter((control) => !control.disabled && !control.closest(".side, #stream-seg"));
    controls.forEach((control) => { control.disabled = true; });
    const result = await ui.run(null, async () => {
      const value = await action();
      ui.output($("#raw"), value);
      if (value && value.error && !value.timed_out) throw new Error(String(value.error));
      return value;
    }, render, $("#notice"));
    if (result === undefined && failed) failed();
    busy = false;
    controls.forEach((control) => { control.disabled = false; });
    update();
  }
  function addArgument(value = "") {
    const row = ui.node("div", null, { class: "line" });
    const input = ui.node("input", null, { "aria-label": T.argLabel, autocomplete: "off", class: "mono" });
    input.value = value;
    const remove = ui.node("button", null, { type: "button", class: "rm", title: T.remove, "aria-label": T.remove });
    remove.innerHTML = '<svg><use href="#i-minus"/></svg>';
    remove.onclick = () => { if (!busy) { row.remove(); number(); } };
    row.append(ui.node("span", "", { class: "idx" }), input, remove);
    $("#arguments").append(row);
    number();
  }
  function number() {
    $("#arguments").querySelectorAll(".idx").forEach((node, index) => { node.textContent = String(index + 1); });
  }
  function argumentsList() {
    return Array.from($("#arguments").querySelectorAll("input")).map((input) => input.value);
  }
  function setExit(text, state = "") {
    $("#exit").textContent = text;
    $("#exit").className = `pill r ${state}`;
  }
  function clearOutput(label) {
    setExit(T.pending);
    $("#stdout").textContent = "";
    $("#stderr").textContent = "";
    $("#output-info").textContent = label;
    $("#outcome").textContent = T.sentOnce;
  }
  function renderOutput(value) {
    if (!value || typeof value.stdout !== "string" || typeof value.stderr !== "string") throw new Error(T.noCapture);
    $("#stdout").textContent = value.stdout || T.empty;
    $("#stderr").textContent = value.stderr || T.empty;
    if (value.timed_out) setExit(T.timedOut(value.exit_code), "bad");
    else if (value.exit_code == null) setExit(T.exitUnknown);
    else setExit(T.exit(value.exit_code), value.exit_code === 0 ? "ok" : "bad");
    $("#output-info").textContent = [value.timed_out ? T.timeoutInfo : T.captureInfo, value.truncated ? T.truncated : ""].filter(Boolean).join(" ");
    $("#outcome").textContent = value.timed_out ? T.timeoutOutcome : value.exit_code === 0 ? T.exit0 : T.exitOther(value.exit_code);
  }
  function header(labels) {
    const row = ui.node("div", null, { class: "lhead" });
    row.append(...labels.map((label) => ui.node("div", label)));
    return row;
  }
  function renderHistory() {
    if (!history.length) {
      ui.replace($("#history"), [ui.node("p", T.noHistory, { class: "empty" })]);
      return;
    }
    ui.replace($("#history"), [header([T.request, T.mode, T.result, ""]), ...history.map((entry) => {
      const row = ui.node("div", null, { class: "lrow" });
      const text = `${entry.command} ${entry.arguments.map((arg) => JSON.stringify(arg)).join(" ")}`.trim();
      const outcome = entry.pid ? T.startedPid(entry.pid) : T.exit(entry.exit ?? "?");
      const state = entry.pid || entry.exit === 0 ? "ok" : "bad";
      const pill = ui.node("span", outcome, { class: `pill ${state}` });
      const result = ui.node("div");
      result.append(pill);
      const button = ui.node("button", T.reuse, { type: "button" });
      button.onclick = () => {
        if (busy) return;
        $("#program").value = entry.command;
        $("#arguments").replaceChildren();
        entry.arguments.forEach(addArgument);
        $("#mode").value = entry.mode;
        $("#shell").checked = entry.shell === true;
        $("#outcome").textContent = T.copied;
        update();
        show("runner");
      };
      const action = ui.node("div", null, { class: "act" });
      action.append(button);
      row.append(ui.node("div", text, { class: "mono", title: text }), ui.node("div", entry.mode === "start" ? T.background : T.capture, { class: "dim" }), result, action);
      return row;
    })]);
  }
  function remember(request, value) {
    history.unshift({ ...request, exit: value.exit_code, pid: value.pid });
    history.splice(20);
    renderHistory();
  }
  function setJobsCount(count) {
    $("#jobs-count").hidden = count == null;
    $("#jobs-count").textContent = count == null ? "" : String(count);
  }
  function renderJobs(value) {
    if (!value || !Array.isArray(value.processes)) throw new Error(T.noRegistry);
    jobs = value.processes;
    jobsGeneration++;
    setJobsCount(jobs.length);
    if (!jobs.length) {
      ui.replace($("#jobs"), [ui.node("p", T.noJobs, { class: "empty" })]);
      return;
    }
    ui.replace($("#jobs"), [header(["PID", T.command, T.started, ""]), ...jobs.map((job) => {
      const row = ui.node("div", null, { class: "lrow" });
      const text = Array.isArray(job.command) ? job.command.map((arg) => JSON.stringify(arg)).join(" ") : T.cmdNotReported;
      const action = ui.node("div", null, { class: "act" });
      if (has("stop") && Number.isInteger(job.pid) && job.pid > 0) {
        const button = ui.node("button", T.stop, { type: "button", class: "danger" });
        const version = jobsGeneration;
        button.onclick = () => stopDialog(job, version);
        action.append(button);
      }
      row.append(ui.node("div", String(job.pid), { class: "mono" }), ui.node("div", text, { class: "mono", title: text }),
        ui.node("div", job.started || T.notReported, { class: "dim" }), action);
      return row;
    })]);
  }
  function stopDialog(job, version) {
    if (busy || version !== jobsGeneration || !jobs.includes(job)) return;
    const dialog = ui.node("dialog");
    dialog.append(ui.node("h2", T.stopTitle(job.pid)), ui.node("p", T.stopBody));
    const form = ui.node("form");
    const label = ui.node("label", null, { class: "chk" });
    const confirm = ui.node("input", null, { type: "checkbox", required: "" });
    label.append(confirm, ui.node("span", T.stopConfirm));
    const actions = ui.node("div", null, { class: "toolbar" });
    const cancel = ui.node("button", T.cancel, { type: "button" });
    const submit = ui.node("button", T.stopGo(job.pid), { type: "submit", class: "danger primary" });
    cancel.onclick = () => dialog.close();
    actions.append(cancel, submit);
    form.append(label, actions);
    form.onsubmit = (event) => {
      event.preventDefault();
      if (busy || !form.reportValidity() || version !== jobsGeneration || !jobs.includes(job)) return;
      jobs = [];
      jobsGeneration++;
      setJobsCount(null);
      ui.replace($("#jobs"), [ui.node("p", T.refreshFirst, { class: "empty" })]);
      $("#jobs-help").textContent = T.stopRequested(job.pid);
      void perform(() => call("stop", { pid: job.pid }), (value) => {
        $("#jobs-help").textContent = value.pid === job.pid && value.status === "stopped" ? T.stopReported(job.pid) : T.stopUnconfirmed;
      }, () => {
        $("#jobs-help").textContent = T.stopFailed(job.pid);
      }).finally(() => dialog.close());
    };
    dialog.append(form);
    dialog.addEventListener("close", () => dialog.remove());
    document.body.append(dialog);
    dialog.showModal();
  }

  document.querySelectorAll(".it[data-view]").forEach((item) => { item.onclick = () => show(item.dataset.view); });
  document.querySelectorAll("#stream-seg button").forEach((button) => {
    button.onclick = () => {
      document.querySelectorAll("#stream-seg button").forEach((other) => other.classList.toggle("on", other === button));
      $("#stdout").hidden = button.dataset.stream !== "stdout";
      $("#stderr").hidden = button.dataset.stream !== "stderr";
    };
  });
  $("#add-argument").onclick = () => { if (!busy) addArgument(); };
  $("#mode").onchange = update;
  $("#shell").onchange = update;
  $("#script-source").onchange = update;
  $("#language").onchange = update;
  $("#shell-option").hidden = native || !YuOsApp.tool(app, "run")?.args.some((arg) => arg.name === "shell");
  if (has("start")) $("#mode").append(ui.node("option", T.background, { value: "start" }));
  document.querySelectorAll("select.seg-source").forEach(segment);
  const timeoutArg = YuOsApp.tool(app, "run")?.args.find((arg) => arg.name === (native ? "timeout_secs" : "timeout"));
  $("#timeout").value = String(timeoutArg?.default ?? 30);
  if (native) $("#timeout").max = "600";
  $("#command").onsubmit = (event) => {
    event.preventDefault();
    if (busy || !$("#command").reportValidity()) return;
    const mode = $("#mode").value;
    const command = $("#program").value;
    const arguments_ = argumentsList();
    const args = { command, arguments: arguments_ };
    const shell = mode === "run" && !native && $("#shell").checked;
    if (mode === "run") {
      args[native ? "timeout_secs" : "timeout"] = Number($("#timeout").value);
      if (!native) args.shell = shell;
    }
    clearOutput(`${mode === "start" ? T.backgroundRequest : T.captured}: ${command}`);
    void perform(() => call(mode, args), (value) => {
      if (mode === "start") {
        if (!Number.isInteger(value.pid) || value.pid < 1) throw new Error(T.noPid);
        setExit(T.startedPid(value.pid), "ok");
        $("#stdout").textContent = T.bgOut;
        $("#stderr").textContent = T.bgErr;
        $("#outcome").textContent = T.bgOutcome(value.pid);
      } else renderOutput(value);
      remember({ command, arguments: arguments_, mode, shell }, value);
    }, () => {
      setExit(T.failed, "bad");
      $("#outcome").textContent = T.failedOutcome;
    });
  };
  $("#which").onclick = () => {
    const program = $("#program").value;
    if (!program.trim()) { $("#lookup").textContent = T.enterName; return; }
    $("#lookup").textContent = "";
    void perform(() => call("which", native ? { program } : { name: program }), (value) => {
      $("#lookup").textContent = value.path ? `${program}: ${value.path}` : `${program}: ${T.notFound}`;
    });
  };
  $("#scripts").hidden = !has("script");
  document.querySelector('.it[data-view="scripts"]').hidden = !has("script");
  $("#script-form").onsubmit = (event) => {
    event.preventDefault();
    if (busy || !$("#script-form").reportValidity()) return;
    const inline = $("#script-source").value === "inline";
    const args = { lang: $("#language").value, timeout: Number($("#script-timeout").value) };
    if (inline) args.code = [$("#code").value];
    else args.file = $("#script-file").value;
    clearOutput(T.scriptRequest(args.lang));
    void perform(() => call("script", args), renderOutput, () => {
      setExit(T.scriptFailed, "bad");
      $("#outcome").textContent = T.scriptFailedOutcome;
    });
  };
  $("#refresh-jobs").onclick = () => {
    jobs = [];
    jobsGeneration++;
    setJobsCount(null);
    $("#jobs-help").textContent = T.jobsHelp;
    ui.replace($("#jobs"), [ui.node("p", T.loading, { class: "empty" })]);
    void perform(() => call("ps"), renderJobs, () => {
      $("#jobs").replaceChildren(ui.node("p", T.registryUnavailable, { class: "empty" }));
    });
  };
  $("#jobs-help").textContent = has("ps") ? T.jobsHelp : T.jobsNative;
  $("#jobs").append(ui.node("p", has("ps") ? T.refreshHint : T.jobsUnavailable, { class: "empty" }));
  renderHistory();
  $("#desktop").hidden = !has("open");
  $("#desktop").onclick = () => {
    ui.command(app, "open", {}, (value) => {
      ui.output($("#raw"), value);
      $("#outcome").textContent = value.opened === true ? T.nativeOpened : T.nativeUnconfirmed;
    }, { title: T.nativeTitle, submitLabel: T.nativeSubmit, description: T.nativeDesc,
      fields: { cwd: { label: T.cwd, type: "text" } } });
    const cancel = document.querySelector("dialog[open] .toolbar button[type=button]");
    if (cancel) cancel.textContent = T.cancel;
  };
  show(view);
  update();
});
