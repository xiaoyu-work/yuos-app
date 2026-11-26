YuOsApp.ready.then((app) => {
  "use strict";
  const ui = YuOsUi;
  const $ = (selector) => document.querySelector(selector);
  const accounts = $("#accounts");
  const folders = $("#folders");
  const messages = $("#messages");
  const notice = $("#notice");
  const accountItems = new Map();
  const has = (name) => YuOsApp.tool(app, name);
  const dateLabel = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" });
  let selected = null;
  let folder = null;
  let folderItems = [];
  let foldersReady = false;
  let activeQuery = "";
  let generation = 0;
  let readGeneration = 0;
  const addresses = (value) => Array.isArray(value) ? value.join(", ") : value || "";
  const defaults = () => ({ account: selected?.account || accounts.value,
    folder: folder?.id, id: selected?.id });
  const fields = {
    account: { label: "Account", hidden: true },
    "source-account": { label: "Draft mailbox account", hidden: true },
    "sender-account": { label: "Sender account", type: "text" },
    id: { label: "Saved draft", hidden: true },
    "request-id": { label: "Submission receipt", hidden: true },
    review: { label: "Reviewed draft fingerprint", readOnly: true },
    to: { label: "To", type: "text", placeholder: "alice@example.com, bob@example.com" },
    cc: { label: "Cc", type: "text" },
    subject: { label: "Subject", type: "text" },
    body: { label: "Message", type: "textarea", rows: 10 },
    confirm: { label: "I have reviewed this draft and confirm it may be sent." },
  };
  function icon(name) {
    const namespace = "http://www.w3.org/2000/svg";
    const svg = document.createElementNS(namespace, "svg");
    const use = document.createElementNS(namespace, "use");
    svg.setAttribute("class", "mail-icon");
    svg.setAttribute("aria-hidden", "true");
    use.setAttribute("href", `#icon-${name}`);
    svg.append(use);
    return svg;
  }
  function action(label, symbol) {
    const button = ui.node("button", null, { type: "button", class: "mail-button" });
    button.append(icon(symbol), label);
    return button;
  }
  function setNotice(text, error = false) {
    notice.textContent = text;
    notice.classList.toggle("error", error);
  }
  async function mailboxRequest(command, args, current, onResult) {
    try {
      const result = await YuOsApp.call(command, args);
      if (!current()) return false;
      onResult(result);
      return true;
    } catch (error) {
      if (current()) setNotice(error.message, true);
      return false;
    }
  }
  function resetMessage(hint = "Select a message to read it.") {
    selected = null;
    ++readGeneration;
    $("#subject").textContent = "Message";
    $("#message-meta").hidden = true;
    $("#message").textContent = hint;
    $("#message").classList.add("muted");
    $("#message-actions").replaceChildren();
    $("#attachments").replaceChildren();
    $("#draft").hidden = true;
    messages.querySelector('[aria-pressed="true"]')?.setAttribute("aria-pressed", "false");
  }
  function aiResult(result) {
    const target = document.querySelector("#ai-result");
    if (typeof result === "string") {
      ui.replace(target, [ui.node("p", result, { class: "prose" })]);
      return;
    }
    const items = [];
    for (const [key, value] of Object.entries(result || {})) {
      if (["usage", "budget", "model", "provider"].includes(key)) continue;
      const section = ui.node("section", null, { class: "card stack" });
      section.append(ui.node("h3", key.replace(/[-_]/g, " ")));
      const values = Array.isArray(value) ? value : [value];
      for (const item of values) {
        if (item && typeof item === "object") {
          for (const [label, text] of Object.entries(item)) section.append(ui.node("p",
            `${label.replace(/[-_]/g, " ")}: ${Array.isArray(text) ? text.join("; ") : text}`, { class: "prose" }));
        } else section.append(ui.node("p", item, { class: "prose" }));
      }
      items.push(section);
    }
    if (result && result.model) items.push(ui.node("p", `Model: ${result.model}`, { class: "small muted" }));
    ui.replace(target, items);
  }
  function compose(values, replacing) {
    const command = has("draft_save") ? "draft_save" : "send";
    const context = { account: accounts.value, "request-id": crypto.randomUUID(), ...values };
    const request = readGeneration;
    ui.command(app, command, context, async (result) => {
      if (result.draft && request === readGeneration) {
        resetMessage("Review the draft before sending it.");
        await reviewDraft(result.draft);
      } else if (!result.draft) setNotice(result.sent ?
        "Submitted to Thunderbird. Delivery is not confirmed." : "Mail operation returned.");
    }, {
      title: replacing ? "Edit saved draft" : "Compose email",
      submitLabel: command === "draft_save" ? "Save draft" : "Send email",
      description: replacing ? "Save the edited plain-text body in Thunderbird. Existing attachments and other recipients are retained; HTML formatting is replaced by plain text. This does not send mail." :
        command === "draft_save" ? "Save in the selected sender's Thunderbird Drafts folder. This does not send mail." :
        "Review the recipients and complete message before sending.",
      fields,
    });
  }
  function showDraft(draft) {
    const message = draft.message;
    const target = document.querySelector("#draft-content");
    const details = ui.node("dl", null, { class: "detail-grid" });
    for (const [label, value] of [["From", message.from], ["To", addresses(message.to)],
      ["Cc", addresses(message.cc)], ["Bcc", addresses(message.bcc)], ["Subject", message.subject],
      ["Mailbox account", message.account]]) details.append(ui.node("dt", label), ui.node("dd", value || "(None)"));
    const content = [details, ui.node("p", message.body || "(Empty message)", { class: "prose" })];
    if (message.html) {
      const html = ui.node("details");
      html.append(ui.node("summary", "Complete HTML source included in this review"),
        ui.node("pre", message.html));
      content.push(html);
    }
    for (const attachment of message.attachments || []) content.push(ui.node("p",
      `${attachment.name} - ${attachment.size} bytes - ${attachment.sha256 || ""}`, { class: "small" }));
    content.push(ui.node("p", `Review: ${draft.review}`, { class: "small muted" }));
    ui.replace(target, content);
    const actions = document.querySelector("#draft-actions");
    actions.replaceChildren();
    const account = message.account;
    if (YuOsApp.tool(app, "draft_save")) {
      const edit = ui.node("button", "Edit draft");
      edit.onclick = () => compose({ account: accounts.value, id: draft.id,
        "source-account": account, to: addresses(message.to), cc: addresses(message.cc),
        subject: message.subject, body: message.body }, true);
      actions.append(edit);
    }
    if (YuOsApp.tool(app, "draft_send")) {
      const senders = ui.node("select", null, { "aria-label": "Send draft from account" });
      for (const item of accountItems.values()) {
        if (item.can_send && item.id.split(":")[0] === account.split(":")[0]) {
          senders.append(ui.node("option", item.name || item.id, { value: item.id }));
        }
      }
      if (Array.from(senders.options).some((option) => option.value === accounts.value)) senders.value = accounts.value;
      const send = ui.node("button", "Send reviewed draft", { class: "primary" });
      send.disabled = !senders.options.length;
      send.onclick = () => ui.command(app, "draft_send", {
        account, id: draft.id, "sender-account": senders.value, review: draft.review,
        "request-id": crypto.randomUUID(),
      }, (result) => {
        document.querySelector("#draft").hidden = true;
        document.querySelector("#message").textContent = result.sent ?
          "Submitted to Thunderbird. Delivery is not confirmed." : "Draft operation returned.";
        if (result.draft_removed === false) document.querySelector("#message").textContent +=
          " The source draft was retained; do not send it again.";
      }, { title: "Send reviewed draft?", submitLabel: "Confirm and send", fields: {
        ...fields, "sender-account": { label: "Sender account", readOnly: true },
      }, description: "Send the unchanged reviewed content above, including all recipients and attachments. Sending is not delivery proof." });
      actions.append(senders, send);
    }
    document.querySelector("#draft").hidden = false;
  }
  function reviewDraft(draft) {
    if (!has("native_draft")) return;
    const request = readGeneration;
    setNotice("Loading saved draft...");
    return mailboxRequest("native_draft", { id: draft.id }, () => request === readGeneration, (result) => {
      setNotice("");
      showDraft(result.draft);
      revealReader(true);
    });
  }
  function revealReader(draft = false) {
    if (draft || window.matchMedia("(max-width: 690px)").matches) {
      $(draft ? "#draft" : ".mail-reader").scrollIntoView({ block: "nearest" });
    }
  }
  function displayMessage(value, item) {
    selected = value;
    $("#subject").textContent = value.subject || item.subject || "(No subject)";
    $("#sender").textContent = value.from || "Unknown sender";
    const senderName = (value.from || "").split("<")[0].trim();
    $(".mail-avatar").textContent = senderName.match(/[\p{L}\p{N}]/u)?.[0].toLocaleUpperCase() || "?";
    $("#message-date").textContent = value.date ? new Date(value.date).toLocaleString() : "";
    $("#message-meta").hidden = false;
    $("#message").textContent = value.body || "(No plain-text body)";
    $("#message").classList.toggle("muted", !value.body);
    $("#ai-input").value = value.body || "";
    const attachments = $("#attachments");
    if (value.truncated) attachments.append(ui.node("p", "The displayed message is truncated.", { class: "notice" }));
    for (const attachment of value.attachments || []) {
      const row = ui.node("div", null, { class: "mail-attachment" });
      row.append(icon("clip"), ui.node("span", `${attachment.name} - ${attachment.size} bytes`, { class: "small" }));
      if (has("attachment_download")) {
        const download = ui.node("button", "Save attachment");
        download.onclick = () => ui.command(app, "attachment_download", {
          account: value.account || accounts.value, id: value.id, attachment: attachment.id,
        }, (result) => {
          row.append(ui.node("span", `Saved ${result.bytes} bytes to ${result.path}`, { class: "small muted" }));
          download.disabled = true;
        }, { title: `Save ${attachment.name}`, submitLabel: "Save attachment", fields: {
          ...fields, attachment: { label: "Attachment", hidden: true },
          output: { label: "Absolute destination path", type: "text", placeholder: "/home/user/Downloads/attachment.pdf" },
        }, description: "Choose an existing destination directory. An existing file is never overwritten." });
        row.append(download);
      }
      attachments.append(row);
    }
    const actions = $("#message-actions");
    if (!$("#compose").disabled) {
      const reply = action("Reply", "reply");
      reply.onclick = () => compose({ to: value.from, subject: /^re:/i.test(value.subject || "") ?
        value.subject : `Re: ${value.subject || ""}`, body: `\n\n${value.body || ""}` });
      actions.append(reply);
    }
    const source = folderItems.find((entry) => entry.id === value.folder);
    if (source?.special_use?.includes("drafts") && has("native_draft")) {
      const review = action("Review saved draft", "drafts");
      review.onclick = () => { void reviewDraft(value); };
      actions.append(review);
    }
    revealReader();
  }
  function list(result) {
    const items = ui.rows(result);
    const limit = Number($("#limit").value);
    $("#message-count").textContent = `${items.length} shown${items.length === limit ? ` (limit ${limit})` : ""}`;
    ui.replace(messages, items.length ? items.map((item) => {
      const subject = item.subject || "(No subject)";
      const sender = item.from || "Unknown sender";
      const button = ui.node("button", null, {
        type: "button", class: item.unread ? "mail-message unread" : "mail-message",
        "aria-pressed": "false", "aria-label": `${item.unread ? "Unread: " : ""}${sender}, ${subject}`,
      });
      const heading = ui.node("span", null, { class: "mail-row-heading" });
      heading.append(ui.node("span", sender, { class: "mail-from", title: sender }));
      if (item.date) {
        const date = new Date(item.date);
        if (!Number.isNaN(date.getTime())) heading.append(ui.node("time", dateLabel.format(date),
          { datetime: date.toISOString(), title: date.toLocaleString() }));
      }
      button.append(heading, ui.node("span", subject, { class: "mail-subject-line", title: subject }));
      button.disabled = !has("read");
      button.onclick = () => {
        resetMessage("Loading message...");
        const request = readGeneration;
        button.setAttribute("aria-pressed", "true");
        setNotice("");
        const read = mailboxRequest("read", { account: item.account || accounts.value, id: item.id },
          () => request === readGeneration, (value) => displayMessage(value, item));
        void read.then((shown) => {
          if (request === readGeneration && !shown) resetMessage("Could not open this message.");
        });
      };
      return button;
    }) : [ui.node("p", activeQuery ? "No messages match this search." : "No messages to show.", { class: "mail-empty" })]);
  }
  function listHint(text) {
    $("#message-count").textContent = "";
    ui.replace(messages, text ? [ui.node("p", text, { class: "mail-empty" })] : []);
  }
  function syncControls() {
    const canList = foldersReady && !!folder && has("list");
    const canSearch = foldersReady && has("search");
    $("#query").disabled = $("#search").disabled = !canSearch;
    $("#refresh").disabled = !(activeQuery ? canSearch : canList);
    $("#unread").disabled = !canList || !!activeQuery;
    $("#limit").disabled = !(canList || canSearch);
  }
  function load() {
    const command = activeQuery ? "search" : "list";
    if (!foldersReady || !has(command) || (!activeQuery && !folder)) return;
    const request = ++generation;
    resetMessage();
    $("#folder-title").textContent = activeQuery ? "Search results" : folder.name || folder.id;
    listHint("Loading messages...");
    setNotice("");
    const args = { account: accounts.value, "max-results": Number($("#limit").value) };
    if (folder) args.folder = folder.id;
    if (activeQuery) args.query = activeQuery;
    else args.unread = $("#unread").checked;
    const loading = mailboxRequest(command, args, () => request === generation, list);
    return loading.then((shown) => {
      if (request === generation && !shown) listHint("Could not load messages.");
    });
  }
  function selectFolder(entry, button) {
    folder = entry;
    folders.querySelector('[aria-pressed="true"]')?.setAttribute("aria-pressed", "false");
    button?.setAttribute("aria-pressed", "true");
    $("#query").value = activeQuery = "";
    $("#query").placeholder = $("#query").ariaLabel = folder ? "Search current folder" : "Search mailbox";
    $("#folder-title").textContent = folder ? folder.name || folder.id : "Messages";
    syncControls();
    if (folder && has("list")) void load();
    else {
      ++generation;
      setNotice("");
      resetMessage();
      listHint(folder ? "" : "This account has no folders.");
    }
  }
  async function selectAccount() {
    const request = ++generation;
    resetMessage();
    setNotice("");
    foldersReady = false;
    folder = null;
    folderItems = [];
    folders.replaceChildren();
    $("#query").value = activeQuery = "";
    $("#folder-title").textContent = "Messages";
    syncControls();
    listHint("Loading folders...");
    const item = accountItems.get(accounts.value);
    $("#compose").disabled = !item?.can_send || !(has("draft_save") || has("send"));
    if (!item || !has("folders")) {
      listHint("No mailbox folders are available.");
      return;
    }
    await mailboxRequest("folders", { account: item.id }, () => request === generation, (result) => {
      foldersReady = true;
      folderItems = ui.rows(result, ["folders"]);
      const folderIcons = { inbox: "inbox", drafts: "drafts", sent: "sent", archives: "archive", trash: "trash", junk: "junk" };
      const buttons = folderItems.map((entry) => {
        const symbol = Object.keys(folderIcons).find((key) => entry.special_use?.includes(key));
        const button = ui.node("button", null, { type: "button", class: "folder-button", "aria-pressed": "false" });
        button.append(icon(folderIcons[symbol] || "folder"), ui.node("span", entry.name || entry.id));
        button.onclick = () => selectFolder(entry, button);
        return button;
      });
      folders.replaceChildren(...buttons);
      const inbox = folderItems.findIndex((entry) => entry.special_use?.includes("inbox"));
      const index = Math.max(inbox, 0);
      selectFolder(folderItems[index], buttons[index]);
    });
    if (request === generation && !foldersReady) listHint("Could not load folders.");
  }
  accounts.onchange = () => { void selectAccount(); };
  $("#refresh").onclick = () => { void load(); };
  $("#unread").onchange = () => { void load(); };
  $("#limit").onchange = () => { void load(); };
  $("#search").onclick = () => {
    activeQuery = $("#query").value.trim();
    if (!activeQuery) {
      selectFolder(folder, folders.querySelector('[aria-pressed="true"]'));
      return;
    }
    syncControls();
    void load();
  };
  $("#query").oninput = () => {
    if (activeQuery && !$("#query").value.trim()) $("#search").click();
  };
  $("#query").onkeydown = (event) => {
    if (event.key === "Enter") { event.preventDefault(); $("#search").click(); }
  };
  $("#compose").onclick = () => compose();
  const aiCommands = { summarize: "Summarize message", smart_reply: "Suggest replies",
    smart_compose: "Help compose", translate: "Translate", triage: "Triage", chat: "Ask about mail", draft: "Draft from context" };
  for (const [command, label] of Object.entries(aiCommands)) {
    if (!YuOsApp.tool(app, command)) continue;
    const button = ui.node("button", label);
    button.onclick = () => ui.command(app, command, {
      body: document.querySelector("#ai-input").value, context: document.querySelector("#ai-input").value,
      subject: selected && selected.subject, sender: selected && selected.from,
    }, aiResult, { title: label, submitLabel: label, fields: {
      subject: fields.subject, body: fields.body, context: { label: "Writing context", type: "textarea", rows: 8 },
      sender: { label: "Message sender", type: "text" }, lang: { label: "Output language", type: "text" },
      question: { label: "Your question", type: "textarea" }, intent: { label: "What do you want to say?", type: "textarea" },
    } });
    document.querySelector("#ai-actions").append(button);
  }
  document.querySelector("#ai").hidden = !document.querySelector("#ai-actions").children.length;
  ui.allTools($("#more"), app, (value) => ui.output($("#tool-result"), value), defaults);
  if (has("accounts")) {
    void ui.run(null, () => YuOsApp.call("accounts"), (result) => {
      const values = ui.rows(result, ["accounts", "items"]);
      for (const item of values) accountItems.set(item.id, item);
      ui.replace(accounts, values.map((item) => ui.node("option", item.name || item.id, { value: item.id })));
      accounts.disabled = !values.length;
      if (values.length) void selectAccount();
      else listHint("No accounts yet. Add an account in Thunderbird to open a mailbox.");
    }, notice);
  } else {
    $("#mail-search").hidden = true;
    $("#mail-toolbar").hidden = true;
    $("#mailbox").hidden = true;
    setNotice("Mail AI processes only the content you supply. It does not read another App's mailbox.");
  }
});
