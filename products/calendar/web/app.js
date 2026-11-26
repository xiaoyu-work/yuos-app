const CALENDAR_TEXT = /^zh\b/i.test(navigator.language || "") ? {
  lang: "zh-CN", today: "今天", newEvent: "新建日程", invite: "邀请", calendars: "日历", previous: "上一月", next: "下一月",
  refresh: "刷新", moreTools: "更多 App 工具", toolsHeading: "更多 App 工具", run: "运行",
  providers: { local: "本地日历", google: "Google", outlook: "Outlook" },
  tools: { invite: "邀请他人", respond: "回复邀请", local_event: "查看本地日程", google_event: "查看 Google 日程",
    outlook_event: "查看 Outlook 日程", list: "列出日程", create: "创建日程", update: "更新日程", delete: "删除日程", today: "今日日程" },
  count: (n) => `${n} 个日程`, status: (provider, n) => `${provider} · 本月共 ${n} 个日程`, more: (n) => `还有 ${n} 个`,
  noEvents: "这一天没有日程。", select: "选择一个日程以查看详情。", untitled: "（无标题日程）", notSpecified: "未指定",
  noDescription: "没有描述。", allDay: "全天", attendees: "参与者", deleted: "日程已删除。", gone: "此日程已不在所选范围内。",
  invalidDate: "请选择有效的日期和时间。", noList: "此入口未声明日历列表工具。",
  localInvite: "请选择 Google 或 Outlook 发送邀请。本地日程不会发送邮件。",
  field: { provider: "日历", id: "日程", title: "标题", start: "开始", end: "结束", description: "描述", location: "地点",
    attendees: "邀请", response: "你的回复", comment: "附言", from: "从", to: "到", date: "日期",
    confirm: "我确认此邀请或回复将发送给日历服务商。" },
  placeholder: { title: "日程标题", location: "添加地点", description: "添加描述", attendees: "alice@example.com, bob@example.com" },
  responses: { accepted: "已接受", tentative: "待定", declined: "已拒绝", pending: "等待回复", organizer: "组织者" },
  edit: "编辑", respond: "回复邀请", delete: "删除", editEvent: "编辑日程", deleteEvent: "删除日程？",
  deleteDescription: (title, provider) => `从 ${provider} 删除“${title}”。`, respondTitle: "回复邀请", schedule: "安排会议",
  create: "创建日程", save: "保存更改", sendResponse: "发送回复", sendInvites: "创建并发送邀请",
  readOnly: "只读", panelLabel: "只读的本地日历日程。账户日历和日程编辑属于独立的日历入口。",
  loading: "正在加载所选日期的日程…", unavailable: "日历不可用", agendaUnavailable: "日程不可用。上方的服务错误可能表示访问被拒绝或 OS 服务不可用。",
  invalidAgenda: "日历服务返回了无效的日程。", noDayService: "此 App 未声明当日日程服务。",
} : {
  lang: "en", today: "Today", newEvent: "New Event", invite: "Invite", calendars: "Calendars", previous: "Previous month",
  next: "Next month", refresh: "Refresh", moreTools: "More App tools", toolsHeading: "Additional App tools", run: "Run",
  providers: { local: "Local calendar", google: "Google", outlook: "Outlook" },
  tools: { invite: "Invite people", respond: "Respond to invitation", local_event: "Open local event", google_event: "Open Google event",
    outlook_event: "Open Outlook event", list: "List events", create: "Create event", update: "Update event", delete: "Delete event",
    today: "Today's events" },
  count: (n) => `${n} ${n === 1 ? "event" : "events"}`, status: (provider, n) => `${provider} · ${n} ${n === 1 ? "event" : "events"} this month`,
  more: (n) => `${n} more`, noEvents: "No events on this day.", select: "Select an event to inspect it.", untitled: "(Untitled event)",
  notSpecified: "Not specified", noDescription: "No description.", allDay: "All day", attendees: "Attendees", deleted: "Event deleted.",
  gone: "This event is no longer in the selected range.", invalidDate: "Choose a valid date and time.",
  noList: "This entry does not declare the Calendar event-list tool.",
  localInvite: "Choose Google or Outlook to send invitations. Local events do not send mail.",
  field: { provider: "Calendar", id: "Event", title: "Title", start: "Starts", end: "Ends", description: "Description", location: "Location",
    attendees: "Invite", response: "Your response", comment: "Comment", from: "From", to: "To", date: "Date",
    confirm: "I confirm this invitation or response will be sent to the provider." },
  placeholder: { title: "Event title", location: "Add location", description: "Add description", attendees: "alice@example.com, bob@example.com" },
  responses: { accepted: "Accepted", tentative: "Maybe", declined: "Declined", pending: "Pending", organizer: "Organizer" },
  edit: "Edit", respond: "Respond", delete: "Delete", editEvent: "Edit Event", deleteEvent: "Delete event?",
  deleteDescription: (title, provider) => `Delete "${title}" from ${provider}.`, respondTitle: "Respond to Invitation",
  schedule: "Schedule a Meeting", create: "Create Event", save: "Save Changes", sendResponse: "Send Response",
  sendInvites: "Create and Send Invitations", readOnly: "Read-only",
  panelLabel: "Read-only local Calendar agenda. Account calendars and event editing belong to the separate Calendar entry.",
  loading: "Loading the selected day's agenda…", unavailable: "Calendar unavailable",
  agendaUnavailable: "Agenda unavailable. The service error above may indicate denied access or an unavailable OS service.",
  invalidAgenda: "The Calendar service returned an invalid agenda.", noDayService: "This App does not declare the day-agenda service.",
};

