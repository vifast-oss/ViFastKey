// ViFastKey web demo: engine Rust → WASM.
// Hỗ trợ cả hai mô hình đầu ra:
//   REPLACE — áp từng Action lên văn bản, kiểu UniKey/OpenKey.
//   PREEDIT — giữ vùng preedit gạch chân, commit nguyên tử khi kết thúc từ (TSF/IMK/Fcitx5).
import init, {
  WasmSession,
  charset_decode,
  charset_encode,
  charset_encode_bytes,
  charset_list,
} from "./pkg/vfk_wasm.js";

const STORAGE_KEY = "vifastkey.demo.config.v2";
// Khớp MAX_RAW trong engine: vượt 24 phím thì từ chuyển sang chế độ thô.
const MAX_RAW = 24;
const METHODS = {
  telex: "Telex",
  simple_telex: "Simple Telex",
  vni: "VNI",
  viqr: "VIQR",
};
const BOOL_KEYS = [
  "spell_check",
  "w_as_u",
  "bracket_keys",
  "quick_telex",
  "auto_restore_english",
  "auto_capitalize",
  "smart_caps",
  "macro_enabled",
  "macro_digits",
  "allow_coda_k",
  "edit_previous_word",
];

const DEFAULT_MACROS = { ko: "không", vn: "Việt Nam", dc: "được" };

