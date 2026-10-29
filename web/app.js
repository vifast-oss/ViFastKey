// ViFastKey web demo: engine Rust → WASM, ô nhập mô phỏng chế độ REPLACE.
import init, { WasmSession } from "./pkg/vfk_wasm.js";

const STORAGE_KEY = "vifastkey.demo.config.v1";
const METHODS = {
  telex: "Telex",
  simple_telex: "Simple Telex",
  vni: "VNI",
};
const BOOL_KEYS = [
  "spell_check",
  "w_as_u",
  "bracket_keys",
  "quick_telex",
  "auto_restore_english",
  "auto_capitalize",
  "smart_caps",
];

const DEFAULT_CONFIG = {
  input_method: "telex",
  tone_style: "old",
  spell_check: true,
  w_as_u: true,
  bracket_keys: false,
  quick_telex: false,
  auto_restore_english: true,
  auto_capitalize: false,
  smart_caps: false,
  macro_enabled: true,
  edit_previous_word: true,
  unicode_form: "NFC",
};

const $ = (sel) => document.querySelector(sel);
const pad = $("#pad");
const rawLine = $("#raw-line");
const actionLine = $("#action-line");
const modePill = $("#pg-mode");
const viToggle = $("#toggle-vi");
const viLabel = $("#vi-label");
const liveDot = $(".live-dot");

const state = {
  ready: false,
  text: "",
  cursor: 0,
  raw: "",
  enabled: true,
  config: { ...DEFAULT_CONFIG },
  lastAction: null,
};

let session = null;
let renderEnabled = true;

const clone = (value) => JSON.parse(JSON.stringify(value));
const chars = (value) => Array.from(value);
const cpLen = (value) => chars(value).length;
const cpToUtf16 = (value, n) => chars(value).slice(0, n).join("").length;
const utf16ToCp = (value, i) => chars(value.slice(0, i)).length;

function normalizeConfig(raw) {
  const config = { ...DEFAULT_CONFIG };
  if (!raw || typeof raw !== "object") return config;
  if (Object.hasOwn(METHODS, raw.input_method)) config.input_method = raw.input_method;
  if (raw.tone_style === "old" || raw.tone_style === "new") config.tone_style = raw.tone_style;
  for (const key of BOOL_KEYS) {
    if (typeof raw[key] === "boolean") config[key] = raw[key];
  }
  return config;
}

function loadConfig() {
  try {
    return normalizeConfig(JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}"));
  } catch {
    return { ...DEFAULT_CONFIG };
  }
}

function saveConfig() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state.config));
  } catch {
    // Chế độ riêng tư / chặn localStorage: bỏ qua, demo vẫn chạy.
  }
}

function setStatus(text, isError = false) {
  actionLine.textContent = text;
  actionLine.classList.toggle("error", isError);
  actionLine.title = "";
}

function describeAction(action) {
  if (!action) return "sẵn sàng";
  if (action.type === "pass") return "pass";
  if (action.type === "replace") return `replace −${action.delete_before} +${cpLen(action.insert)}`;
  if (action.type === "commit") return `commit +${cpLen(action.text)}`;
  if (action.type === "preedit") return `preedit ${cpLen(action.text)}`;
  return action.type;
}

function updateMeta() {
  if (!state.ready) return;
  const raw = session ? session.raw() : "";
  state.raw = raw;
  rawLine.textContent = `Phím thô: ${raw || "—"}`;
  if (state.lastAction) {
    actionLine.textContent = describeAction(state.lastAction);
    actionLine.title = JSON.stringify(state.lastAction);
  } else {
    setStatus("sẵn sàng");
  }
  actionLine.classList.remove("error");
}

function render() {
  if (!renderEnabled || !state.ready) return;
  const scroll = pad.scrollTop;
  pad.value = state.text;
  const pos = cpToUtf16(state.text, state.cursor);
  pad.setSelectionRange(pos, pos);
  pad.scrollTop = scroll;
  updateMeta();
}

function insertText(text) {
  const list = chars(state.text);
  const insert = chars(text);
  list.splice(state.cursor, 0, ...insert);
  state.text = list.join("");
  state.cursor += insert.length;
}

function deleteBefore(n = 1) {
  if (state.cursor <= 0) return;
  const list = chars(state.text);
  const count = Math.min(n, state.cursor);
  list.splice(state.cursor - count, count);
  state.text = list.join("");
  state.cursor -= count;
}

function deleteAtCursor() {
  const list = chars(state.text);
  if (state.cursor >= list.length) return;
  list.splice(state.cursor, 1);
  state.text = list.join("");
}

