// ViFastKey — phần server demo: API Python, thống kê và góp ý.
// Không phụ thuộc app.js/WASM; nếu fetch lỗi thì playground WASM vẫn chạy.

const byId = (id) => document.getElementById(id);
const VISITOR_KEY = "vifastkey.visitor.v1";
const VISIT_SESSION_KEY = "vifastkey.visited.v1";
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

async function loadStats() {
  const node = byId("footer-stats");
  if (!node) return;
  try {
    const stats = await api("/api/stats");
    node.textContent = `${number.format(stats.page_views)} lượt xem · ${number.format(stats.demos)} lượt API · ${number.format(stats.chars_total)} ký tự`;
  } catch {
    node.textContent = "Python API · không tải được thống kê";
  }
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

setupServerDemo();
setupFeedback();
loadMeta();
loadStats();
trackVisit();
