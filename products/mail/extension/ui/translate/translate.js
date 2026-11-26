// Translate popup. Entry points:
//   - context menu (selection → translate)
//   - context menu (message → translate)
//   - opened from the assistant page
//
// If `storage.local.pending` carries selectionText or messageIds, we
// seed the source area. Otherwise the user pastes / types text.

const { aiCall, showBusy, clearBusy, showError, el, localiseDom, copyToClipboard } = window.YuOsUI;
const { getBody } = window.YuOsMessages;

let lastTranslation = null;

document.addEventListener("DOMContentLoaded", async () => {
  localiseDom();

  const settings = await YuOsUI.getSettings();
  const target = settings.translate.defaultTarget || "English";
  document.getElementById("target").value = target;

  document.getElementById("btn-translate").addEventListener("click", run);
  document.getElementById("btn-refresh").addEventListener("click", run);

  await seedFromPending();
});

async function seedFromPending() {
  const pending = await browser.runtime.sendMessage({ kind: "getPending" });
  if (!pending || pending.kind !== "translate") return;
  await browser.runtime.sendMessage({ kind: "clearPending" });

  if (pending.selectionText) {
    document.getElementById("source").value = pending.selectionText;
    return;
  }
  if (pending.messageIds && pending.messageIds.length) {
    try {
      const body = await getBody(pending.messageIds[0]);
      document.getElementById("source").value = body?.plain || "";
    } catch (_) {}
  }
}

async function run() {
  const out = document.getElementById("out");
  const source = document.getElementById("source").value;
  const target = document.getElementById("target").value;

  if (!source.trim()) {
    showError(out, browser.i18n.getMessage("err_empty_source") || "Source is empty.");
    return;
  }
  if (!target.trim()) {
    showError(out, browser.i18n.getMessage("err_no_target") || "Pick a target language.");
    return;
  }

  await YuOsUI.setSettings({ translate: { defaultTarget: target } });

  showBusy(out);

  const res = await aiCall("translate", { text: source, target });
  clearBusy(out);
  if (!res || !res.ok) {
    showError(out, res?.error || "AI call failed", res?.detail);
    return;
  }
  lastTranslation = res.result;
  renderTranslation(out, res.result);
}

function renderTranslation(out, r) {
  out.innerHTML = "";
  const card = el("div", { class: "translation" }, [
    el("div", { text: r.translation || "" }),
  ]);
  out.appendChild(card);
  out.appendChild(el("div", { class: "footer-row" }, [
    el("button", { class: "ghost",   onclick: () => copy(r.translation), text: browser.i18n.getMessage("action_copy")  || "Copy" }),
  ]));
}

async function copy(text) {
  try { await copyToClipboard(text || ""); } catch (_) {}
}