function calendarView(ui, T) {
  "use strict";
  const $ = (id) => document.getElementById(id);
  const key = (date) => `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
  const icon = (name) => {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
    use.setAttribute("href", `#i-${name}`);
    svg.append(use);
    return svg;
  };
  const when = (value) => {
    if (!value) return T.notSpecified;
    const date = new Date(value);
    if (!Number.isFinite(date.getTime())) return value;
    return /T|:/.test(value) ? date.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" })
      : date.toLocaleDateString(undefined, { dateStyle: "medium" });
  };
  const time = (value) => {
    const date = new Date(value);
    return Number.isFinite(date.getTime()) ? date.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit", hour12: false }) : "";
  };
  document.documentElement.lang = T.lang;
  for (const element of document.querySelectorAll("[data-text]")) element.textContent = T[element.dataset.text];
  for (const element of document.querySelectorAll("[data-label]")) {
    element.setAttribute("aria-label", T[element.dataset.label]);
    element.title = T[element.dataset.label];
  }
  function weekdays(target, format, className) {
    ui.replace(target, Array.from({ length: 7 }, (_, index) => ui.node("span",
      new Date(2026, 10, 1 + index).toLocaleDateString(undefined, { weekday: format }), className ? { class: className } : undefined)));
  }
  function monthStart(month) {
    const start = new Date(month.getFullYear(), month.getMonth(), 1);
    start.setDate(1 - start.getDay());
    return start;
  }
  function agendaRow(event, selected, onclick) {
    const button = ui.node("button", null, { type: "button", class: selected ? "selected" : "" });
    const text = ui.node("span");
    text.append(ui.node("strong", event.title || T.untitled));
    if (event.location) text.append(ui.node("span", event.location, { class: "sub" }));
    const times = ui.node("span", null, { class: "time" });
    if (event.start && !/T|:/.test(event.start)) times.append(T.allDay);
    else times.append(time(event.start), ui.node("br"), event.end ? time(event.end) : "");
    button.append(ui.node("span", null, { class: "stripe" }), text, times);
    button.onclick = onclick;
    return button;
  }
  function detailGrid(event) {
    const grid = ui.node("dl", null, { class: "detail-grid" });
    for (const [label, value] of [[T.field.start, when(event.start)], [T.field.end, when(event.end)],
      [T.field.location, event.location || T.notSpecified]]) grid.append(ui.node("dt", label), ui.node("dd", value));
    return grid;
  }
  function hint(target, text) {
    ui.replace(target, [ui.node("p", text, { class: "muted" })]);
  }
  return { $, key, icon, when, time, weekdays, monthStart, agendaRow, detailGrid, hint };
}

