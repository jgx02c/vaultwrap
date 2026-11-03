// Vaultwrap desktop UI. Talks to the vaultwrap daemon through the `call`
// Tauri command, which forwards protocol requests (see vaultwrap-core/src/protocol.rs).
"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ------------------------------------------------------------------ icons
// Line icons in the style of Lucide (ISC licensed), drawn at 24x24.

const ICONS = {
  lock: '<rect x="4" y="11" width="16" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/>',
  unlock: '<rect x="4" y="11" width="16" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 7.75-1.4"/>',
  settings: '<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  refresh: '<path d="M3 12a9 9 0 0 1 15.5-6.2L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-15.5 6.2L3 16"/><path d="M8 16H3v5"/>',
  copy: '<rect x="8" y="8" width="13" height="13" rx="2"/><path d="M4 16a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2"/>',
  eye: '<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>',
  eyeOff: '<path d="M9.9 9.9a3 3 0 1 0 4.2 4.2"/><path d="M10.7 5.1A10.4 10.4 0 0 1 12 5c6.5 0 10 7 10 7a13 13 0 0 1-1.7 2.7"/><path d="M6.6 6.6A13.5 13.5 0 0 0 2 12s3.5 7 10 7a9.7 9.7 0 0 0 5.4-1.6"/><path d="M2 2l20 20"/>',
  pencil: '<path d="M17 3a2.8 2.8 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5z"/>',
  trash: '<path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>',
  shield: '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>',
  building: '<rect x="4" y="2" width="16" height="20" rx="2"/><path d="M9 22v-4h6v4"/><path d="M8 6h.01M12 6h.01M16 6h.01M8 10h.01M12 10h.01M16 10h.01M8 14h.01M12 14h.01M16 14h.01"/>',
  folder: '<path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.7-.9l-.8-1.2A2 2 0 0 0 7.9 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2z"/>',
  layers: '<path d="m12 2 10 5-10 5L2 7l10-5z"/><path d="m2 17 10 5 10-5"/><path d="m2 12 10 5 10-5"/>',
  terminal: '<path d="m4 17 6-6-6-6"/><path d="M12 19h8"/>',
  key: '<circle cx="7.5" cy="15.5" r="5.5"/><path d="m21 2-9.6 9.6"/><path d="m15.5 7.5 3 3L22 7l-3-3"/>',
  cloud: '<path d="M17.5 19H9a7 7 0 1 1 6.7-9h1.8a4.5 4.5 0 1 1 0 9z"/>',
  file: '<path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><path d="M14 2v6h6"/>',
  upload: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m17 8-5-5-5 5"/><path d="M12 3v12"/>',
  x: '<path d="M18 6 6 18M6 6l12 12"/>',
  check: '<path d="M20 6 9 17l-5-5"/>',
  alert: '<path d="m21.7 18-8-14a2 2 0 0 0-3.5 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.7-3z"/><path d="M12 9v4M12 17h.01"/>',
  info: '<circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/>',
  chevron: '<path d="m9 18 6-6-6-6"/>',
  database: '<ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5v14a9 3 0 0 0 18 0V5"/><path d="M3 12a9 3 0 0 0 18 0"/>',
  search: '<circle cx="11" cy="11" r="7"/><path d="m21 21-4.3-4.3"/>',
  plug: '<path d="M12 22v-5M9 8V2M15 8V2M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8z"/>',
  play: '<path d="M6 3l14 9-14 9V3z"/>',
  link: '<path d="M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7"/><path d="M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7"/>',
  power: '<path d="M12 2v10"/><path d="M18.4 6.6a9 9 0 1 1-12.8 0"/>',
  monitor: '<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>',
  sparkle: '<path d="M12 3l1.9 5.8L20 10l-6.1 1.2L12 17l-1.9-5.8L4 10l6.1-1.2z"/>',
  fingerprint: '<path d="M12 10a2 2 0 0 0-2 2c0 1.02-.1 2.51-.26 4"/><path d="M14 13.12c0 2.38 0 6.38-1 8.88"/><path d="M17.29 21.02c.12-.6.43-2.3.5-3.02"/><path d="M2 12a10 10 0 0 1 18-6"/><path d="M2 16h.01"/><path d="M21.8 16c.2-2 .131-5.354 0-6"/><path d="M5 19.5C5.5 18 6 15 6 12a6 6 0 0 1 .34-2"/><path d="M8.65 22c.21-.66.45-1.32.57-2"/><path d="M9 6.8a6 6 0 0 1 9 5.2v2"/>',
};

const icon = (name, cls = "") => `<svg class="i ${cls}" viewBox="0 0 24 24" aria-hidden="true">${ICONS[name] || ""}</svg>`;

// ------------------------------------------------------------------ state

const S = {
  status: null,      // daemon Status
  setup: null,       // SetupInfo from Rust
  tree: [],          // OrgNode[]
  sel: null,         // {type:'env',org,project,env} | {type:'org',org} | {type:'settings'}
  tab: "vars",
  resolved: null,
  resolveError: null,
  reveal: new Set(),
  filter: "",
};

// ------------------------------------------------------------------ helpers

const $ = (sel, el = document) => el.querySelector(sel);
const h = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

async function rpc(op, args = {}) {
  const r = await invoke("call", { request: { op, ...args } });
  if (!r.ok) {
    const e = new Error(r.error || "Something went wrong");
    e.code = r.code;
    throw e;
  }
  return r.data;
}

const ignoreExists = (p) => p.then(() => true, (e) => { if (/already exists/.test(e.message)) return false; throw e; });
const targetOf = (s) => ({ org: s.org, project: s.project, env: s.env });
const tstr = (t) => `${t.org}/${t.project}/${t.env}`;
const errText = (e) => (typeof e === "string" ? e : e?.message || String(e));

let toastTimer;
function toast(msg, isError = false) {
  const t = $("#toast");
  t.innerHTML = `${icon(isError ? "alert" : "check", "sm")}<span>${h(msg)}</span>`;
  t.className = "toast show" + (isError ? " error" : "");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (t.className = "toast"), isError ? 5000 : 2200);
}

async function copy(text, what = "Copied") {
  try {
    await navigator.clipboard.writeText(text);
    toast(what);
  } catch {
    toast("Could not copy to the clipboard", true);
  }
}

const mask = (v) => (v.length ? "•".repeat(Math.min(Math.max(v.length, 6), 16)) : "(empty)");
// "macOS Keychain", or for dev/demo builds the keychain file name without its path.
const kcName = () => (S.status?.keychain || "Keychain").split(" /")[0];
const plural = (n, w) => `${n} ${w}${n === 1 ? "" : "s"}`;

// ------------------------------------------------------------------ modal

