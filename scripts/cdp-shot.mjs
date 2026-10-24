// 用 CDP 的 Page.captureScreenshot 直接抓渲染结果：
// 比 CopyFromScreen 可靠 —— 不受窗口层级/遮挡/DPI 影响。
// 用法：node scripts/cdp-shot.mjs <输出路径> [是否先打开设置页 "settings"]
const PORT = 9333;

const out = process.argv[2] ?? "docs/screenshot.png";
const mode = process.argv[3] ?? "";

const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
const target = list.find((t) => t.type === "page" && t.url.includes("tauri.localhost"));
if (!target) {
  console.error("找不到页面目标，应用是否以 --remote-debugging-port=9333 启动？");
  process.exit(1);
}

const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = reject;
});

let seq = 0;
const pending = new Map();
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  const slot = pending.get(msg.id);
  if (slot) {
    pending.delete(msg.id);
    slot(msg);
  }
};

function send(method, params) {
  const id = ++seq;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    ws.send(JSON.stringify({ id, method, params }));
  });
}

async function evaluate(expression) {
  const res = await send("Runtime.evaluate", {
    expression,
    returnByValue: true,
    awaitPromise: true,
  });
  return res.result?.result?.value;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

if (mode === "settings") {
  if (!(await evaluate(`!!document.querySelector('.settings-page')`))) {
    await evaluate(`document.querySelector('.window-actions .icon-button:last-child').click()`);
    await sleep(900);
  }
} else if (!(await evaluate(`!document.querySelector('.settings-page')`))) {
  // 非设置页截图：先关掉设置页，免得遮住主界面。
  await evaluate(`document.querySelector('.settings-head .ghost-btn').click()`);
  await sleep(500);
}

// 等一帧，确保刚触发的状态已经落进渲染。
await evaluate(`new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))`);

const shot = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
const base64 = shot.result?.data;
if (!base64) {
  console.error("截图失败:", JSON.stringify(shot.error ?? shot));
  process.exit(1);
}

const { writeFileSync } = await import("node:fs");
writeFileSync(out, Buffer.from(base64, "base64"));
console.log(`saved ${out}`);
ws.close();
