// 译文验收：两个来源各搜一首有译文的歌，统计预览里贴上的译文行数。
// 用法：先带 CDP 启动应用，再 node scripts/e2e-translation.mjs
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

async function pickTab(label) {
  return ev(`
    (() => {
      const tab = [...document.querySelectorAll('.tab')].find(t => t.innerText.includes(${JSON.stringify(label)}));
      if (!tab) return 'missing';
      tab.click();
      return tab.innerText;
    })()`);
}

async function search(keyword) {
  await ev(`
    (() => {
      const el = document.querySelector('.search-input');
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
      setter.call(el, ${JSON.stringify(keyword)});
      el.dispatchEvent(new Event('input', { bubbles: true }));
      document.querySelector('.search-form').requestSubmit();
      return 'ok';
    })()`);

  for (let i = 0; i < 40; i++) {
    await sleep(500);
    if (await ev(`document.querySelectorAll('.song-item').length`)) break;
  }
  return ev(`document.querySelectorAll('.song-item').length`);
}

async function openFirstMatch(title) {
  await ev(`
    (() => {
      const item = [...document.querySelectorAll('.song-item')]
        .find(e => e.innerText.includes(${JSON.stringify(title)}));
      (item || document.querySelector('.song-item')).click();
      return 'ok';
    })()`);

  for (let i = 0; i < 60; i++) {
    await sleep(500);
    if (await ev(`document.querySelectorAll('.lyric-line').length`)) break;
  }
}

async function measure(label, keyword, title) {
  await pickTab(label);
  await sleep(700);
  const rows = await search(keyword);
  await openFirstMatch(title);

  const stats = await ev(`
    (() => {
      const lines = [...document.querySelectorAll('.lyric-line')];
      const trans = lines.filter(e => e.querySelector('.line-translation')).length;
      const sample = lines
        .filter(e => e.querySelector('.line-translation'))
        .slice(2, 6)
        .map(e => e.innerText.split(String.fromCharCode(10)).join(' / '));
      return { lines: lines.length, trans, sample };
    })()`);

  console.log(
    `${label} 「${keyword}」：候选 ${rows} 条，预览 ${stats.lines} 行，其中 ${stats.trans} 行有译文（${Math.round((stats.trans / Math.max(stats.lines, 1)) * 100)}%）`,
  );
  for (const line of stats.sample) console.log("   ", line);
  return stats;
}

const qq = await measure("QQ", "Shape of You", "Shape of You");
const kugou = await measure("酷狗", "Shape of You", "Shape of You");

console.log("\n结论：QQ", qq.trans, "/", qq.lines, "；酷狗", kugou.trans, "/", kugou.lines);
ws.close();
if (qq.trans / Math.max(qq.lines, 1) < 0.8 || kugou.trans / Math.max(kugou.lines, 1) < 0.8) {
  console.error("译文覆盖率不足 80%");
  process.exit(1);
}
