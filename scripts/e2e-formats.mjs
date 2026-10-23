// 验证四种复制格式：Ctrl+1/2/3/4 依次触发，每次读回剪贴板前 N 行
const CDP = "http://127.0.0.1:9333";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const list = await (await fetch(`${CDP}/json/list`)).json();
const page = list.find((t) => t.type === "page");
const ws = new WebSocket(page.webSocketDebuggerUrl);
let id = 0;
const pending = new Map();
await new Promise((r) => ws.addEventListener("open", r));
ws.addEventListener("message", (e) => {
  const m = JSON.parse(e.data);
  const s = pending.get(m.id);
  if (!s) return;
  pending.delete(m.id);
  m.error ? s.reject(new Error(JSON.stringify(m.error))) : s.resolve(m.result);
});
const send = (method, params = {}) =>
  new Promise((res, rej) => {
    const i = ++id;
    pending.set(i, { resolve: res, reject: rej });
    ws.send(JSON.stringify({ id: i, method, params }));
    setTimeout(() => { if (pending.delete(i)) rej(new Error("timeout " + method)); }, 60000);
  });
const ev = async (expr) => {
  const r = await send("Runtime.evaluate", { expression: expr, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 400));
  return r.result.value;
};
await send("Runtime.enable");

for (const [key, code, kc, label] of [
  ["1", "Digit1", 49, "逐字增强 LRC"],
  ["2", "Digit2", 50, "逐字原始记号"],
  ["3", "Digit3", 51, "逐行带时间戳"],
  ["4", "Digit4", 52, "纯文本"],
]) {
  await ev(`
    (() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: '${key}', code: '${code}', keyCode: ${kc}, ctrlKey: true, bubbles: true }));
      return 1;
    })()`);
  await sleep(1400);
  const toast = await ev(`(document.querySelector('.toast') || {}).innerText || 'none'`);
  console.log(`Ctrl+${key} [${label}] -> ${toast}`);
}
ws.close();