const DEFAULT_CONFIG = {
  input_method: "telex",
  tone_style: "new",
  spell_check: true,
  w_as_u: false,
  bracket_keys: false,
  quick_telex: false,
  auto_restore_english: true,
  auto_capitalize: false,
  smart_caps: false,
  macro_enabled: true,
  macro_digits: false,
  allow_coda_k: false,
  edit_previous_word: true,
  macros: DEFAULT_MACROS,
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
const macroText = $("#macro-text");
const unicodeForm = $("#unicode-form");
const csFrom = $("#cs-from");
const csTo = $("#cs-to");
const csText = $("#cs-text");
const csOut = $("#cs-out");

const state = {
  ready: false,
  text: "",
  cursor: 0,
  raw: "",
  enabled: true,
  config: { ...DEFAULT_CONFIG, macros: { ...DEFAULT_MACROS } },
  lastAction: null,
  mode: "preedit",
  preeditActive: false,
  preeditStart: 0,
  preeditLen: 0,
  composing: false,
  compositionBase: null,
  longLiteral: false,
  skipAdopt: false,
};

let session = null;
let renderEnabled = true;

const chars = (value) => Array.from(value);
const cpLen = (value) => chars(value).length;
const cpToUtf16 = (value, n) => chars(value).slice(0, n).join("").length;
const utf16ToCp = (value, i) => chars(value.slice(0, i)).length;

function normalizeConfig(raw) {
  const config = { ...DEFAULT_CONFIG, macros: { ...DEFAULT_MACROS } };
  if (!raw || typeof raw !== "object") return config;
  if (Object.hasOwn(METHODS, raw.input_method)) config.input_method = raw.input_method;
  if (raw.tone_style === "old" || raw.tone_style === "new") config.tone_style = raw.tone_style;
  if (raw.unicode_form === "NFC" || raw.unicode_form === "NFD") config.unicode_form = raw.unicode_form;
  for (const key of BOOL_KEYS) {
    if (typeof raw[key] === "boolean") config[key] = raw[key];
  }
  if (raw.macros && typeof raw.macros === "object" && !Array.isArray(raw.macros)) {
    const macros = {};
    for (const [key, value] of Object.entries(raw.macros)) {
      if (typeof value === "string" && key.trim()) macros[key.trim().toLowerCase()] = value;
    }
    config.macros = macros;
  }
  return config;
}

function loadConfig() {
  try {
    return normalizeConfig(JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}"));
  } catch {
    return normalizeConfig({});
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
  state.longLiteral = raw.length > MAX_RAW;
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

function clearPreeditState() {
  state.preeditActive = false;
  state.preeditStart = 0;
  state.preeditLen = 0;
}

function applyAction(action, { typedChar = null, keyName = null } = {}) {
  let list = chars(state.text);
  if (action.type === "preedit") {
    const start = state.preeditActive ? state.preeditStart : state.cursor;
    if (state.preeditActive) list.splice(state.preeditStart, state.preeditLen);
    const insert = chars(action.text);
    list.splice(start, 0, ...insert);
    state.text = list.join("");
    state.preeditActive = true;
    state.preeditStart = start;
    state.preeditLen = insert.length;
    state.cursor = start + insert.length;
  } else if (action.type === "replace") {
    const start = state.preeditActive ? state.preeditStart : state.cursor;
    if (state.preeditActive) list.splice(state.preeditStart, state.preeditLen);
    clearPreeditState();
    const del = Math.max(0, Math.min(action.delete_before, start));
    const insert = chars(action.insert);
    list.splice(start - del, del, ...insert);
    state.text = list.join("");
    state.cursor = start - del + insert.length;
    if (action.pass_key) applyDefault(typedChar, keyName);
  } else if (action.type === "commit") {
    const start = state.preeditActive ? state.preeditStart : state.cursor;
    if (state.preeditActive) list.splice(state.preeditStart, state.preeditLen);
    clearPreeditState();
    const insert = chars(action.text);
    list.splice(start, 0, ...insert);
    state.text = list.join("");
    state.cursor = start + insert.length;
    if (action.pass_key) applyDefault(typedChar, keyName);
  } else if (action.type === "pass") {
    if (state.preeditActive && typedChar != null) clearPreeditState();
    applyDefault(typedChar, keyName);
  }
  state.lastAction = action;
  render();
}

function createSession() {
  session = new WasmSession(JSON.stringify(state.config), state.mode);
  clearPreeditState();
}

function processKey(keyName, typedChar = null) {
  if (!state.ready) return;
  const before = chars(state.text).slice(0, state.cursor).join("");
  const isChar = keyName.length === 1;
  // Sau khi cắt từ quá dài, không cho engine "adopt" phần chữ còn sót lại;
  // nếu không phím đầu tiên của từ mới sẽ bị ghép vào từ cũ. Giữ cờ này qua
  // các phím xoá/dấu cách, chỉ nhả khi bắt đầu ký tự của từ mới.
  session.set_surrounding(state.skipAdopt && isChar ? "" : before);
  if (state.skipAdopt && isChar) state.skipAdopt = false;
  const action = JSON.parse(session.process_key(keyName, false, false, false));
  applyAction(action, { typedChar, keyName });
}

function handleBackspace() {
  if (deleteSelection()) return;
  const wasLong = state.longLiteral;
  processKey("Backspace", null);
  // Từ > MAX_RAW bị engine giữ ở chế độ thô vĩnh viễn. Sau khi xoá, reset
  // session để từ tiếp theo gõ tiếng Việt được, kể cả khi còn chữ sót lại.
  if (wasLong && session) {
    session.reset();
    state.skipAdopt = true;
    state.longLiteral = false;
    clearPreeditState();
    state.raw = "";
    state.lastAction = null;
    updateMeta();
  }
}

// Bàn phím ảo Android/Gboard thường không phát keydown cho từng phím, hoặc
// phát keyCode 229. Các hàm dưới xử lý qua beforeinput/compositionend.
function isRawAscii(value) {
  return /^[\x20-\x7E]+$/.test(value);
}

function processTextAsKeys(value) {
  for (const ch of chars(value)) {
    if (ch === "\n") {
      processKey("Enter", null);
    } else {
      processKey(ch, ch);
    }
  }
}

function syncLiteralFromPad() {
  state.text = pad.value;
  state.cursor = utf16ToCp(pad.value, pad.selectionEnd ?? pad.value.length);
  if (session) session.reset();
  clearPreeditState();
  // Chữ đã do host/app commit là văn bản cuối, không phải raw để engine adopt.
  state.skipAdopt = true;
  state.raw = "";
  state.lastAction = null;
  updateMeta();
}

function restoreBase(base) {
  state.text = base.text;
  state.cursor = base.cursor;
  pad.value = base.text;
  if (base.start !== base.end) {
    pad.setSelectionRange(base.start, base.end);
    deleteSelection();
  } else {
    const pos = cpToUtf16(base.text, base.cursor);
    pad.setSelectionRange(pos, pos);
  }
}

function handleCompositionEnd(inserted) {
  const base = state.compositionBase;
  state.composing = false;
  state.compositionBase = null;
  if (!base) return;

  if (!inserted) {
    restoreBase(base);
    render();
    return;
  }

  // Gboard tiếng Việt đã commit chữ có dấu. Giữ nguyên văn bản trình duyệt đã
  // đưa vào, không ghi lại value (dễ làm Gboard mất composition). Đây là văn
  // bản cuối, không phải phím thô: reset nội bộ và không adopt lại.
  if (!isRawAscii(inserted) || inserted.length > 64) {
    syncLiteralFromPad();
    return;
  }

  // Chữ thô ASCII (Telex/VNI): trả textarea về trạng thái trước composition
  // rồi cho engine xử lý đúng chuỗi phím.
  restoreBase(base);
  if (inserted === "\n" || inserted === "\r") processKey("Enter", null);
  else processTextAsKeys(inserted);
}

function flushSession() {
  if (!session) return;
  const action = JSON.parse(session.cursor_moved());
  if (action.type !== "pass") {
    applyAction(action, {});
  } else {
    clearPreeditState();
    state.lastAction = null;
    state.raw = "";
    updateMeta();
  }
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
  if (session) session.reset();
  clearPreeditState();
  state.raw = "";
  state.lastAction = null;
  render();
  return true;
}

function syncFromPad({ flush = true } = {}) {
  if (!state.ready) return;
  const wanted = utf16ToCp(pad.value, pad.selectionEnd ?? pad.value.length);
  state.text = pad.value;
  state.cursor = wanted;
  if (!flush) {
    updateMeta();
    return;
  }
  const before = state.text;
  flushSession();
  // Chỉ khôi phục vị trí click nếu flush không làm đổi văn bản.
  if (state.text === before) {
    state.cursor = Math.min(wanted, cpLen(state.text));
    render();
  }
}

function updateEnabledUI() {
  const enabled = session ? session.is_enabled() : true;
  state.enabled = enabled;
  viToggle.setAttribute("aria-pressed", String(enabled));
  viLabel.textContent = enabled ? "Tiếng Việt" : "Tiếng Anh";
  liveDot.classList.toggle("off", !enabled);
}

function formatMacros(macros) {
  return Object.entries(macros || {}).map(([key, value]) => `${key}=${value}`).join("\n");
}

function parseMacros(text) {
  const out = {};
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;
    const eq = line.indexOf("=");
    if (eq <= 0) continue;
    const key = line.slice(0, eq).trim().toLowerCase();
    const value = line.slice(eq + 1).trim();
    if (key) out[key] = value;
  }
  return out;
}

function syncControls() {
  for (const button of document.querySelectorAll("[data-method]")) {
    button.classList.toggle("active", button.dataset.method === state.config.input_method);
  }
  for (const button of document.querySelectorAll("[data-tone]")) {
    button.classList.toggle("active", button.dataset.tone === state.config.tone_style);
  }
  for (const button of document.querySelectorAll("[data-output-mode]")) {
    button.classList.toggle("active", button.dataset.outputMode === state.mode);
  }
  for (const checkbox of document.querySelectorAll("[data-opt]")) {
    checkbox.checked = Boolean(state.config[checkbox.dataset.opt]);
  }
  if (document.activeElement !== macroText) macroText.value = formatMacros(state.config.macros);
  unicodeForm.value = state.config.unicode_form;
  const output = state.mode === "preedit" ? " · Preedit" : "";
  modePill.textContent = `${METHODS[state.config.input_method] || "Telex"}${output}`;
}

function pushConfig() {
  if (!session) return false;
  const ok = session.set_config(JSON.stringify(state.config));
  if (!ok) {
    setStatus("Cấu hình không hợp lệ", true);
    return false;
  }
  saveConfig();
  syncControls();
  clearPreeditState();
  updateMeta();
  return true;
}

function resetWord() {
  if (session) session.reset();
  clearPreeditState();
  state.raw = "";
  state.lastAction = null;
  updateMeta();
}

function setMethod(method) {
  if (!Object.hasOwn(METHODS, method) || method === state.config.input_method) {
    syncControls();
    return;
  }
  state.config.input_method = method;
  pushConfig();
  resetWord();
}

function setTone(tone) {
  if (tone !== "old" && tone !== "new") return;
  state.config.tone_style = tone;
  pushConfig();
  resetWord();
}

function setOutputMode(mode) {
  if (mode !== "replace" && mode !== "preedit") return;
  if (mode === state.mode) return;
  state.mode = mode;
  createSession();
  state.raw = "";
  state.lastAction = null;
  syncControls();
  updateEnabledUI();
  updateMeta();
  pad.focus();
}

function clearPad({ focus = true } = {}) {
  state.text = "";
  state.cursor = 0;
  resetWord();
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

function runExample(text, enableOption = null) {
  if (!state.ready) return;
  if (enableOption && !state.config[enableOption]) {
    state.config[enableOption] = true;
    if (!pushConfig()) return;
  }
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
  resetWord();

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

function setupCharsetTool() {
  const list = JSON.parse(charset_list());
  for (const select of [csFrom, csTo]) {
    select.replaceChildren();
    for (const item of list) {
      const option = document.createElement("option");
      option.value = item.key;
      option.textContent = item.name;
      select.append(option);
    }
  }
  csFrom.value = "unicode";
  csTo.value = "tcvn3";
  const run = () => {
    try {
      const decoded = csFrom.value === "unicode" ? csText.value : charset_decode(csFrom.value, csText.value);
      const encoded = charset_encode(csTo.value, decoded);
      const bytes = charset_encode_bytes(csTo.value, decoded);
      const hex = bytes.length ? ` · hex ${Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join(" ")}` : "";
      csOut.textContent = `${encoded}${hex}`;
    } catch (error) {
      csOut.textContent = `Lỗi: ${error}`;
    }
  };
  $("#cs-run").addEventListener("click", run);
  csText.addEventListener("change", run);
  csFrom.addEventListener("change", run);
  csTo.addEventListener("change", run);
  run();
}

function setupEvents() {
  pad.addEventListener("beforeinput", (event) => {
    if (!state.ready || event.isComposing || state.composing || !event.cancelable) return;
    const type = event.inputType;
    if (type === "insertText") {
      const data = event.data ?? event.dataTransfer?.getData("text/plain") ?? "";
      if (!data) return;
      // Chữ có dấu (Gboard tiếng Việt, dán, gợi ý) để trình duyệt tự chèn;
      // input handler sẽ đồng bộ nguyên văn và reset từ đang gõ.
      if (!isRawAscii(data)) return;
      event.preventDefault();
      deleteSelection();
      if (data === "\n" || data === "\r") processKey("Enter", null);
      else processTextAsKeys(data);
      return;
    }
    if (type === "deleteContentBackward") {
      event.preventDefault();
      handleBackspace();
      return;
    }
    if (type === "deleteContentForward") {
      event.preventDefault();
      if (!deleteSelection()) processKey("Delete", null);
      return;
    }
    if (type === "insertLineBreak" || type === "insertParagraph") {
      event.preventDefault();
      deleteSelection();
      processKey("Enter", null);
    }
  });

  pad.addEventListener("compositionstart", () => {
    if (!state.ready) return;
    state.composing = true;
    state.compositionBase = {
      text: state.text,
      cursor: state.cursor,
      start: pad.selectionStart ?? cpToUtf16(state.text, state.cursor),
      end: pad.selectionEnd ?? cpToUtf16(state.text, state.cursor),
    };
  });

  pad.addEventListener("compositionend", (event) => {
    if (!state.ready) return;
    const base = state.compositionBase;
    const committedValue = pad.value;
    let inserted = "";
    if (base) {
      const before = base.text.slice(0, base.start);
      const after = base.text.slice(base.end);
      if (committedValue.startsWith(before) && committedValue.endsWith(after)) {
        inserted = committedValue.slice(
          before.length,
          committedValue.length - after.length,
        );
      }
    }
    if (!inserted) inserted = event.data ?? "";
    handleCompositionEnd(inserted);
  });

  pad.addEventListener("keydown", (event) => {
    if (event.isComposing || state.composing || event.keyCode === 229) return;
    if (event.ctrlKey || event.metaKey || event.altKey || event.getModifierState?.("AltGraph")) return;

    const key = event.key;
    if (key === "Tab") {
      flushSession();
      state.raw = "";
      updateMeta();
      return;
    }

    if (key === "Backspace") {
      event.preventDefault();
      handleBackspace();
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
      if (!deleteSelection()) processKey("Delete", null);
      return;
    }
    if (key.length === 1) {
      // Chữ có dấu sẵn (Gboard tiếng Việt, bàn phím layout Việt) để trình duyệt
      // chèn như văn bản thường; beforeinput/input sẽ đồng bộ lại.
      if (key.codePointAt(0) > 0x7f) return;
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
  pad.addEventListener("blur", () => flushSession());

  pad.addEventListener("input", (event) => {
    // Trong lúc composition, textarea đang chứa chữ thô; compositionend sẽ xử lý.
    if (event.isComposing || state.composing) return;
    // beforeinput đã xử lý và render xong: bỏ qua input lặp.
    if (pad.value === state.text) {
      state.cursor = utf16ToCp(pad.value, pad.selectionEnd ?? pad.value.length);
      return;
    }
    // Paste / autofill / undo / xoá cả từ / chữ có dấu: đồng bộ nguyên văn
    // và reset từ đang gõ.
    syncLiteralFromPad();
  });

  for (const button of document.querySelectorAll("[data-method]")) {
    button.addEventListener("click", () => setMethod(button.dataset.method));
  }
  for (const button of document.querySelectorAll("[data-tone]")) {
    button.addEventListener("click", () => setTone(button.dataset.tone));
  }
  for (const button of document.querySelectorAll("[data-output-mode]")) {
    button.addEventListener("click", () => setOutputMode(button.dataset.outputMode));
  }
  for (const checkbox of document.querySelectorAll("[data-opt]")) {
    checkbox.addEventListener("change", () => {
      state.config[checkbox.dataset.opt] = checkbox.checked;
      pushConfig();
      resetWord();
    });
  }

  macroText.addEventListener("change", () => {
    state.config.macros = parseMacros(macroText.value);
    if (pushConfig()) resetWord();
  });
  unicodeForm.addEventListener("change", () => {
    state.config.unicode_form = unicodeForm.value;
    if (pushConfig()) resetWord();
  });

  viToggle.addEventListener("click", () => {
    if (!session) return;
    const action = JSON.parse(session.toggle_vietnamese());
    applyAction(action, {});
    updateEnabledUI();
  });

  $("#clear-pad").addEventListener("click", () => clearPad());
  for (const button of document.querySelectorAll("[data-example]")) {
    button.addEventListener("click", () => runExample(button.dataset.example, button.dataset.enable || null));
  }

  document.querySelector(".pg-screen").addEventListener("click", (event) => {
    if (event.target !== pad) pad.focus();
  });
}

async function boot() {
  state.config = loadConfig();
  try {
    await init("./pkg/vfk_wasm_bg.wasm?v=20261031g");
    createSession();
    state.ready = true;
    syncControls();
    updateEnabledUI();
    setupCharsetTool();
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
