// ViFastKey — phần server demo: API Python, thống kê và góp ý.
// Không phụ thuộc app.js/WASM; nếu fetch lỗi thì playground WASM vẫn chạy.

const byId = (id) => document.getElementById(id);
const VISITOR_KEY = "vifastkey.visitor.v1";
const VISIT_SESSION_KEY = "vifastkey.visited.v1";
const DOWNLOAD_URL = "/api/download";
const number = new Intl.NumberFormat("vi-VN");

function visitorId() {
  try {
    let id = localStorage.getItem(VISITOR_KEY);
    if (!id) {
      id = crypto.randomUUID?.() || `vfk-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
      localStorage.setItem(VISITOR_KEY, id);
    }
    return id;
  } catch {
    return null;
  }
}

async function api(path, options = {}) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 12000);
  try {
    const response = await fetch(path, {
      ...options,
      signal: controller.signal,
      headers: { accept: "application/json", ...(options.headers || {}) },
    });
    const text = await response.text();
    let body = null;
    try {
      body = JSON.parse(text);
    } catch {
      // Giữ null; bên dưới dùng HTTP status để báo lỗi.
    }
    if (!response.ok) {
      const detail = Array.isArray(body?.detail)
        ? body.detail.map((item) => item?.msg || String(item)).join("; ")
        : body?.detail;
      throw new Error(detail || `Máy chủ trả ${response.status}`);
    }
    return body;
  } finally {
    clearTimeout(timer);
  }
}

function setHeaderLive(ok, detail = "") {
  const live = byId("header-live");
  if (!live) return;
  live.classList.toggle("off", !ok);
  live.title = detail || (ok ? "API Python đang chạy" : "API Python tạm không kết nối");
}

async function loadMeta() {
  const meta = byId("api-meta");
  try {
    const data = await api("/api/meta");
    setHeaderLive(true, `Python ${data.version} · ${data.engine}`);
    if (meta) meta.textContent = `Python ${data.version}`;
    const raw = byId("api-raw");
    if (raw && data.max_type_chars) raw.maxLength = data.max_type_chars;
  } catch {
    setHeaderLive(false);
    if (meta) meta.textContent = "không kết nối";
  }
}

function renderDownloads(items) {
  const box = byId("download-platforms");
  if (!box) return;
  const list = Array.isArray(items) ? items : [];
  box.textContent = "";
  if (!list.some((item) => Number(item.count) > 0)) {
    const empty = document.createElement("p");
    empty.className = "stats-empty";
    empty.textContent = "Chưa có lượt bấm tải nào. Hãy thử nút tải phía trên.";
    box.appendChild(empty);
    return;
  }
  const max = Math.max(1, ...list.map((item) => Number(item.count) || 0));
  for (const item of list) {
    const row = document.createElement("div");
    row.className = "dl-stat-row";
    const label = document.createElement("span");
    label.textContent = item.label || item.platform;
    const bar = document.createElement("div");
    bar.className = "dl-bar";
    const fill = document.createElement("span");
    const count = Number(item.count) || 0;
    fill.style.width = count ? `${Math.max(4, (count / max) * 100).toFixed(1)}%` : "0";
    bar.appendChild(fill);
    const value = document.createElement("b");
    value.textContent = number.format(count);
    row.append(label, bar, value);
    box.appendChild(row);
  }
}

async function loadStats() {
  const node = byId("footer-stats");
  const setStat = (id, value) => {
    const el = byId(id);
    if (el) el.textContent = number.format(value || 0);
  };
  try {
    const stats = await api("/api/stats");
    if (node) {
      node.textContent = `${number.format(stats.page_views)} lượt xem · ${number.format(stats.demos)} lượt API · ${number.format(stats.downloads_total || 0)} lượt tải`;
    }
    setStat("stat-pageviews", stats.page_views);
    setStat("stat-demos", stats.demos);
    setStat("stat-visitors", stats.visitors);
    setStat("stat-chars", stats.chars_total);
    setStat("stat-feedback", stats.feedback_total);
    setStat("stat-downloads", stats.downloads_total);
    const today = byId("stat-demos-today");
    if (today) today.textContent = `${number.format(stats.demos_today || 0)} hôm nay`;
    const downloadsToday = byId("stat-downloads-today");
    if (downloadsToday) downloadsToday.textContent = `${number.format(stats.downloads_today || 0)} hôm nay`;
    renderDownloads(stats.downloads_by_platform);
  } catch {
    if (node) node.textContent = "Python API · không tải được thống kê";
  }
}

function trackDownload(asset) {
  api(DOWNLOAD_URL, {
    method: "POST",
    headers: { "content-type": "application/json" },
    keepalive: true,
    body: JSON.stringify({ asset, visitor_id: visitorId(), source: "web" }),
  }).catch(() => {});
}

function setupDownloads() {
  document.querySelectorAll("[data-download]").forEach((link) => {
    link.addEventListener("click", () => {
      trackDownload(link.dataset.download);
      setTimeout(loadStats, 500);
    });
  });
}

function trackVisit() {
  try {
    if (sessionStorage.getItem(VISIT_SESSION_KEY)) return;
    sessionStorage.setItem(VISIT_SESSION_KEY, "1");
  } catch {
    // Chế độ riêng tư: vẫn gửi một lượt, chấp nhận đếm trùng.
  }
  api("/api/visit", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ visitor_id: visitorId() }),
  }).catch(() => {});
}

function setupServerDemo() {
  const raw = byId("api-raw");
  const method = byId("api-method");
  const tone = byId("api-tone");
  const run = byId("api-run");
  const out = byId("api-out");
  const latency = byId("api-latency");
  const meta = byId("api-meta");
  const response = byId("api-response");
  if (!raw || !method || !tone || !run || !out) return;

  const showError = (message) => {
    out.textContent = message;
    out.classList.add("error");
    if (latency) latency.textContent = "lỗi";
    if (meta) meta.textContent = "";
  };

  const execute = async () => {
    const text = raw.value;
    if (!text.trim()) {
      showError("Nhập ít nhất một phím trước đã.");
      raw.focus();
      return;
    }
    run.disabled = true;
    out.classList.remove("error");
    if (latency) latency.textContent = "đang chuyển…";
    try {
      const data = await api("/api/type", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          text,
          method: method.value,
          tone: tone.value,
          mode: "replace",
          visitor_id: visitorId(),
        }),
      });
      out.textContent = data.output || "—";
      out.classList.remove("error");
      if (latency) latency.textContent = `${Number(data.elapsed_ms).toFixed(2)} ms`;
      if (meta) meta.textContent = `${data.method} · ${data.tone} · Python ${data.version}`;
      if (response) {
        response.textContent = JSON.stringify({
          output: data.output,
          engine: data.engine,
          version: data.version,
          elapsed_ms: data.elapsed_ms,
        });
      }
      loadStats();
    } catch (error) {
      showError(error.message || "Không chuyển được; thử lại sau.");
    } finally {
      run.disabled = false;
    }
  };

  run.addEventListener("click", execute);
  raw.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
      event.preventDefault();
      execute();
    }
  });
}

function setupFeedback() {
  const form = byId("feedback-form");
  const status = byId("feedback-status");
  const submit = byId("feedback-submit");
  if (!form) return;

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const message = byId("feedback-message")?.value?.trim() || "";
    if (message.length < 5) {
      if (status) {
        status.textContent = "Nội dung cần ít nhất 5 ký tự.";
        status.className = "feedback-status error";
      }
      return;
    }
    submit.disabled = true;
    if (status) {
      status.textContent = "Đang gửi…";
      status.className = "feedback-status";
    }
    try {
      await api("/api/feedback", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          category: byId("feedback-category")?.value || "gop_y",
          message,
          contact: byId("feedback-contact")?.value || "",
          visitor_id: visitorId(),
          website: form.elements.website?.value || "",
        }),
      });
      if (status) {
        status.textContent = "Đã gửi. Cảm ơn bạn!";
        status.className = "feedback-status ok";
      }
      form.reset();
    } catch (error) {
      if (status) {
        status.textContent = error.message || "Gửi chưa được; thử lại sau.";
        status.className = "feedback-status error";
      }
    } finally {
      submit.disabled = false;
    }
  });

  if (location.hash === "#gop-y") byId("feedback-message")?.focus();
}

function setupPlatformTabs() {
  const tabs = Array.from(document.querySelectorAll(".platform-tab"));
  const panels = Array.from(document.querySelectorAll(".platform-panel"));
  if (!tabs.length || !panels.length) return;

  const selectPlatform = (name, focus = false) => {
    for (const tab of tabs) {
      const active = tab.dataset.platform === name;
      tab.setAttribute("aria-selected", active ? "true" : "false");
      tab.tabIndex = active ? 0 : -1;
      if (active && focus) tab.focus();
    }
    for (const panel of panels) {
      panel.hidden = panel.id !== `panel-${name}`;
    }
  };

  for (const tab of tabs) {
    tab.addEventListener("click", () => selectPlatform(tab.dataset.platform));
    tab.addEventListener("keydown", (event) => {
      const index = tabs.indexOf(tab);
      let next = null;
      if (event.key === "ArrowRight") next = (index + 1) % tabs.length;
      if (event.key === "ArrowLeft") next = (index - 1 + tabs.length) % tabs.length;
      if (event.key === "Home") next = 0;
      if (event.key === "End") next = tabs.length - 1;
      if (next === null) return;
      event.preventDefault();
      selectPlatform(tabs[next].dataset.platform, true);
    });
  }

  const detectPlatform = () => {
    const ua = navigator.userAgent;
    if (/Android/i.test(ua)) return "android";
    if (/iPhone|iPad|iPod/i.test(ua)) return "ios";
    if (/Macintosh|Mac OS X/i.test(ua)) return "macos";
    if (/Linux/i.test(ua)) return "linux";
    return "windows";
  };

  const openFromHash = () => {
    const match = location.hash.match(/^#panel-(windows|macos|linux|android|ios|extension|uefi|cli)$/);
    if (!match) return;
    selectPlatform(match[1]);
    document.getElementById(`panel-${match[1]}`)?.scrollIntoView({ block: "start" });
  };

  selectPlatform(detectPlatform());
  openFromHash();
  window.addEventListener("hashchange", openFromHash);
}

setupServerDemo();
setupFeedback();
setupPlatformTabs();
setupDownloads();
loadMeta();
loadStats();
trackVisit();