function modal({ title, body, submit = "Save", danger = false, onSubmit, wide = false, cancel = "Cancel" }) {
  return new Promise((resolve) => {
    const root = $("#modal-root");
    root.innerHTML = `
      <div class="modal-bg">
        <form class="modal" style="${wide ? "width:600px" : ""}" autocomplete="off">
          <header>${h(title)}</header>
          <div class="body">${body}<div class="err" id="modal-err"></div></div>
          <footer>
            ${cancel ? `<button type="button" class="btn" data-x="cancel">${h(cancel)}</button>` : ""}
            <button type="submit" class="btn ${danger ? "danger solid" : "primary"}">${h(submit)}</button>
          </footer>
        </form>
      </div>`;
    const form = $("form", root);
    const close = (v) => { root.innerHTML = ""; resolve(v); };
    $("[data-x=cancel]", root)?.addEventListener("click", () => close(null));
    $(".modal-bg", root).addEventListener("mousedown", (e) => { if (e.target.classList.contains("modal-bg")) close(null); });
    form.addEventListener("keydown", (e) => { if (e.key === "Escape") close(null); });
    form.onsubmit = async (e) => {
      e.preventDefault();
      const btn = $("button[type=submit]", form);
      btn.disabled = true;
      try {
        const v = onSubmit ? await onSubmit(form) : true;
        close(v === undefined ? true : v);
      } catch (err) {
        $("#modal-err", root).textContent = errText(err);
        btn.disabled = false;
      }
    };
    const first = $("input:not([type=checkbox]):not([type=hidden]):not([readonly]), textarea, select", form);
    (first || $("button[type=submit]", form)).focus();
    root._form = form;
  });
}

const confirmDialog = (title, text, submit = "Delete") =>
  modal({ title, body: `<p class="muted" style="margin:0 0 8px">${text}</p>`, submit, danger: true });

const val = (form, name) => (form.elements[name]?.value ?? "").trim();

// ------------------------------------------------------------------ boot

async function boot() {
  try {
    [S.status, S.setup] = await Promise.all([rpc("status"), invoke("setup_info")]);
  } catch (e) {
    return renderFatal(errText(e));
  }
  if (!S.status.vault_exists) return renderOnboarding();
  try {
    S.tree = await rpc("tree"); // unlocks from the keychain when allowed
    S.status = await rpc("status");
  } catch (e) {
    if (e.code === "locked") return renderLock();
    return renderFatal(e.message);
  }
  renderApp();
}

async function reloadTree() {
  S.tree = await rpc("tree");
  if (S.sel?.type === "env" && !findEnv(S.sel)) S.sel = null;
  if (S.sel?.type === "org" && !S.tree.find((o) => o.name === S.sel.org)) S.sel = null;
}

function findEnv(t) {
  const o = S.tree.find((o) => o.name === t.org);
  const p = o?.projects.find((p) => p.name === t.project);
  return p?.envs.find((e) => e.name === t.env);
}

listen("vault-changed", () => boot());
window.addEventListener("focus", async () => {
  if (!$(".app") || $("#modal-root").innerHTML) return;
  try {
    const st = await rpc("status");
    if (!st.unlocked) return boot();
    S.status = st;
    await reloadTree();
    renderApp(false);
  } catch {}
});

// ------------------------------------------------------------------ auth screens

function authShell(inner) {
  $("#root").innerHTML = `
    <div class="drag" style="position:fixed;top:0;left:0;right:0;height:38px"></div>
    <div class="auth-wrap">${inner}</div>`;
}

function renderFatal(msg) {
  authShell(`
    <div class="auth">
      <img class="logo" src="logo.png" alt="" />
      <h1>Can't reach the vault</h1>
      <p class="lead">The background service didn't respond.</p>
      <div class="banner error">${icon("alert")}<div>${h(msg)}</div></div>
      <button class="btn primary" id="retry">${icon("refresh")} Try again</button>
    </div>`);
  $("#retry").onclick = boot;
}

function setupChecklist() {
  const s = S.setup;
  if (!s) return "";
  const item = (ok, label, action) => `
    <div class="item">
      <span class="st ${ok ? "ok" : "todo"}">${icon(ok ? "check" : "alert", "sm")}</span>
      <span class="spacer">${label}</span>
      ${ok ? "" : `<button class="btn" type="button" data-setup="${action}">Install</button>`}
    </div>`;
  return `<div class="checklist">
    ${item(s.service_current || !s.in_bundle, "Runs in the background at login", "service")}
    ${item(s.cli_linked, "<code>vaultwrap</code> command in Terminal", "cli")}
  </div>`;
}

function wireSetupButtons(root, after) {
  root.querySelectorAll("[data-setup]").forEach((b) => {
    b.onclick = async () => {
      b.disabled = true;
      try {
        if (b.dataset.setup === "cli") await invoke("install_cli");
        if (b.dataset.setup === "service") await invoke("install_service");
        S.setup = await invoke("setup_info");
        toast("Installed");
      } catch (e) {
        if (errText(e) !== "cancelled") toast(errText(e), true);
      }
      after();
    };
  });
}

function renderOnboarding() {
  authShell(`
    <form class="auth" id="f">
      <img class="logo" src="logo.png" alt="" />
      <h1>Welcome to Vaultwrap</h1>
      <p class="lead">Create your encrypted vault. Its key is stored in your ${h(kcName())}, so it unlocks when you log in to this Mac.</p>
      <div class="field"><label>Master password</label><input type="password" name="pw" minlength="8" required placeholder="At least 8 characters" /></div>
      <div class="field"><label>Confirm password</label><input type="password" name="pw2" required />
        <div class="hint">Used to recover the vault on a new Mac or if the Keychain entry is removed. If you forget it, you can reset it with your Mac password.</div></div>
      <div class="err" id="err"></div>
      <button class="btn primary" type="submit">${icon("lock")} Create vault</button>
      ${setupChecklist()}
    </form>`);
  wireSetupButtons($("#root"), renderOnboarding);
  $("#f [name=pw]").focus();
  $("#f").onsubmit = async (e) => {
    e.preventDefault();
    const f = e.target;
    if (f.pw.value !== f.pw2.value) return ($("#err").textContent = "Passwords don't match");
    const btn = $("button[type=submit]", f);
    btn.disabled = true;
    try {
      const r = await rpc("init", { password: f.pw.value });
      if (r.warning) toast(r.warning, true);
      else toast("Vault created");
      boot();
    } catch (err) {
      $("#err").textContent = err.message;
      btn.disabled = false;
    }
  };
}

