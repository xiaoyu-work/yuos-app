// Options page. Read settings, render controls, write on Save.

const { localiseDom } = window.YuOsUI;

document.addEventListener("DOMContentLoaded", async () => {
  localiseDom();
  await loadSettings();
  document.getElementById("btn-save").addEventListener("click", save);
});

async function loadSettings() {
  const s = await YuOsUI.getSettings();

  for (const cb of document.querySelectorAll("[data-feature]")) {
    cb.checked = !!s.features[cb.dataset.feature];
  }

  document.getElementById("triage-autoTag").checked            = !!s.triage.autoTag;
  document.getElementById("triage-autoMoveImportant").checked  = !!s.triage.autoMoveImportant;
  document.getElementById("triage-autoMoveNewsletter").checked = !!s.triage.autoMoveNewsletter;
  document.getElementById("triage-tagPrefix").value            = s.triage.tagPrefix || "yuos/";

  document.getElementById("compose-defaultStyle").value       = s.compose.defaultStyle  || "formal";
  document.getElementById("translate-defaultTarget").value    = s.translate.defaultTarget || "English";
}

async function save() {
  const features = {};
  for (const cb of document.querySelectorAll("[data-feature]")) {
    features[cb.dataset.feature] = !!cb.checked;
  }

  const triage = {
    autoTag:            document.getElementById("triage-autoTag").checked,
    autoMoveImportant:  document.getElementById("triage-autoMoveImportant").checked,
    autoMoveNewsletter: document.getElementById("triage-autoMoveNewsletter").checked,
    tagPrefix:          (document.getElementById("triage-tagPrefix").value || "yuos/").trim(),
  };

  const compose   = { defaultStyle:    document.getElementById("compose-defaultStyle").value || "formal" };
  const translate = { defaultTarget:   (document.getElementById("translate-defaultTarget").value || "English").trim() };

  await YuOsUI.setSettings({ features, triage, compose, translate });

  const status = document.getElementById("save-status");
  status.textContent = browser.i18n.getMessage("saved") || "Saved";
  setTimeout(() => { status.textContent = ""; }, 1500);

  // Re-broadcast spaces visibility change.
  try { await browser.runtime.sendMessage({ kind: "refreshSpaces" }); } catch (_) {}
}

