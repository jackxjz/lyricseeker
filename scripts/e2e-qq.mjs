// QQ 来源验收：切 tab → 搜索 → 选中 → 读预览 → Ctrl+1 复制
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
const FLAT = "[...document.querySelectorAll(SEL)].map(e => e.innerText.split(String.fromCharCode(10)).join(' | '))";

await send("Runtime.enable");

const tab = await ev(`
  (() => {
    const tabs = [...document.querySelectorAll('.tab')];
    const qq = tabs.find(t => t.innerText.includes('QQ'));
    if (!qq) return 'no-qq-tab';
    qq.click();
    return 'clicked: ' + qq.innerText;
  })()`);
console.log("tab:", tab);
await sleep(700);

console.log("input:", await ev(`
  (() => {
    const el = document.querySelector('.search-input');
    if (!el) return 'no-input';
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    setter.call(el, '晴天');
    el.dispatchEvent(new Event('input', { bubbles: true }));
    return el.value;
  })()`));
console.log("submit:", await ev(`(() => { document.querySelector('.search-form').requestSubmit(); return 'ok'; })()`));

for (let i = 0; i < 40; i++) {
  await sleep(500);
  if (await ev(`document.querySelectorAll('.song-item').length`)) break;
}
console.log("rows:", await ev(`document.querySelectorAll('.song-item').length`));
console.log("first5:", await ev(FLAT.replace("SEL", "'.song-item'").replace("slice(0,3)", "slice(0,5)").replace("'.song-item')]", "'.song-item')].slice(0,5)")));

console.log("click:", await ev(`
  (() => {
    const e = document.querySelector('.song-item');
    if (!e) return 'none';
    e.click();
    return e.innerText.split(String.fromCharCode(10)).join(' | ');
  })()`));

for (let i = 0; i < 40; i++) {
  await sleep(500);
  if (await ev(`document.querySelectorAll('.lyric-line').length`)) break;
}
console.log("lines:", await ev(`document.querySelectorAll('.lyric-line').length`));
console.log("syl:", await ev(`document.querySelectorAll('.syl').length`));
console.log("badges:", await ev(`[...document.querySelectorAll('.badge')].map(e => e.innerText).join(' / ')`));
console.log("preview:", await ev(`
  [...document.querySelectorAll('.lyric-line')].slice(0,4)
    .map(e => e.innerText.split(String.fromCharCode(10)).join(' ')).join(' ;; ')`));

console.log("ctrl1:", await ev(`
  (() => {
    window.dispatchEvent(new KeyboardEvent('keydown', { key: '1', code: 'Digit1', keyCode: 49, ctrlKey: true, bubbles: true }));
    return 'dispatched';
  })()`));
await sleep(1500);
console.log("toast:", await ev(`(document.querySelector('.toast') || {}).innerText || 'none'`));

ws.close();