function renderLock() {
  const st = S.status;
  const keyAvailable = st.key_in_keychain;
  authShell(`
    <form class="auth" id="f">
      <img class="logo" src="logo.png" alt="" />
      <h1>Vault locked</h1>
      <p class="lead">${keyAvailable
        ? "Unlock with your Mac password or your master password."
        : `The vault key isn't in your ${h(kcName())}. Enter your master password to unlock. The key will be saved there again.`}</p>
      ${keyAvailable ? `
        <button class="btn primary" type="button" id="mac">${icon("fingerprint")} Unlock with Mac password</button>
        <div class="divider">or</div>` : ""}
      <div class="field"><input type="password" name="pw" required placeholder="Master password" /></div>
      <div class="err" id="err"></div>
      <button class="btn ${keyAvailable ? "wide" : "primary"}" type="submit">${icon("unlock")} Unlock with master password</button>
      <div class="foot"><a id="forgot">Forgot your master password?</a></div>
    </form>`);
  if (!keyAvailable) $("#f [name=pw]").focus();
  $("#mac")?.addEventListener("click", async () => {
    try {
      await invoke("confirm_mac_password", { reason: "Vaultwrap wants to unlock your vault." });
      await rpc("unlock_keychain");
      boot();
    } catch (e) {
      if (errText(e) !== "cancelled") $("#err").textContent = errText(e);
    }
  });
  $("#forgot").onclick = forgotPassword;
  $("#f").onsubmit = async (e) => {
    e.preventDefault();
    const f = e.target;
    const btn = $("button[type=submit]", f);
    btn.disabled = true;
    try {
      const r = await rpc("unlock", { password: f.pw.value });
      if (r.warning) toast(r.warning, true);
      boot();
    } catch (err) {
      $("#err").textContent = err.message;
      btn.disabled = false;
      f.pw.select();
    }
  };
}

async function forgotPassword() {
  const st = S.status;
  if (st.key_in_keychain) {
    const ok = await modal({
      title: "Reset your master password",
      body: `<p class="muted" style="margin-top:0">Your vault key is still in your ${h(kcName())}, so your Mac password is enough.
        Confirm it, then choose a new master password.</p>
        <div class="field"><label>New master password</label><input type="password" name="pw" minlength="8" required /></div>
        <div class="field"><label>Confirm</label><input type="password" name="pw2" required /></div>`,
      submit: "Confirm with Mac password",
      onSubmit: async (f) => {
        if (f.pw.value !== f.pw2.value) throw new Error("Passwords don't match");
        await invoke("confirm_mac_password", { reason: "Vaultwrap wants to reset your vault's master password." });
        await rpc("unlock_keychain");
        await rpc("set_password", { new_password: f.pw.value });
      },
    });
    if (ok) {
      toast("Master password reset");
      boot();
    }
    return;
  }
  await eraseVault(
    `Without the master password or the ${h(kcName())} entry, this vault can't be decrypted. ` +
    `You can start over with a new, empty vault. The old one is moved to a backup folder, not deleted.`,
  );
}

async function eraseVault(explain) {
  const r = await modal({
    title: "Erase vault and start over",
    body: `<div class="banner error">${icon("alert")}<div>${explain}</div></div>
      <div class="field"><label>Type ERASE to confirm</label><input type="text" name="confirm" required autocomplete="off" /></div>`,
    submit: "Erase vault",
    danger: true,
    onSubmit: async (f) => {
      if (val(f, "confirm") !== "ERASE") throw new Error("Type ERASE to confirm");
      return rpc("erase_vault");
    },
  });
  if (r) {
    toast("Vault erased. Backup kept.");
    S.sel = null;
    localStorage.removeItem("sel");
    boot();
  }
}

// ------------------------------------------------------------------ main app

function renderApp(refetch = true) {
  if (!$(".app")) {
    $("#root").innerHTML = `
      <div class="app">
        <aside class="sidebar">
          <div class="side-top drag"></div>
          <div class="brand">
            <img src="logo.png" alt="" /><span class="name">Vaultwrap</span>
            <span class="spacer"></span>${S.setup?.demo ? '<span class="badge accent">Demo</span>' : ""}
          </div>
          <div class="side-label"><span>Environments</span>
            <button class="icon-btn" title="New environment" data-a="new-env">${icon("plus")}</button></div>
          <div class="tree" id="tree"></div>
          <div class="side-bottom">
            <button class="nav-btn" data-a="new-org">${icon("building", "lg")} New organization</button>
            <button class="nav-btn" id="nav-settings" data-a="settings">${icon("settings", "lg")} Settings</button>
          </div>
        </aside>
        <main class="content">
          <div class="topbar drag">
            <div class="crumbs" id="crumbs"></div>
            <span class="spacer"></span>
            <span class="status-pill" title="Vault key in ${h(kcName())}"><span class="dot"></span>Unlocked</span>
            <button class="btn" data-a="lock" title="Lock the vault">${icon("lock")} Lock</button>
          </div>
          <div class="view" id="view"></div>
        </main>
      </div>`;
    $(".app").addEventListener("click", onClick);
  }
  if (S.sel) localStorage.setItem("sel", JSON.stringify(S.sel));
  if (!S.sel) {
    try {
      const saved = JSON.parse(localStorage.getItem("sel") || "null");
      if (saved?.type === "env" && findEnv(saved)) S.sel = saved;
      if (saved?.type === "org" && S.tree.some((o) => o.name === saved.org)) S.sel = saved;
    } catch {}
  }
  if (!S.sel) {
    const o = S.tree.find((o) => o.projects.some((p) => p.envs.length));
    const p = o?.projects.find((p) => p.envs.length);
    if (p) S.sel = { type: "env", org: o.name, project: p.name, env: p.envs[0].name };
  }
  renderTree();
  renderCrumbs();
  $("#nav-settings").classList.toggle("sel", S.sel?.type === "settings");
  renderView(refetch);
}

function renderCrumbs() {
  const s = S.sel;
  const sep = icon("chevron", "sm");
  let html = "";
  if (!s) html = "<b>Home</b>";
  else if (s.type === "settings") html = "<b>Settings</b>";
  else if (s.type === "org") html = `${icon("building", "sm")}<b>${h(s.org)}</b>`;
  else html = `${icon("building", "sm")}<a data-a="org" data-org="${h(s.org)}">${h(s.org)}</a>${sep}<span>${h(s.project)}</span>${sep}<b>${h(s.env)}</b>`;
  $("#crumbs").innerHTML = html;
}

function renderTree() {
  const sel = S.sel || {};
  $("#tree").innerHTML = S.tree.length
    ? S.tree.map((o) => `
      <div class="org">
        <button class="org-row ${sel.type === "org" && sel.org === o.name ? "sel" : ""}" data-a="org" data-org="${h(o.name)}">
          ${icon("building")}<span class="spacer">${h(o.name)}</span>
        </button>
        ${o.projects.map((p) => `
          <div class="proj">${icon("folder", "sm")}${h(p.name)}</div>
          <div class="envs">
          ${p.envs.map((e) => `
            <button class="env ${sel.type === "env" && sel.org === o.name && sel.project === p.name && sel.env === e.name ? "sel" : ""}"
              data-a="env" data-org="${h(o.name)}" data-project="${h(p.name)}" data-env="${h(e.name)}">
              <span class="spacer">${h(e.name)}</span>${e.protected ? `<span class="shield" title="Protected">${icon("shield", "sm")}</span>` : ""}
            </button>`).join("")}
          </div>`).join("")}
      </div>`).join("")
    : `<div class="empty">No environments yet.</div>`;
}

async function renderView(refetch) {
  const view = $("#view");
  const sel = S.sel;
  if (!sel) return renderWelcome(view);
  if (sel.type === "settings") return renderSettings(view);
  if (sel.type === "org") return renderOrg(view);
  if (refetch) {
    S.resolved = null;
    S.resolveError = null;
    renderEnv(view);
    try {
      S.resolved = await rpc("resolve", { target: targetOf(sel), refresh: false });
    } catch (e) {
      S.resolveError = e.message;
    }
    if (S.sel !== sel) return;
  }
  renderEnv(view);
}

