// 搜索 → 选中 → 读回歌词行与滚动区，验证格式
const CDP = "http://127.0.0.1:9333";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const [source, keyword] = [process.argv[2] ?? "qq", process.argv[3] ?? "晴天"];

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
    setTimeout(() => { if (pending.delete(i)) rej(new Error("timeout " + method)); }, 90000);
  });
const ev = async (expr) => {
  const r = await send("Runtime.evaluate", { expression: expr, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 400));
  return r.result.value;
};
await send("Runtime.enable");

// 切来源 tab
await ev(`(() => {
  const tab = [...document.querySelectorAll('.tab')].find(t => t.innerText.includes('${source === "qq" ? "QQ" : "酷狗"}'));
  if (tab) tab.click();
  return !!tab;
})()`);
await sleep(400);

// 填关键词并提交
await ev(`(() => {
  const input = document.querySelector('.search-input');
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
  setter.call(input, '${keyword}');
  input.dispatchEvent(new Event('input', { bubbles: true }));
  document.querySelector('.search-form').requestSubmit();
  return 1;
})()`);

// 轮询等结果（QQ 慢时约 3s）。注意：不能只看 rows>0 ——
// 上一次搜索的列表还在 DOM 里，必须等到首条真的换成新关键词才算新结果。
const firstText = () =>
  ev(`(document.querySelector('.song-item') || {}).innerText || ''`);
let rows = 0;
for (let i = 0; i < 25; i++) {
  await sleep(600);
  rows = await ev(`document.querySelectorAll('.song-item').length`);
  const text = await firstText();
  if (rows > 0 && text.includes(keyword.split(" ")[0])) break;
}
console.log(`搜索「${keyword}」(${source}) -> ${rows} 条`);
if (rows === 0) { ws.close(); process.exit(1); }

console.log("首 3 条候选：");
console.log(await ev(`[...document.querySelectorAll('.song-item')].slice(0,3).map(e => e.innerText.replace(/\\n/g,' | ')).join('\\n')`));

// 选第一条。同样不能只看「有歌词行」——上一次的歌词还在，
// 必须等首行文本真的变了（或状态栏刷新）才算加载完成。
const firstLyric = () =>
  ev(`(document.querySelector('.lyric-line') || {}).innerText || ''`);
const before = await firstLyric();
await ev(`(() => { document.querySelectorAll('.song-item')[0].click(); return 1; })()`);
let lines = 0;
for (let i = 0; i < 25; i++) {
  await sleep(600);
  lines = await ev(`document.querySelectorAll('.lyric-line').length`);
  if (lines > 0 && (await firstLyric()) !== before) break;
}
const status = await ev(`(document.querySelector('.statusbar') || {}).innerText || ''`);
console.log(`歌词 ${lines} 行 | 状态栏：${status.replace(/\n/g, ' ')}`);
ws.close();
