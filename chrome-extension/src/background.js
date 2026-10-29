// Service worker: context menu + WASM normalize. Chỉ dùng activeTab,
// không có content script thường trú và không đọc trang nếu người dùng không chọn.
import init, { normalize_text } from "./pkg/vfk_wasm.js";

const ready = init();

async function normalize(text, method = "telex", tone = "old") {
  await ready;
  return normalize_text(text, method, tone);
}

function setBadge(tabId, text) {
  chrome.action.setBadgeText({ tabId, text });
  setTimeout(() => chrome.action.setBadgeText({ tabId, text: "" }), 1200);
}

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: "vifastkey-normalize",
      title: "ViFastKey: chuẩn hoá vùng chọn",
      contexts: ["selection"],
    });
    chrome.contextMenus.create({
      id: "vifastkey-remove-diacritics",
      title: "ViFastKey: bỏ dấu vùng chọn",
      contexts: ["selection"],
    });
  });
});

async function activeSelection(tabId) {
  const [result] = await chrome.scripting.executeScript({
    target: { tabId },
    func: () => {
      const el = document.activeElement;
      if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA")
          && typeof el.selectionStart === "number") {
        return el.value.slice(el.selectionStart, el.selectionEnd);
      }
      return window.getSelection()?.toString() ?? "";
    },
  });
  return result?.result ?? "";
}

async function replaceSelection(tabId, text) {
  await chrome.scripting.executeScript({
    target: { tabId },
    func: (value) => {
      const el = document.activeElement;
      if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA")
          && typeof el.selectionStart === "number") {
        const start = el.selectionStart;
        const end = el.selectionEnd;
        el.setRangeText(value, start, end, "end");
        el.dispatchEvent(new Event("input", { bubbles: true }));
        el.dispatchEvent(new Event("change", { bubbles: true }));
        return true;
      }
      const selection = window.getSelection();
      if (selection && selection.rangeCount > 0 && !selection.isCollapsed) {
        const range = selection.getRangeAt(0);
        range.deleteContents();
        range.insertNode(document.createTextNode(value));
        selection.removeAllRanges();
        const after = document.createRange();
        after.setStartAfter(range.endContainer);
        after.collapse(true);
        selection.addRange(after);
        return true;
      }
      return false;
    },
    args: [text],
  });
}

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (!tab?.id) return;
  const selected = await activeSelection(tab.id);
  if (!selected) return;
  const text = info.menuItemId === "vifastkey-remove-diacritics"
    ? selected.normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/đ/g, "d").replace(/Đ/g, "D")
    : await normalize(selected, "telex", "old");
  if (text && text !== selected) {
    await replaceSelection(tab.id, text);
    setBadge(tab.id, "✓");
  }
});

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  (async () => {
    try {
      if (message?.type === "normalize") {
        sendResponse({ ok: true, text: await normalize(message.text ?? "", message.method, message.tone) });
      } else if (message?.type === "get-selection") {
        const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
        sendResponse({ ok: true, text: tab?.id ? await activeSelection(tab.id) : "" });
      } else if (message?.type === "insert") {
        const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
        if (!tab?.id) throw new Error("Không có tab đang hoạt động");
        await replaceSelection(tab.id, message.text ?? "");
        setBadge(tab.id, "✓");
        sendResponse({ ok: true });
      } else {
        sendResponse({ ok: false, error: "unknown message" });
      }
    } catch (error) {
      sendResponse({ ok: false, error: String(error) });
    }
  })();
  return true;
});