function renderWelcome(view) {
  view.innerHTML = `
    <div class="empty-state">
      <div class="ico">${icon("layers", "lg")}</div>
      <h1>Create your first environment</h1>
      <p>An environment like <code>chally/api/staging</code> is an organization, a project and a stage.</p>
      <button class="btn primary" data-a="new-env">${icon("plus")} New environment</button>
    </div>`;
}

// ---------------------------------------------------------------- env view

function renderEnv(view) {
  const sel = S.sel;
  const node = findEnv(sel);
  if (!node) return renderWelcome(view);
  const n = S.resolved?.vars.length;
  const tabs = [
    ["vars", "Variables", "key", n ?? ""],
    ["sources", "Sources", "layers", node.sources.length + 1],
    ["use", "Use in terminal", "terminal", ""],
  ];
  view.innerHTML = `
    <div class="page-head">
      <div>
        <div class="eyebrow">${h(sel.org)} / ${h(sel.project)}</div>
        <div class="row"><h1>${h(sel.env)}</h1>${node.protected ? `<span class="badge warn">${icon("shield", "sm")} Protected</span>` : ""}</div>
      </div>
      <span class="spacer"></span>
      <div class="btn-group">
        <button class="btn" data-a="refresh" title="Refetch remote sources">${icon("refresh")} Refresh</button>
        <button class="btn" data-a="toggle-protect">${icon("shield")} ${node.protected ? "Unprotect" : "Protect"}</button>
        <button class="btn" data-a="clone-env">${icon("copy")} Duplicate</button>
        <button class="btn danger" data-a="delete-env" title="Delete environment">${icon("trash")}</button>
      </div>
    </div>
    <div class="tabs">${tabs.map(([k, l, ic, c]) => `<button class="tab ${S.tab === k ? "on" : ""}" data-a="tab" data-tab="${k}">${icon(ic, "sm")} ${l} ${c !== "" ? `<span class="count">${c}</span>` : ""}</button>`).join("")}</div>
    <div id="tab"></div>`;
  const tab = $("#tab");
  if (S.tab === "vars") renderVars(tab, node);
  if (S.tab === "sources") renderSources(tab, node);
  if (S.tab === "use") renderUse(tab);
}

function renderVars(el, node) {
  const r = S.resolved;
  let html = "";
  if (S.resolveError) html += `<div class="banner error">${icon("alert")}<div>${h(S.resolveError)}</div></div>`;
  for (const w of r?.warnings || []) html += `<div class="banner warn">${icon("alert")}<div>${h(w)}</div></div>`;
  html += `
    <div class="row" style="margin-bottom:12px">
      <div class="search">${icon("search", "sm")}<input type="search" id="filter" placeholder="Filter variables" value="${h(S.filter)}" /></div>
      <span class="spacer"></span>
      <button class="btn" data-a="import">${icon("upload")} Import .env</button>
      <button class="btn primary" data-a="add-var">${icon("plus")} Add variable</button>
    </div>`;
  if (!r && !S.resolveError) {
    html += `<div class="center" style="height:140px"><div class="spinner"></div></div>`;
  } else {
    const vars = r ? r.vars : node.local_keys.map((k) => ({ key: k, value: null, source: "local" }));
    const f = S.filter.toLowerCase();
    const shown = vars.filter((v) => !f || v.key.toLowerCase().includes(f)).sort((a, b) => a.key.localeCompare(b.key));
    if (!vars.length) {
      html += `<div class="card"><div class="empty-state">
        <div class="ico">${icon("key", "lg")}</div>
        <h1>No variables yet</h1>
        <p>Add one, import a <code>.env</code> file, or attach a source like Azure Key Vault.</p></div></div>`;
    } else {
      const localCount = vars.filter((v) => v.source === "local").length;
      html += `<div class="card"><table><thead><tr><th>Name</th><th>Value</th><th>Source</th><th></th></tr></thead><tbody>
        ${shown.map((v) => {
          const revealed = S.reveal.has(v.key);
          const value = v.value === null ? '<span class="faint">unavailable</span>'
            : revealed ? h(v.value) : `<span class="masked">${mask(v.value)}</span>`;
          const local = v.source === "local";
          return `<tr>
            <td class="key">${h(v.key)}</td>
            <td class="val selectable" title="${revealed ? h(v.value) : ""}">${value}</td>
            <td>${local ? `<span class="badge">${icon("database", "sm")} local</span>` : `<span class="badge accent">${icon("cloud", "sm")} ${h(v.source)}</span>`}</td>
            <td class="actions">
              <button class="icon-btn ${v.value === null ? "hidden" : ""}" title="${revealed ? "Hide" : "Reveal"}" data-a="reveal" data-key="${h(v.key)}">${icon(revealed ? "eyeOff" : "eye")}</button>
              <button class="icon-btn ${v.value === null ? "hidden" : ""}" title="Copy value" data-a="copy-val" data-key="${h(v.key)}">${icon("copy")}</button>
              <button class="icon-btn" title="${local ? "Edit" : "Override locally"}" data-a="edit-var" data-key="${h(v.key)}">${icon("pencil")}</button>
              <button class="icon-btn danger ${local ? "" : "hidden"}" title="Delete" data-a="delete-var" data-key="${h(v.key)}">${icon("trash")}</button>
            </td></tr>`;
        }).join("")}
      </tbody></table>
      <div class="table-foot"><span>${plural(vars.length, "variable")} · ${localCount} local, ${vars.length - localCount} from sources</span><span>Local values override sources</span></div></div>`;
    }
  }
  el.innerHTML = html;
  const fi = $("#filter", el);
  fi.oninput = () => {
    S.filter = fi.value;
    const pos = fi.selectionStart;
    renderVars(el, node);
    const n = $("#filter", el);
    n.focus();
    n.setSelectionRange(pos, pos);
  };
}

const KINDS = {
  "azure-keyvault": { label: "Azure Key Vault", icon: "cloud" },
  dotenv: { label: ".env file", icon: "file" },
  command: { label: "Command", icon: "terminal" },
};
const kindLabel = (k) => KINDS[k]?.label || k;
const kindIcon = (k) => KINDS[k]?.icon || "plug";

function renderSources(el, node) {
  const org = S.tree.find((o) => o.name === S.sel.org);
  const opts = (o) => Object.entries(o || {}).map(([k, v]) => `${h(k)}=${h(v)}`).join("  ");
  el.innerHTML = `
    <p class="muted" style="margin-top:0">Sources merge top to bottom: later ones override earlier ones, and local values always win.</p>
    <div class="card">
      ${node.sources.map((s, i) => `
        <div class="layer">
          <span class="num">${i + 1}</span>
          <span class="provider-icon">${icon(kindIcon(s.kind))}</span>
          <div class="spacer"><div class="row"><b>${h(s.provider)}</b><span class="badge">${h(kindLabel(s.kind))}</span></div>
            <div class="mono faint">${opts(s.options) || "no options"}</div></div>
          <button class="btn danger" data-a="remove-source" data-provider="${h(s.provider)}">Detach</button>
        </div>`).join("")}
      <div class="layer local">
        <span class="num">${node.sources.length + 1}</span>
        <span class="provider-icon" style="background:var(--bg-sunk);color:var(--fg2);border-color:var(--line)">${icon("database")}</span>
        <div class="spacer"><b>Local values</b><div class="faint small">${plural(node.local_keys.length, "variable")} stored in this vault</div></div>
      </div>
    </div>
    ${org.providers.length
      ? `<button class="btn primary" data-a="add-source">${icon("plus")} Attach source</button>`
      : `<div class="banner info">${icon("info")}<div>${h(org.name)} has no providers yet. <a data-a="add-provider" data-org="${h(org.name)}">Connect Azure Key Vault, a .env file or a CLI</a>.</div></div>`}`;
}