YuOsApp.ready.then((app) => {
  "use strict";
  const ui = YuOsUi;
  const T = CALENDAR_TEXT;
  const view = calendarView(ui, T);
  const { $, key, icon } = view;
  ui.showApp(app);
  if (app.id === "panel-calendar") {
    panelCalendar(app, ui, T, view);
    return;
  }
  let selectedDay = new Date();
  let month = new Date(selectedDay.getFullYear(), selectedDay.getMonth(), 1);
  let events = [];
  let selectedEvent = null;
  let loaded = false;
  let generation = 0;
  const notice = $("notice");
  const provider = $("provider");
  const providerName = () => T.providers[provider.value] || provider.value;
  const dateTime = (value) => {
    const date = new Date(value);
    if (!Number.isFinite(date.getTime())) return "";
    const pad = (number) => String(number).padStart(2, "0");
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
  };
  const timestamp = (value) => {
    const date = new Date(value);
    if (!Number.isFinite(date.getTime())) throw new Error(T.invalidDate);
    return date.toISOString();
  };
  const labels = Object.fromEntries(Object.entries(T.field).map(([name, label]) => [name, { label }]));
  labels.description = { label: T.field.description, type: "textarea" };
  const fields = (showProvider) => ({
    provider: showProvider ? { label: T.field.provider, choices: [provider.value] } : { label: T.field.provider, hidden: true },
    id: { label: T.field.id, hidden: true },
    title: { label: T.field.title, type: "text", placeholder: T.placeholder.title },
    start: { label: T.field.start, type: "datetime-local", format: dateTime, parse: timestamp },
    end: { label: T.field.end, type: "datetime-local", format: dateTime, parse: timestamp },
    description: { label: T.field.description, type: "textarea", rows: 3, includeEmpty: true, placeholder: T.placeholder.description },
    location: { label: T.field.location, type: "text", includeEmpty: true, placeholder: T.placeholder.location },
    attendees: { label: T.field.attendees, type: "text", placeholder: T.placeholder.attendees },
    response: { label: T.field.response },
    comment: { label: T.field.comment },
    confirm: { label: T.field.confirm },
  });
  const choiceNames = { ...T.providers, accepted: T.responses.accepted, tentative: T.responses.tentative, declined: T.responses.declined };
  function nameChoices() {
    const dialog = document.querySelector("dialog[open]");
    if (!dialog) return;
    for (const option of dialog.querySelectorAll("select option")) {
      if (choiceNames[option.value]) option.textContent = choiceNames[option.value];
    }
  }
  const defaults = () => {
    const start = new Date(selectedDay.getFullYear(), selectedDay.getMonth(), selectedDay.getDate(), 9);
    return { provider: provider.value, start: start.toISOString(), end: new Date(start.getTime() + 3600000).toISOString() };
  };
  function edit(command, title, values, submitLabel) {
    ui.command(app, command, values, async () => { await load(); }, { title, submitLabel, fields: fields(true) });
    nameChoices();
  }
  function dayEvents(date) {
    const start = new Date(date.getFullYear(), date.getMonth(), date.getDate());
    const end = new Date(date.getFullYear(), date.getMonth(), date.getDate() + 1);
    return events.filter((item) => {
      const begins = new Date(item.start);
      const finishes = item.end ? new Date(item.end) : begins;
      return begins < end && (finishes > start || begins >= start);
    });
  }
  function select(event, date) {
    selectedEvent = event.id;
    if (date) selectedDay = date;
    render();
  }
  function responseStatus(value) {
    const normalized = { tentativelyAccepted: "tentative", needsAction: "pending", notResponded: "pending", none: "pending" }[value] || value || "pending";
    return { text: T.responses[normalized] || value, icon: { accepted: "check", tentative: "maybe", declined: "declined" }[normalized] || "pending" };
  }
  function details() {
    const target = $("details");
    const actions = $("event-actions");
    const event = selectedEvent && events.find((item) => item.id === selectedEvent);
    if (!event) {
      actions.hidden = true;
      return;
    }
    const source = ui.node("div", null, { class: "source" });
    source.append(ui.node("i"), providerName());
    const content = [ui.node("h2", event.title || T.untitled), source, view.detailGrid(event),
      ui.node("p", event.description || T.noDescription, { class: "prose" })];
    if ((event.attendees || []).length) {
      const people = ui.node("div", null, { class: "people" });
      people.append(ui.node("div", T.attendees, { class: "heading" }));
      for (const attendee of event.attendees) {
        const email = attendee.email || String(attendee);
        const status = responseStatus(attendee.response);
        const row = ui.node("div", null, { class: "person" });
        const badge = ui.node("span", null, { class: "status" });
        badge.append(icon(status.icon), status.text);
        row.append(ui.node("span", email.slice(0, 2), { class: "avatar" }), ui.node("span", email, { class: "email" }), badge);
        people.append(row);
      }
      content.push(people);
    }
    ui.replace(target, content);
    const values = { ...event, provider: provider.value };
    const buttons = [];
    const add = (command, label, symbol, run, className) => {
      if (!YuOsApp.tool(app, command)) return;
      const button = ui.node("button", null, { type: "button", class: className || "" });
      button.append(icon(symbol));
      if (className === "danger") { button.setAttribute("aria-label", label); button.title = label; } else button.append(label);
      button.onclick = run;
      buttons.push(button);
    };
    add("update", T.edit, "pen", () => edit("update", T.editEvent, values, T.save));
    if (provider.value !== "local") {
      add("respond", T.respond, "reply", () => edit("respond", T.respondTitle, values, T.sendResponse));
    }
    add("delete", T.delete, "trash", () => ui.command(app, "delete", values, async () => {
      selectedEvent = null;
      await load();
      view.hint(target, T.deleted);
    }, { title: T.deleteEvent, description: T.deleteDescription(event.title || event.id, providerName()),
      submitLabel: T.delete, fields: fields(false) }), "danger");
    ui.replace(actions, buttons);
    actions.hidden = !buttons.length;
  }
  function agenda() {
    $("day-title").textContent = selectedDay.toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric" });
    const values = dayEvents(selectedDay);
    $("day-count").textContent = values.length ? T.count(values.length) : "";
    ui.replace($("agenda"), values.length ? values.map((event) => view.agendaRow(event, event.id === selectedEvent, () => select(event)))
      : [ui.node("p", T.noEvents, { class: "empty-note" })]);
  }
  function mini() {
    $("mini-title").textContent = month.toLocaleDateString(undefined, { year: "numeric", month: "long" });
    const start = view.monthStart(month);
    const cells = Array.from({ length: 7 }, (_, index) => ui.node("span",
      new Date(2026, 10, 1 + index).toLocaleDateString(undefined, { weekday: "narrow" }), { class: "weekday" }));
    for (let index = 0; index < 42; index++) {
      const date = new Date(start); date.setDate(start.getDate() + index);
      const classes = [date.getMonth() !== month.getMonth() && "outside", key(date) === key(new Date()) && "today",
        key(date) === key(selectedDay) && "selected", dayEvents(date).length && "has"].filter(Boolean).join(" ");
      const button = ui.node("button", date.getDate(), { type: "button", class: classes, "aria-label": date.toLocaleDateString() });
      button.onclick = () => { selectedDay = date; render(); };
      cells.push(button);
    }
    ui.replace($("mini"), cells);
  }
  function render() {
    $("month").textContent = month.toLocaleDateString(undefined, { year: "numeric", month: "long" });
    $("app").style.setProperty("--provider", `var(--c-${provider.value})`);
    for (const button of $("providers").children) button.setAttribute("aria-checked", String(button.dataset.provider === provider.value));
    const start = view.monthStart(month);
    const cells = [];
    for (let index = 0; index < 42; index++) {
      const date = new Date(start); date.setDate(start.getDate() + index);
      const cell = ui.node("div", null, { class: "day" });
      if (key(date) === key(new Date())) cell.classList.add("today");
      if (date.getMonth() !== month.getMonth()) cell.classList.add("outside");
      if (key(date) === key(selectedDay)) cell.classList.add("selected");
      if (index % 7 === 0 || index % 7 === 6) cell.classList.add("weekend");
      const label = date.getDate() === 1 && date.getMonth() !== month.getMonth()
        ? date.toLocaleDateString(undefined, { month: "short", day: "numeric" }) : date.getDate();
      const day = ui.node("button", label, { type: "button", class: "day-number", "aria-label": date.toLocaleDateString() });
      day.onclick = () => { selectedDay = date; render(); };
      cell.append(day);
      const list = dayEvents(date);
      for (const event of list.slice(0, list.length > 3 ? 2 : 3)) {
        const button = ui.node("button", null, { type: "button", class: `event${event.id === selectedEvent ? " selected" : ""}`,
          title: event.title || T.untitled });
        button.append(ui.node("span", null, { class: "dot" }), ui.node("span", event.title || T.untitled, { class: "label" }));
        button.onclick = () => select(event, date);
        cell.append(button);
      }
      if (list.length > 3) {
        const more = ui.node("button", T.more(list.length - 2), { type: "button", class: "event more" });
        more.onclick = () => { selectedDay = date; render(); };
        cell.append(more);
      }
      cells.push(cell);
    }
    ui.replace($("calendar"), cells);
    mini();
    agenda();
    details();
    $("status").textContent = loaded ? T.status(providerName(), events.length) : providerName();
  }
  function load() {
    const request = ++generation;
    events = [];
    loaded = false;
    render();
    if (!YuOsApp.tool(app, "list")) {
      notice.textContent = T.noList;
      return;
    }
    return ui.run($("refresh"), () => YuOsApp.call("list", {
      provider: provider.value,
      from: new Date(month.getFullYear(), month.getMonth(), 1).toISOString(),
      to: new Date(month.getFullYear(), month.getMonth() + 1, 1).toISOString(),
    }), (result) => {
      if (request !== generation) return;
      events = ui.rows(result).sort((a, b) => new Date(a.start) - new Date(b.start));
      loaded = true;
      const kept = !selectedEvent || events.some((item) => item.id === selectedEvent);
      if (!kept) selectedEvent = null;
      render();
      if (!kept) view.hint($("details"), T.gone);
    }, notice);
  }
  function move(offset) {
    month = new Date(month.getFullYear(), month.getMonth() + offset, 1);
    selectedDay = new Date(month);
    void load();
  }
  $("previous").onclick = () => move(-1);
  $("next").onclick = () => move(1);
  $("mini-previous").onclick = () => move(-1);
  $("mini-next").onclick = () => move(1);
  $("today").onclick = () => {
    selectedDay = new Date();
    month = new Date(selectedDay.getFullYear(), selectedDay.getMonth(), 1);
    void load();
  };
  $("refresh").onclick = load;
  for (const value of ["local", "google", "outlook"]) {
    const button = ui.node("button", null, { type: "button", role: "radio", "data-provider": value });
    button.append(ui.node("span", null, { class: "swatch" }), T.providers[value], icon(value === "local" ? "device" : "cloud"));
    button.firstChild.style.background = `var(--c-${value})`;
    button.onclick = () => {
      if (provider.value === value) return;
      provider.value = value;
      provider.dispatchEvent(new Event("change"));
    };
    $("providers").append(button);
  }
  provider.onchange = () => {
    selectedEvent = null;
    view.hint($("details"), T.select);
    void load();
  };
  $("create").disabled = !YuOsApp.tool(app, "create");
  $("create").onclick = () => edit("create", T.newEvent, defaults(), T.create);
  if (YuOsApp.tool(app, "invite")) {
    $("invite").hidden = false;
    $("invite").onclick = () => {
      if (provider.value === "local") {
        notice.classList.remove("error");
        notice.textContent = T.localInvite;
        return;
      }
      edit("invite", T.schedule, defaults(), T.sendInvites);
    };
  }
  const menu = $("more");
  const moreButton = $("more-button");
  const groups = { invite: 0, respond: 0, local_event: 1, google_event: 1, outlook_event: 1 };
  const order = Object.keys(T.tools);
  const tools = [...app.tools].sort((a, b) => (order.indexOf(a.command) + 1 || 99) - (order.indexOf(b.command) + 1 || 99));
  const items = [ui.node("div", T.toolsHeading, { class: "heading" })];
  let group = null;
  for (const tool of tools) {
    const current = groups[tool.command] ?? 2;
    if (group !== null && current !== group) items.push(ui.node("hr"));
    group = current;
    const item = ui.node("button", null, { type: "button", role: "menuitem" });
    item.append(T.tools[tool.command] || tool.command, ui.node("code", tool.command));
    item.onclick = () => {
      menu.hidden = true;
      moreButton.setAttribute("aria-expanded", "false");
      ui.command(app, tool.name, defaults(), (value) => {
        selectedEvent = null;
        $("event-actions").hidden = true;
        ui.output($("details"), value);
      }, { title: T.tools[tool.command] || tool.command, submitLabel: T.run, fields: labels });
      nameChoices();
    };
    items.push(item);
  }
  ui.replace(menu, items);
  moreButton.hidden = !app.tools.length;
  moreButton.onclick = (event) => {
    event.stopPropagation();
    menu.hidden = !menu.hidden;
    moreButton.setAttribute("aria-expanded", String(!menu.hidden));
  };
  document.addEventListener("click", (event) => {
    if (!menu.hidden && !menu.contains(event.target)) {
      menu.hidden = true;
      moreButton.setAttribute("aria-expanded", "false");
    }
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !menu.hidden) {
      menu.hidden = true;
      moreButton.setAttribute("aria-expanded", "false");
      moreButton.focus();
    }
  });
  view.weekdays($("weekdays"), "short");
  view.hint($("details"), T.select);
  void load();
});

