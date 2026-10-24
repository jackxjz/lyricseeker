// 验证「含翻译」选项：勾上后复制，译文应出现在剪贴板文本里。
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

// 打开「选项」面板（面板类名是 .options-panel，别重复点击导致又关掉）
console.log("选项按钮:", await ev(`
  (() => {
    const btn = [...document.querySelectorAll('button')].find(b => b.innerText.trim() === '选项');
    if (!btn) return 'missing';
    if (!document.querySelector('.options-panel')) btn.click();
    return document.querySelector('.options-panel') ? 'panel-open' : 'panel-missing';
  })()`));
await sleep(600);

// 勾上「含翻译」
console.log("含翻译:", await ev(`
  (() => {
    const labels = [...document.querySelectorAll('label')];
    const target = labels.find(l => l.innerText.includes('含翻译'));
    if (!target) return 'missing label';
    const box = target.querySelector('input[type=checkbox]');
    if (!box) return 'missing box';
    if (!box.checked) box.click();
    return 'checked=' + box.checked;
  })()`));
await sleep(400);

console.log("Ctrl+1:", await ev(`
  (() => {
    window.dispatchEvent(new KeyboardEvent('keydown', { key: '1', code: 'Digit1', keyCode: 49, ctrlKey: true, bubbles: true }));
    return 'ok';
  })()`));
await sleep(1800);
console.log("toast:", await ev(`(document.querySelector('.toast') || {}).innerText || 'none'`));

ws.close();