function renderUse(el) {
  const t = tstr(S.sel);
  const shell = S.setup?.shell || "zsh";
  const hook = shell === "fish" ? "vaultwrap hook fish | source" : `eval "$(vaultwrap hook ${shell})"`;
  const rc = { zsh: "~/.zshrc", bash: "~/.bashrc", fish: "~/.config/fish/config.fish" }[shell] || "your shell profile";
  const snip = (cmd) => `<div class="snippet"><code>${h(cmd)}</code><button class="btn" data-a="copy" data-text="${h(cmd)}">${icon("copy", "sm")} Copy</button></div>`;
  const step = (ic, title, desc, cmd) => `<div class="use-step"><div class="ico">${icon(ic)}</div><div><h2>${title}</h2><p>${desc}</p>${snip(cmd)}</div></div>`;
  el.innerHTML = `
    ${S.setup && !S.setup.cli_linked ? `<div class="banner warn">${icon("alert")}<div>The <code>vaultwrap</code> command isn't installed yet. <a data-a="settings">Install it in Settings</a>.</div></div>` : ""}
    <div class="card"><div class="card-body" style="padding-top:0;padding-bottom:0">
      ${step("link", "Link a repository", "Run it inside the repo. It writes <code>.vaultwrap.toml</code> (safe to commit), so commands there use this environment.", `vaultwrap link ${S.sel.org}/${S.sel.project} -e ${S.sel.env}`)}
      ${step("play", "Run a command with these secrets", "In a linked repo, <code>vaultwrap run -- npm run dev</code> is enough.", `vaultwrap run -e ${t} -- npm run dev`)}
      ${step("terminal", "Load into the current shell", "Undo with <code>vaultwrap_deactivate</code>.", `eval "$(vaultwrap activate -e ${t})"`)}
      ${step("refresh", "Load automatically on cd", `Add this line to ${h(rc)}. Protected environments are never auto-loaded.`, hook)}
      ${step("file", "Write a .env file", "For tools that only read files. Don't commit the result.", `vaultwrap export -e ${t} > .env.local`)}
    </div></div>`;
}

// ---------------------------------------------------------------- org view

function renderOrg(view) {
  const org = S.tree.find((o) => o.name === S.sel.org);
  if (!org) return renderWelcome(view);
  const kv = (cfg) => Object.entries(cfg).map(([k, v]) => `<div>${h(k)}</div><div class="selectable">${h(typeof v === "string" ? v : JSON.stringify(v))}</div>`).join("");
  view.innerHTML = `
    <div class="page-head">
      <div><div class="eyebrow">Organization</div><h1>${h(org.name)}</h1></div>
      <span class="spacer"></span>
      <button class="btn danger" data-a="delete-org">${icon("trash")} Delete organization</button>
    </div>

    <div class="section">
      <div class="section-head"><h2>Providers</h2><span class="muted">Where this organization's secrets live</span><span class="spacer"></span>
        <button class="btn primary" data-a="add-provider" data-org="${h(org.name)}">${icon("plus")} Add provider</button></div>
      ${org.providers.map((p) => `
        <div class="card accent">
          <div class="card-head">
            <span class="provider-icon">${icon(kindIcon(p.kind))}</span>
            <div class="spacer"><div class="title">${h(p.name)}</div><div class="faint small">${h(kindLabel(p.kind))}</div></div>
            <button class="btn" data-a="test-provider" data-name="${h(p.name)}">${icon("plug")} Test</button>
            <button class="btn danger" data-a="delete-provider" data-name="${h(p.name)}" title="Delete provider">${icon("trash")}</button>
          </div>
          <div class="card-body"><div class="kv">${kv(p.config)}</div></div>
        </div>`).join("") || `<p class="muted">None yet. Secrets can also live directly in the vault.</p>`}
    </div>

    <div class="section">
      <div class="section-head"><h2>Projects</h2><span class="spacer"></span>
        <button class="btn" data-a="new-env" data-org="${h(org.name)}">${icon("plus")} New project</button></div>
      ${org.projects.length ? `<div class="card">${org.projects.map((p) => `
        <div class="layer">
          <span class="provider-icon" style="background:var(--bg-sunk);color:var(--fg2);border-color:var(--line)">${icon("folder")}</span>
          <div class="spacer"><b>${h(p.name)}</b>
            <div class="row" style="margin-top:4px;flex-wrap:wrap">${p.envs.map((e) => `<a class="badge ${e.protected ? "warn" : ""}" data-a="env" data-org="${h(org.name)}" data-project="${h(p.name)}" data-env="${h(e.name)}">${e.protected ? icon("shield", "sm") : ""}${h(e.name)}</a>`).join("") || '<span class="faint small">no environments</span>'}</div>
          </div>
          <button class="btn" data-a="new-env" data-org="${h(org.name)}" data-project="${h(p.name)}">${icon("plus")} Environment</button>
          <button class="btn danger" data-a="delete-project" data-project="${h(p.name)}" title="Delete project">${icon("trash")}</button>
        </div>`).join("")}</div>` : `<p class="muted">No projects yet.</p>`}
    </div>`;
}

// ---------------------------------------------------------------- settings

function renderSettings(view) {
  const st = S.status;
  const s = S.setup;
  const toggle = (id, on) => `<label class="toggle"><input type="checkbox" id="${id}" ${on ? "checked" : ""}/><span></span></label>`;
  const shortId = (st.vault_id || "").slice(0, 8);
  view.innerHTML = `
    <div class="page-head"><div><div class="eyebrow">Preferences</div><h1>Settings</h1></div></div>

    <div class="section">
      <div class="section-head"><h2>Security</h2></div>
      <div class="card">
        <div class="setting">
          <span class="provider-icon">${icon("key")}</span>
          <div class="label"><div class="title">Vault key in ${h(kcName())}</div>
            <div class="muted">${st.key_in_keychain
              ? `Unlocks automatically when you log in. Stored as <code>dev.vaultwrap</code> / <code>vault-key:${h(shortId)}…</code>`
              : "Not stored. The next unlock asks for your master password, then saves it again."}</div></div>
          ${st.key_in_keychain
            ? `<span class="badge ok">${icon("check", "sm")} Stored</span><button class="btn" data-a="forget-keychain">Remove</button>`
            : `<span class="badge warn">Not stored</span>`}
        </div>
        <div class="setting">
          <span class="provider-icon">${icon("lock")}</span>
          <div class="label"><div class="title">Master password</div>
            <div class="muted">Recovers the vault on a new Mac or if the Keychain entry is removed.</div></div>
          <button class="btn" data-a="passwd">Change</button>
          <button class="btn" data-a="reset-passwd">${icon("fingerprint")} Reset with Mac password</button>
        </div>
      </div>
    </div>

    <div class="section">
      <div class="section-head"><h2>System</h2></div>
      <div class="card">
        <div class="setting">
          <span class="provider-icon">${icon("terminal")}</span>
          <div class="label"><div class="title">Command line tool</div><div class="muted">${s.cli_linked ? "<code>vaultwrap</code> is installed at /usr/local/bin" : s.cli_other ? `/usr/local/bin/vaultwrap points to another copy: ${h(s.cli_other)}` : "Adds <code>vaultwrap</code> to your Terminal (asks for your password)."}</div></div>
          ${s.cli_linked ? `<span class="badge ok">${icon("check", "sm")} Installed</span>` : `<button class="btn primary" data-setup="cli">${s.cli_other ? "Replace" : "Install"}</button>`}
        </div>
        <div class="setting">
          <span class="provider-icon">${icon("power")}</span>
          <div class="label"><div class="title">Background service</div><div class="muted">Keeps the vault running from login, so the terminal and shell hooks work without this window open.</div></div>
          ${s.service_current ? `<span class="badge ok">${icon("check", "sm")} Running at login</span>` : `<button class="btn primary" data-setup="service">${s.service_installed ? "Repair" : "Install"}</button>`}
        </div>
        <div class="setting">
          <span class="provider-icon">${icon("monitor")}</span>
          <div class="label"><div class="title">Show in menu bar at login</div><div class="muted">Quick access to lock and open the vault.</div></div>
          ${toggle("login", s.open_at_login)}
        </div>
      </div>
    </div>

    <div class="section">
      <div class="section-head"><h2>About</h2></div>
      <div class="card"><div class="card-body"><div class="kv">
        <div>Version</div><div>${h(s.version)}</div>
        <div>Daemon</div><div>pid ${h(st.pid)} · v${h(st.version)}</div>
        <div>Vault</div><div class="selectable">${h(st.home)}/vault.db</div>
        <div>CLI</div><div class="selectable">${h(s.cli_path || "not found")}</div>
      </div></div></div>
    </div>

    <div class="section">
      <div class="card danger-zone">
        <div class="card-head">${icon("alert")}<span class="title">Danger zone</span></div>
        <div class="setting">
          <div class="label"><div class="title">Erase vault</div><div class="muted">Moves the vault to a backup folder and starts over with an empty one.</div></div>
          <button class="btn danger" data-a="erase">Erase vault</button>
        </div>
      </div>
    </div>`;
  wireSetupButtons(view, () => renderSettings(view));
  $("#login", view).onchange = async (e) => {
    try {
      await invoke("set_open_at_login", { enabled: e.target.checked });
      S.setup = await invoke("setup_info");
    } catch (err) {
      toast(errText(err), true);
    }
  };
}

// ------------------------------------------------------------------ actions

async function onClick(e) {
  const el = e.target.closest("[data-a]");
  if (!el) return;
  try {
    await ACTIONS[el.dataset.a]?.(el.dataset, el);
  } catch (err) {
    toast(errText(err), true);
  }
}

const ACTIONS = {
  env(d) {
    S.sel = { type: "env", org: d.org, project: d.project, env: d.env };
    S.reveal.clear();
    S.filter = "";
    renderApp();
  },
  org(d) {
    S.sel = { type: "org", org: d.org };
    renderApp();
  },
  async settings() {
    S.sel = { type: "settings" };
    [S.status, S.setup] = await Promise.all([rpc("status"), invoke("setup_info")]);
    renderApp();
  },
  async lock() {
    await rpc("lock");
    toast("Vault locked");
    boot();
  },
  tab(d) {
    S.tab = d.tab;
    renderEnv($("#view"));
  },
  async refresh() {
    S.resolved = null;
    renderEnv($("#view"));
    try {
      S.resolved = await rpc("resolve", { target: targetOf(S.sel), refresh: true });
      S.resolveError = null;
      toast("Refreshed");
    } catch (e) {
      S.resolveError = e.message;
    }
    renderEnv($("#view"));
  },
  reveal(d) {
    S.reveal.has(d.key) ? S.reveal.delete(d.key) : S.reveal.add(d.key);
    renderEnv($("#view"));
  },
  "copy-val"(d) {
    const v = S.resolved?.vars.find((v) => v.key === d.key);
    if (v) copy(v.value, `Copied ${d.key}`);
  },
  copy(d) {
    copy(d.text);
  },

  async "new-org"() {
    const name = await modal({
      title: "New organization",
      body: `<div class="field"><label>Name</label><input type="text" name="name" required placeholder="chally" />
             <div class="hint">A client or company. Each organization has its own providers, such as its own Azure Key Vault.</div></div>`,
      submit: "Create",
      onSubmit: async (f) => {
        await rpc("org_add", { name: val(f, "name") });
        return val(f, "name");
      },
    });
    if (name) {
      await reloadTree();
      S.sel = { type: "org", org: name };
      renderApp();
    }
  },

  async "new-env"(d) {
    const orgs = S.tree.map((o) => o.name);
    const projects = [...new Set(S.tree.flatMap((o) => o.projects.map((p) => p.name)))];
    const envs = S.tree.flatMap((o) => o.projects.flatMap((p) => p.envs.map((e) => `${o.name}/${p.name}/${e.name}`)));
    const t = await modal({
      title: "New environment",
      body: `
        <div class="field"><label>Organization</label><input type="text" name="org" list="dl-org" required value="${h(d.org || "")}" placeholder="chally" /></div>
        <div class="field"><label>Project</label><input type="text" name="project" list="dl-project" required value="${h(d.project || "")}" placeholder="api" /></div>
        <div class="field"><label>Environment</label><input type="text" name="env" list="dl-env" required placeholder="staging" /></div>
        <datalist id="dl-org">${orgs.map((o) => `<option value="${h(o)}">`).join("")}</datalist>
        <datalist id="dl-project">${projects.map((o) => `<option value="${h(o)}">`).join("")}</datalist>
        <datalist id="dl-env"><option value="dev"><option value="staging"><option value="prod"></datalist>
        <div class="field"><label>Copy from</label><select name="from"><option value="">Start empty</option>${envs.map((e) => `<option>${h(e)}</option>`).join("")}</select>
          <div class="hint">Copies sources and local values. Must be in the same organization.</div></div>
        <label class="check"><input type="checkbox" name="protect" /> <span><b>Protected</b><br><span class="muted small">For production: never auto-loaded into shells, and the CLI asks before running.</span></span></label>`,
      submit: "Create",
      onSubmit: async (f) => {
        const t = { org: val(f, "org"), project: val(f, "project"), env: val(f, "env") };
        const from = val(f, "from");
        if (from) {
          const [org, project, env] = from.split("/");
          await rpc("env_clone", { from: { org, project, env }, to: t });
          if (f.protect.checked) await rpc("env_protect", { target: t, protected: true });
        } else {
          await ignoreExists(rpc("org_add", { name: t.org }));
          await ignoreExists(rpc("project_add", { org: t.org, name: t.project }));
          await rpc("env_add", { target: t, protected: f.protect.checked });
        }
        return t;
      },
    });
    if (t) {
      await reloadTree();
      S.sel = { type: "env", ...t };
      S.tab = "vars";
      renderApp();
      toast(`Created ${tstr(t)}`);
    }
  },

  async "toggle-protect"() {
    const node = findEnv(S.sel);
    await rpc("env_protect", { target: targetOf(S.sel), protected: !node.protected });
    await reloadTree();
    renderApp(false);
  },
  async "clone-env"() {
    const from = targetOf(S.sel);
    const t = await modal({
      title: `Duplicate ${from.env}`,
      body: `<div class="field"><label>New environment name</label><input type="text" name="env" required placeholder="staging-2" /></div>
             <div class="field"><label>Project</label><input type="text" name="project" required value="${h(from.project)}" /></div>`,
      submit: "Duplicate",
      onSubmit: async (f) => {
        const to = { org: from.org, project: val(f, "project"), env: val(f, "env") };
        await rpc("env_clone", { from, to });
        return to;
      },
    });
    if (t) {
      await reloadTree();
      S.sel = { type: "env", ...t };
      renderApp();
    }
  },
  async "delete-env"() {
    if (!(await confirmDialog("Delete environment?", `<b>${h(tstr(S.sel))}</b> and its local values will be permanently deleted. Remote providers are not touched.`))) return;
    await rpc("env_remove", { target: targetOf(S.sel) });
    S.sel = null;
    await reloadTree();
    renderApp();
  },
  async "delete-project"(d) {
    const org = S.sel.org;
    if (!(await confirmDialog("Delete project?", `<b>${h(org)}/${h(d.project)}</b> and all its environments will be permanently deleted.`))) return;
    await rpc("project_remove", { org, name: d.project });
    await reloadTree();
    renderApp();
  },
  async "delete-org"() {
    const org = S.sel.org;
    if (!(await confirmDialog("Delete organization?", `<b>${h(org)}</b> and its providers, projects and environments will be permanently deleted.`))) return;
    await rpc("org_remove", { name: org });
    S.sel = null;
    await reloadTree();
    renderApp();
  },

  async "add-var"() {
    await editVar("", "");
  },
  async "edit-var"(d) {
    const v = S.resolved?.vars.find((v) => v.key === d.key);
    await editVar(d.key, v?.source === "local" ? v.value : "", v && v.source !== "local");
  },
  async "delete-var"(d) {
    if (!(await confirmDialog("Delete variable?", `Remove <b>${h(d.key)}</b> from ${h(tstr(S.sel))}?`))) return;
    await rpc("secret_unset", { target: targetOf(S.sel), keys: [d.key] });
    await afterVarChange();
  },
  async import() {
    const n = await modal({
      title: "Import .env",
      wide: true,
      body: `
        <div class="field"><label>Choose a file</label><input type="file" id="envfile" /></div>
        <div class="field"><label>Or paste its contents</label><textarea name="text" rows="10" placeholder="DATABASE_URL=postgres://…&#10;API_KEY=…"></textarea>
          <div class="hint">Stored as local values in ${h(tstr(S.sel))}. Existing names are overwritten.</div></div>`,
      submit: "Import",
      onSubmit: async (f) => {
        const vars = await invoke("parse_dotenv", { text: f.text.value });
        if (!vars.length) throw new Error("No variables found");
        await rpc("secret_set", { target: targetOf(S.sel), vars });
        return vars.length;
      },
    });
    if (n) {
      toast(`Imported ${plural(n, "variable")}`);
      await afterVarChange();
    }
  },

  async "add-source"() {
    const org = S.tree.find((o) => o.name === S.sel.org);
    const ok = await modal({
      title: "Attach source",
      body: `
        <div class="field"><label>Provider</label><select name="provider">${org.providers.map((p) => `<option value="${h(p.name)}">${h(p.name)} (${h(kindLabel(p.kind))})</option>`).join("")}</select></div>
        <div class="field"><label>Name prefix (Azure Key Vault)</label><input type="text" name="prefix" placeholder="{env}-" />
          <div class="hint">Only secrets starting with this, prefix removed: <code>staging-database-url</code> becomes <code>DATABASE_URL</code>. Supports {org} {project} {env}.</div></div>
        <div class="field"><label>Option overrides</label><textarea name="opts" rows="3" placeholder="path=~/clients/acme/.env.{env}&#10;vault_url=https://other.vault.azure.net"></textarea>
          <div class="hint">One key=value per line. Overrides the provider's settings for this environment only.</div></div>`,
      submit: "Attach",
      onSubmit: async (f) => {
        const options = {};
        if (val(f, "prefix")) options.prefix = val(f, "prefix");
        for (const line of f.opts.value.split("\n")) {
          const i = line.indexOf("=");
          if (i > 0) options[line.slice(0, i).trim()] = line.slice(i + 1).trim();
        }
        await rpc("source_add", { target: targetOf(S.sel), provider: f.provider.value, options });
      },
    });
    if (ok) {
      await reloadTree();
      renderApp();
      toast("Source attached");
    }
  },
  async "remove-source"(d) {
    if (!(await confirmDialog("Detach source?", `Stop using <b>${h(d.provider)}</b> in ${h(tstr(S.sel))}? The provider itself stays.`, "Detach"))) return;
    await rpc("source_remove", { target: targetOf(S.sel), provider: d.provider });
    await reloadTree();
    renderApp();
  },

  async "add-provider"(d) {
    await addProvider(d.org || S.sel.org);
  },
  async "test-provider"(d) {
    toast(`Testing ${d.name}…`);
    try {
      const r = await rpc("provider_test", { org: S.sel.org, name: d.name, options: {} });
      await modal({
        title: `${d.name}: ${plural(r.keys.length, "variable")}`,
        body: r.keys.length
          ? `<div class="card"><table><tbody>${r.keys.map((k) => `<tr><td class="key">${h(k)}</td></tr>`).join("")}</tbody></table></div>`
          : `<p class="muted">Connected, but no variables were returned.</p>`,
        submit: "Done",
        cancel: null,
      });
    } catch (e) {
      await modal({ title: `${d.name} failed`, body: `<div class="banner error">${icon("alert")}<div>${h(e.message)}</div></div>`, submit: "OK", cancel: null });
    }
  },
  async "delete-provider"(d) {
    if (!(await confirmDialog("Delete provider?", `<b>${h(d.name)}</b> will be removed and detached from every environment that uses it.`))) return;
    await rpc("provider_remove", { org: S.sel.org, name: d.name });
    await reloadTree();
    renderApp();
  },

  async passwd() {
    const ok = await modal({
      title: "Change master password",
      body: `
        <div class="field"><label>Current password</label><input type="password" name="cur" required /></div>
        <div class="field"><label>New password</label><input type="password" name="pw" minlength="8" required /></div>
        <div class="field"><label>Confirm new password</label><input type="password" name="pw2" required /></div>`,
      submit: "Change password",
      onSubmit: async (f) => {
        if (f.pw.value !== f.pw2.value) throw new Error("New passwords don't match");
        await rpc("verify_password", { password: f.cur.value });
        await rpc("set_password", { new_password: f.pw.value });
      },
    });
    if (ok) toast("Master password changed");
  },
  async "reset-passwd"() {
    const ok = await modal({
      title: "Reset master password",
      body: `<p class="muted" style="margin-top:0">Forgot it? Confirm your Mac password instead of the old master password.</p>
        <div class="field"><label>New master password</label><input type="password" name="pw" minlength="8" required /></div>
        <div class="field"><label>Confirm</label><input type="password" name="pw2" required /></div>`,
      submit: "Confirm with Mac password",
      onSubmit: async (f) => {
        if (f.pw.value !== f.pw2.value) throw new Error("Passwords don't match");
        await invoke("confirm_mac_password", { reason: "Vaultwrap wants to reset your vault's master password." });
        await rpc("set_password", { new_password: f.pw.value });
      },
    });
    if (ok) toast("Master password reset");
  },
  async "forget-keychain"() {
    const ok = await confirmDialog(
      "Remove the vault key from the Keychain?",
      "The vault stays unlocked for now. The next unlock (e.g. after a restart) needs your master password, which then saves the key again.",
      "Remove",
    );
    if (!ok) return;
    await rpc("forget");
    S.status = await rpc("status");
    renderSettings($("#view"));
    toast("Removed from Keychain");
  },
  async erase() {
    await eraseVault("Everything in this vault will be moved to a backup folder and a new, empty vault can be created. Remote providers are not touched.");
  },
};