function applyDefault(typedChar, keyName) {
  if (typedChar != null) {
    insertText(typedChar);
    return;
  }
  if (keyName === "Backspace") deleteBefore(1);
  else if (keyName === "Delete") deleteAtCursor();
  else if (keyName === "Enter") insertText("\n");
}

function applyAction(action, { typedChar = null, keyName = null } = {}) {
  const list = chars(state.text);
  if (action.type === "replace") {
    const del = Math.max(0, Math.min(action.delete_before, state.cursor));
    const insert = chars(action.insert);
    list.splice(state.cursor - del, del, ...insert);
    state.text = list.join("");
    state.cursor = state.cursor - del + insert.length;
    if (action.pass_key) applyDefault(typedChar, keyName);
  } else if (action.type === "commit") {
    const insert = chars(action.text);
    list.splice(state.cursor, 0, ...insert);
    state.text = list.join("");
    state.cursor += insert.length;
    if (action.pass_key) applyDefault(typedChar, keyName);
  } else if (action.type === "pass") {
    applyDefault(typedChar, keyName);
  }
  // PREEDIT không dùng trong demo (demo chạy REPLACE); engine vẫn trả về nếu đổi mode.
  state.lastAction = action;
  render();
}

function processKey(keyName, typedChar = null) {
  if (!state.ready) return;
  const before = chars(state.text).slice(0, state.cursor).join("");
  session.set_surrounding(before);
  const action = JSON.parse(session.process_key(keyName, false, false, false));
  applyAction(action, { typedChar, keyName });
}

function selectionHasRange() {
  return pad.selectionStart !== pad.selectionEnd;
}

function deleteSelection() {
  if (!selectionHasRange()) return false;
  const start = pad.selectionStart;
  const end = pad.selectionEnd;
  const value = pad.value.slice(0, start) + pad.value.slice(end);
  state.text = value;
  state.cursor = utf16ToCp(value, start);
  session.reset();
  state.raw = "";
  state.lastAction = null;
  render();
  return true;
}

function syncFromPad({ flush = true } = {}) {
  if (!state.ready) return;
  state.text = pad.value;
  state.cursor = utf16ToCp(pad.value, pad.selectionEnd ?? pad.value.length);
  if (flush) {
    const action = JSON.parse(session.cursor_moved());
    if (action.type !== "pass") applyAction(action, {});
    else {
      state.lastAction = null;
      state.raw = "";
      updateMeta();
    }
  }
}

function updateEnabledUI() {
  const enabled = session ? session.is_enabled() : true;
  state.enabled = enabled;
  viToggle.setAttribute("aria-pressed", String(enabled));
  viLabel.textContent = enabled ? "Tiếng Việt" : "Tiếng Anh";
  liveDot.classList.toggle("off", !enabled);
}

function syncControls() {
  for (const button of document.querySelectorAll("[data-method]")) {
    button.classList.toggle("active", button.dataset.method === state.config.input_method);
  }
  for (const button of document.querySelectorAll("[data-tone]")) {
    button.classList.toggle("active", button.dataset.tone === state.config.tone_style);
  }
  for (const checkbox of document.querySelectorAll("[data-opt]")) {
    checkbox.checked = Boolean(state.config[checkbox.dataset.opt]);
  }
  modePill.textContent = METHODS[state.config.input_method] || "Telex";
}

function pushConfig() {
  if (!session) return;
  const ok = session.set_config(JSON.stringify(state.config));
  if (!ok) {
    setStatus("Cấu hình không hợp lệ", true);
    return;
  }
  saveConfig();
  syncControls();
  updateMeta();
}

function setMethod(method) {
  if (!Object.hasOwn(METHODS, method) || method === state.config.input_method) {
    syncControls();
    return;
  }
  state.config.input_method = method;
  pushConfig();
  state.raw = "";
  state.lastAction = null;
  session.reset();
  updateMeta();
}

function setTone(tone) {
  if (tone !== "old" && tone !== "new") return;
  state.config.tone_style = tone;
  pushConfig();
  state.raw = "";
  state.lastAction = null;
  session.reset();
  updateMeta();
}

function clearPad({ focus = true } = {}) {
  state.text = "";
  state.cursor = 0;
  state.raw = "";
  state.lastAction = null;
  if (session) session.reset();
  render();
  updateMeta();
  if (focus) pad.focus();
}

