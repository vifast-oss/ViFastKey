import init, { normalize_text } from "./pkg/vfk_wasm.js";

const ready = init();
const $ = (id) => document.getElementById(id);
const input = $("input"), output = $("output"), method = $("method");

async function normalize() {
  await ready;
  output.value = normalize_text(input.value, method.value, "old");
}
$("normalize").addEventListener("click", normalize);

$("copy").addEventListener("click", async () => {
  if (!output.value) await normalize();
  await navigator.clipboard.writeText(output.value);
});
$("insert").addEventListener("click", async () => {
  if (!output.value) await normalize();
  const result = await chrome.runtime.sendMessage({ type: "insert", text: output.value });
  if (!result?.ok) output.value = `Lỗi: ${result?.error || "không chèn được"}`;
});

const keys = "qwertyuiopasdfghjklzxcvbnm1234567890";
// Nhóm hàng phím đơn giản; click chèn ký tự thô, sau đó bấm Chuẩn hoá.
for (const ch of keys) {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = ch;
  button.addEventListener("click", () => {
    input.value += ch;
    input.focus();
  });
  $("keyboard").append(button);
}
const space = document.createElement("button");
space.type = "button";
space.textContent = "␣";
space.addEventListener("click", () => { input.value += " "; input.focus(); });
$("keyboard").append(space);
const back = document.createElement("button");
back.type = "button";
back.textContent = "⌫";
back.addEventListener("click", () => { input.value = Array.from(input.value).slice(0, -1).join(""); input.focus(); });
$("keyboard").append(back);

chrome.runtime.sendMessage({ type: "get-selection" }).then((result) => {
  if (result?.ok && result.text) input.value = result.text;
}).catch(() => {});