async function afterVarChange() {
  await reloadTree();
  renderApp(true);
}

async function editVar(key, value, overriding = false) {
  const ok = await modal({
    title: key ? (overriding ? `Override ${key}` : `Edit ${key}`) : "Add variable",
    body: `
      ${overriding ? `<div class="banner info">${icon("info")}<div>This value comes from a source. Saving stores a local value that overrides it in this environment.</div></div>` : ""}
      <div class="field"><label>Name</label><input type="text" name="key" required value="${h(key)}" ${key ? "readonly" : ""} placeholder="DATABASE_URL" /></div>
      <div class="field"><label>Value</label><textarea name="value" rows="4">${h(value)}</textarea></div>`,
    submit: "Save",
    onSubmit: async (f) => {
      const k = val(f, "key");
      if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(k)) throw new Error("Use letters, digits and underscores, not starting with a digit");
      await rpc("secret_set", { target: targetOf(S.sel), vars: [[k, f.value.value]] });
    },
  });
  if (ok) {
    toast("Saved");
    await afterVarChange();
  }
}

async function addProvider(org) {
  const fields = {
    "azure-keyvault": `
      <div class="field"><label>Vault URL</label><input type="text" name="vault_url" placeholder="https://chally-kv.vault.azure.net" /></div>
      <div class="field"><label>Sign in with</label><select name="auth">
        <option value="cli">Azure CLI (az login), using your own access</option>
        <option value="client-secret">Service principal (app ID and secret)</option></select></div>
      <div data-sp style="display:none">
        <div class="field"><label>Tenant ID</label><input type="text" name="tenant_id" /></div>
        <div class="field"><label>Client (app) ID</label><input type="text" name="client_id" /></div>
        <div class="field"><label>Client secret</label><input type="password" name="client_secret" /><div class="hint">Stored encrypted in your vault.</div></div>
      </div>
      <div data-cli class="field"><label>Tenant ID (optional)</label><input type="text" name="cli_tenant" placeholder="Only if you use several tenants" /></div>`,
    dotenv: `
      <div class="field"><label>File path</label><input type="text" name="path" placeholder="~/clients/{org}/{project}/.env.{env}" />
        <div class="hint">Absolute or ~ path. {org} {project} {env} are filled in per environment.</div></div>`,
    command: `
      <div class="field"><label>Command</label><textarea name="command" rows="3" placeholder="doppler secrets download --no-file --format env --project {project} --config {env}"></textarea>
        <div class="hint">Must print dotenv lines or a JSON object. Works with 1Password (<code>op</code>), Doppler, AWS, Infisical, sops and others.</div></div>
      <div class="field"><label>Output format</label><select name="format"><option value="dotenv">dotenv</option><option value="json">JSON</option></select></div>`,
  };
  const root = $("#modal-root");
  const p = modal({
    title: `Add provider to ${org}`,
    wide: true,
    body: `
      <div class="field"><label>Type</label>
        <div class="seg" id="kind">${Object.entries(KINDS).map(([k, v], i) => `<button type="button" data-k="${k}" class="${i === 0 ? "on" : ""}">${icon(v.icon, "sm")} ${v.label}</button>`).join("")}</div></div>
      <div class="field"><label>Name</label><input type="text" name="name" required placeholder="kv-staging" /></div>
      <input type="hidden" name="kind" value="azure-keyvault" />
      <div id="kind-fields">${fields["azure-keyvault"]}</div>`,
    submit: "Add provider",
    onSubmit: async (f) => {
      const kind = f.kind.value;
      const cfg = {};
      if (kind === "azure-keyvault") {
        cfg.vault_url = val(f, "vault_url");
        if (!/^https:\/\//.test(cfg.vault_url)) throw new Error("Vault URL should look like https://<name>.vault.azure.net");
        cfg.auth = f.auth.value;
        if (cfg.auth === "client-secret") {
          for (const k of ["tenant_id", "client_id", "client_secret"]) {
            if (!val(f, k)) throw new Error("Tenant ID, client ID and secret are required for a service principal");
            cfg[k] = k === "client_secret" ? f[k].value : val(f, k);
          }
        } else if (val(f, "cli_tenant")) cfg.tenant_id = val(f, "cli_tenant");
      } else if (kind === "dotenv") {
        cfg.path = val(f, "path");
        if (!/^[/~]/.test(cfg.path)) throw new Error("Use an absolute path or one starting with ~/");
      } else {
        cfg.command = val(f, "command");
        if (!cfg.command) throw new Error("Command is required");
        cfg.format = f.format.value;
      }
      await ignoreExists(rpc("org_add", { name: org }));
      await rpc("provider_add", { org, name: val(f, "name"), kind, config: cfg, replace: false });
      return val(f, "name");
    },
  });
  const form = root._form;
  const wireKind = () => {
    const sel = form.auth;
    if (sel) sel.onchange = () => {
      $("[data-sp]", form).style.display = sel.value === "client-secret" ? "" : "none";
      $("[data-cli]", form).style.display = sel.value === "client-secret" ? "none" : "";
    };
  };
  wireKind();
  $("#kind", form).onclick = (e) => {
    const b = e.target.closest("button[data-k]");
    if (!b) return;
    form.querySelectorAll("#kind button").forEach((x) => x.classList.toggle("on", x === b));
    form.kind.value = b.dataset.k;
    $("#kind-fields", form).innerHTML = fields[b.dataset.k];
    wireKind();
  };
  const name = await p;
  if (name) {
    await reloadTree();
    renderApp(false);
    toast(`Added ${name}. Testing…`);
    ACTIONS["test-provider"]({ name });
  }
}

// The import dialog's file picker fills the text area.
document.addEventListener("change", (e) => {
  if (e.target.id !== "envfile" || !e.target.files[0]) return;
  const reader = new FileReader();
  reader.onload = () => {
    const ta = e.target.form?.elements.text;
    if (ta) ta.value = reader.result;
  };
  reader.readAsText(e.target.files[0]);
});

boot();
