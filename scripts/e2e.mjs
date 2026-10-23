// 通过 WebView2 的 CDP 端口驱动 LyricSeeker 做端到端验收。
// 前提: 以 WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333 启动 exe。
// 用法: node scripts/e2e.mjs [关键词]
const CDP = "http://127.0.0.1:9333";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function pickPage() {
  const list = await (await fetch(`${CDP}/json/list`)).json();
  const page = list.find((t) => t.type === "page");
  if (!page) throw new Error("no page target");
  return page.webSocketDebuggerUrl;
}

class Cdp {
  constructor(url) {
    this.ws = new WebSocket(url);
    this.id = 0;
    this.pending = new Map();
    this.ready = new Promise((res, rej) => {
      this.ws.addEventListener("open", () => res());
      this.ws.addEventListener("error", () => rej(new Error("ws error")));
    });
    this.ws.addEventListener("message", (ev) => {
      const msg = JSON.parse(ev.data);
      const slot = this.pending.get(msg.id);
      if (!slot) return;
      this.pending.delete(msg.id);
      msg.error ? slot.reject(new Error(JSON.stringify(msg.error))) : slot.resolve(msg.result);
    });
  }
  async send(method, params = {}) {
    await this.ready;
    const id = ++this.id;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
      setTimeout(() => {
        if (this.pending.delete(id)) reject(new Error(`timeout: ${method}`));
      }, 60000);
    });
  }
  async eval(expression) {
    const r = await this.send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    if (r.exceptionDetails) throw new Error("eval: " + JSON.stringify(r.exceptionDetails).slice(0, 300));
    return r.result.value;
  }
  close() { this.ws.close(); }
}

async function waitFor(cdp, expr, label, tries = 40, gap = 500) {
  for (let i = 0; i < tries; i++) {
    const v = await cdp.eval(expr);
    if (v) return v;
    await sleep(gap);
  }
  throw new Error(`waitFor timeout: ${label}`);
}

async function main() {
  const keyword = process.argv[2] || "晴天";
  const cdp = new Cdp(await pickPage());
  await cdp.send("Runtime.enable");
  console.log("title:", await cdp.eval("document.title"));

  // 填词
  console.log("input:", await cdp.eval(`
    (() => {
      const el = document.querySelector('.search-input');
      if (!el) return 'no-input';
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
      setter.call(el, ${JSON.stringify(keyword)});
      el.dispatchEvent(new Event('input', { bubbles: true }));
      return el.value;
    })()`));

  // 提交表单
  console.log("submit:", await cdp.eval(`
    (() => {
      const f = document.querySelector('.search-form');
      if (!f) return 'no-form';
      f.requestSubmit();
      return 'ok';
    })()`));

  const rows = await waitFor(cdp, "document.querySelectorAll('.song-item').length", "song rows");
  console.log("song rows:", rows);
  console.log("first 3:", await cdp.eval(`
    [...document.querySelectorAll('.song-item')].slice(0,3).map(e => e.innerText.replace(/\\n/g,' | '))`));

  // 选中第一首
  console.log("click:", await cdp.eval(`
    (() => { const e = document.querySelector('.song-item'); e.click(); return 'clicked'; })()`));

  const lines = await waitFor(cdp, "document.querySelectorAll('.lyric-line').length", "lyric lines");
  console.log("lyric lines:", lines);
  console.log("syllable spans:", await cdp.eval("document.querySelectorAll('.syl').length"));
  console.log("badges:", await cdp.eval(`[...document.querySelectorAll('.badge')].map(e=>e.innerText).join(' / ')`));

  console.log("---- 前 5 行预览 ----");
  console.log(await cdp.eval(`
    [...document.querySelectorAll('.lyric-line')].slice(0,5).map(e => e.innerText.replace(/\\n/g,' ')).join('\\n')`));

  // 逐字增强 LRC (Ctrl+1)
  console.log("---- Ctrl+1 复制 ----");
  console.log(await cdp.eval(`
    (() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: '1', code: 'Digit1', keyCode: 49, ctrlKey: true, bubbles: true }));
      return 'dispatched';
    })()`));
  await sleep(1500);
  console.log("toast:", await cdp.eval(`(document.querySelector('.toast')||{}).innerText || 'none'`));
  console.log("status:", await cdp.eval(`(document.querySelector('.status-bar')||{}).innerText || 'none'`));

  cdp.close();
}

main().catch((e) => { console.error("E2E ERROR:", e.message); process.exit(1); });
