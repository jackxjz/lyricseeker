// 触发指定的复制快捷键（1..4），由外部读剪贴板。
// 用法：node scripts/press-copy.mjs <1|2|3|4>
const key = process.argv[2] ?? "1";
const list = await (await fetch("http://127.0.0.1:9333/json/list")).json();
const target = list.find((t) => t.type === "page");
const ws = new WebSocket(target.webSocketDebuggerUrl);
let nextId = 1;
const pending = new Map();
ws.addEventListener("message", (event) => {
  const msg = JSON.parse(event.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg);
    pending.delete(msg.id);
  }
});
await new Promise((res, rej) => {
  ws.addEventListener("open", res, { once: true });
  ws.addEventListener("error", rej, { once: true });
});
function evaluate(expression) {
  const id = nextId++;
  return new Promise((resolve) => {
    pending.set(id, (msg) => resolve(msg.result?.result?.value));
    ws.send(
      JSON.stringify({
        id,
        method: "Runtime.evaluate",
        params: { expression, awaitPromise: true, returnByValue: true },
      }),
    );
  });
}
await evaluate(`(() => {
  if (document.querySelector('.settings-page')) {
    document.querySelector('.settings-head .ghost-btn').click();
  }
  return true;
})()`);
await new Promise((r) => setTimeout(r, 300));
await evaluate(`(() => {
  window.dispatchEvent(new KeyboardEvent('keydown', { key: '${key}', code: 'Digit${key}', keyCode: ${48 + Number(key)}, ctrlKey: true, bubbles: true }));
  return true;
})()`);
await new Promise((r) => setTimeout(r, 1200));
ws.close();
process.exit(0);