function panelCalendar(app, ui, T, view) {
  "use strict";
  const { $, key } = view;
  let selected = new Date();
  let month = new Date(selected.getFullYear(), selected.getMonth(), 1);
  let selectedEvent = null;
  let generation = 0;
  const dateKey = (date) => `${String(date.getFullYear()).padStart(4, "0")}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
  $("app").classList.add("panel");
  $("app").style.setProperty("--provider", "var(--c-local)");
  for (const id of ["provider", "create", "invite", "more-button"]) $(id).hidden = true;
  $("app-label").hidden = false;
  $("app-label").textContent = T.panelLabel;
  $("day-count").textContent = T.readOnly;
  view.weekdays($("weekdays"), "narrow");

  function renderMonth() {
    $("month").textContent = month.toLocaleDateString(undefined, { month: "long", year: "numeric" });
    $("day-title").textContent = selected.toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric", year: "numeric" });
    const first = view.monthStart(month);
    ui.replace($("calendar"), Array.from({ length: 42 }, (_, index) => {
      const date = new Date(first); date.setDate(first.getDate() + index);
      const cell = ui.node("div", null, {
        class: `day${key(date) === key(selected) ? " selected" : ""}${date.getMonth() !== month.getMonth() ? " outside" : ""}${key(date) === key(new Date()) ? " today" : ""}`,
      });
      const button = ui.node("button", date.getDate(), { type: "button", class: "day-number",
        "aria-label": date.toLocaleDateString(), "aria-pressed": String(key(date) === key(selected)) });
      button.onclick = () => { selected = date; void load(); };
      cell.append(button);
      return cell;
    }));
  }
  function show(events) {
    ui.replace($("agenda"), events.length ? events.map((event, index) => view.agendaRow(event, index === selectedEvent, () => {
      selectedEvent = index;
      show(events);
    })) : [ui.node("p", T.noEvents, { class: "empty-note" })]);
    const event = events[selectedEvent];
    if (event) ui.replace($("details"), [ui.node("h2", event.title || T.untitled), view.detailGrid(event)]);
    else view.hint($("details"), T.select);
  }
  async function load() {
    const request = ++generation;
    selectedEvent = null;
    renderMonth();
    $("refresh").disabled = true;
    $("notice").textContent = "";
    $("notice").classList.remove("error");
    ui.replace($("agenda"), [ui.node("p", T.loading, { class: "empty-note" })]);
    view.hint($("details"), T.select);
    try {
      if (!YuOsApp.tool(app, "panel-calendar.day")) throw new Error(T.noDayService);
      const result = await YuOsApp.call("panel-calendar.day", { date: dateKey(selected) });
      if (request !== generation) return;
      if (!Array.isArray(result.events)) throw new Error(T.invalidAgenda);
      show(result.events);
    } catch (error) {
      if (request !== generation) return;
      $("notice").textContent = `${T.unavailable}: ${error.message}`;
      $("notice").classList.add("error");
      ui.replace($("agenda"), [ui.node("p", T.agendaUnavailable, { class: "empty-note" })]);
    } finally {
      if (request === generation) $("refresh").disabled = false;
    }
  }
  function move(offset) {
    month = new Date(month.getFullYear(), month.getMonth() + offset, 1);
    selected = new Date(month);
    void load();
  }
  $("previous").onclick = () => move(-1);
  $("next").onclick = () => move(1);
  $("today").onclick = () => {
    selected = new Date();
    month = new Date(selected.getFullYear(), selected.getMonth(), 1);
    void load();
  };
  $("refresh").onclick = load;
  void load();
}