function finalizeWord() {
  const before = chars(state.text).slice(0, state.cursor).join("");
  session.set_surrounding(before);
  const action = JSON.parse(session.process_key("Enter", false, false, false));
  applyAction(action, { typedChar: null, keyName: "Enter" });
  if (state.text.endsWith("\n")) {
    state.text = state.text.slice(0, -1);
    state.cursor = Math.min(state.cursor, cpLen(state.text));
    render();
  }
}

function runExample(text) {
  if (!state.ready) return;
  if (!state.enabled) {
    applyAction(JSON.parse(session.toggle_vietnamese()), {});
    updateEnabledUI();
  }
  if (state.config.input_method !== "telex") {
    state.config.input_method = "telex";
    pushConfig();
  }
  state.text = "";
  state.cursor = 0;
  state.raw = "";
  state.lastAction = null;
  session.reset();

  renderEnabled = false;
  for (const ch of text) {
    const before = chars(state.text).slice(0, state.cursor).join("");
    session.set_surrounding(before);
    const action = JSON.parse(session.process_key(ch, false, false, false));
    applyAction(action, { typedChar: ch, keyName: ch });
  }
  renderEnabled = true;
  finalizeWord();
  pad.focus();
}

function setupEvents() {
  pad.addEventListener("keydown", (event) => {
    if (event.isComposing || event.keyCode === 229) return;
    if (event.ctrlKey || event.metaKey || event.altKey || event.getModifierState?.("AltGraph")) return;

    const key = event.key;
    if (key === "Tab") {
      if (session) session.cursor_moved();
      state.raw = "";
      updateMeta();
      return;
    }

    if (key === "Backspace") {
      event.preventDefault();
      deleteSelection();
      processKey("Backspace", null);
      return;
    }
    if (key === "Enter") {
      event.preventDefault();
      deleteSelection();
      processKey("Enter", null);
      return;
    }
    if (key === "Escape") {
      event.preventDefault();
      processKey("Escape", null);
      return;
    }
    if (key === "Delete") {
      event.preventDefault();
      deleteSelection();
      processKey("Delete", null);
      return;
    }
    if (key.length === 1) {
      event.preventDefault();
      deleteSelection();
      processKey(key, key);
      return;
    }
    // Mũi tên/Home/End: để trình duyệt di chuyển caret, đồng bộ lại sau keyup.
  });

  pad.addEventListener("keyup", (event) => {
    if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End", "PageUp", "PageDown"].includes(event.key)) {
      syncFromPad();
    }
  });

  pad.addEventListener("click", () => syncFromPad());
  pad.addEventListener("blur", () => {
    if (!session) return;
    session.cursor_moved();
    state.lastAction = null;
    state.raw = "";
  });

  pad.addEventListener("input", () => {
    // Paste / autofill / undo: đồng bộ lại văn bản và reset từ đang gõ.
    state.text = pad.value;
    state.cursor = utf16ToCp(pad.value, pad.selectionEnd ?? pad.value.length);
    if (session) session.reset();
    state.raw = "";
    state.lastAction = null;
    updateMeta();
  });

  for (const button of document.querySelectorAll("[data-method]")) {
    button.addEventListener("click", () => setMethod(button.dataset.method));
  }
  for (const button of document.querySelectorAll("[data-tone]")) {
    button.addEventListener("click", () => setTone(button.dataset.tone));
  }
  for (const checkbox of document.querySelectorAll("[data-opt]")) {
    checkbox.addEventListener("change", () => {
      state.config[checkbox.dataset.opt] = checkbox.checked;
      pushConfig();
      state.lastAction = null;
      state.raw = "";
      session.reset();
      updateMeta();
    });
  }

  viToggle.addEventListener("click", () => {
    if (!session) return;
    const action = JSON.parse(session.toggle_vietnamese());
    applyAction(action, {});
    updateEnabledUI();
  });

  $("#clear-pad").addEventListener("click", () => clearPad());
  for (const button of document.querySelectorAll("[data-example]")) {
    button.addEventListener("click", () => runExample(button.dataset.example));
  }

  document.querySelector(".pg-screen").addEventListener("click", (event) => {
    if (event.target !== pad) pad.focus();
  });
}

async function boot() {
  state.config = loadConfig();
  try {
    await init();
    session = new WasmSession(JSON.stringify(state.config), "replace");
    state.ready = true;
    syncControls();
    updateEnabledUI();
    setupEvents();
    state.lastAction = null;
    render();
    updateMeta();
  } catch (error) {
    console.error(error);
    setStatus("Không nạp được WASM — hãy chạy bằng web server", true);
    pad.placeholder = "Mở thư mục web bằng `python3 -m http.server` rồi tải lại trang.";
  }
}

boot();
