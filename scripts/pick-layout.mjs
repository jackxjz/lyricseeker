// 设置一种翻译摆放方式，然后触发一次「逐行」复制（Ctrl+3）。
//
// 用法：node scripts/pick-layout.mjs <sameLine|nextLine|before|interleaved>
// 剪贴板由外部读取（页面内 navigator.clipboard.readText 会因窗口失焦失败）。

const layout = process.argv[2];
if (!layout) {
  console.error("用法：node scripts/pick-layout.mjs <layout>");
  process.exit(2);
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const list = await (await fetch("http://127.0.0.1:9333/json/list")).json();
const target = list.find((t) => t.type === "page");
if (!target) {
  console.error("未找到调试页面，请确认应用启动时带 --remote-debugging-port=9333");
  process.exit(1);
}

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
    pending.set(id, (msg) => {
      if (msg.result?.exceptionDetails) {
        resolve({
          __error:
            msg.result.exceptionDetails.text +
            " " +
            (msg.result.exceptionDetails.exception?.description ?? "").slice(0, 300),
        });
      } else {
        resolve(msg.result?.result?.value);
      }
    });
    ws.send(
      JSON.stringify({
        id,
        method: "Runtime.evaluate",
        params: { expression, awaitPromise: true, returnByValue: true },
      }),
    );
  });
}

// 设置页是开关式：已经开着就不要再点，否则会关掉。
const opened = await evaluate(`(() => {
  if (document.querySelector('.settings-page')) return 'already';
  document.querySelector('.window-actions .icon-button:last-child')?.click();
  return 'clicked';
})()`);
await sleep(400);

// 「带翻译」必须开着，否则单选是置灰的。
const enabled = await evaluate(`(() => {
  const row = [...document.querySelectorAll('.settings-row')].find(r => r.innerText.includes('带翻译'));
  if (!row) return 'no-row';
  const box = row.querySelector('input');
  if (!box.checked) box.click();
  return box.checked;
})()`);
await sleep(250);

const picked = await evaluate(`(() => {
  const input = document.querySelector('.settings-layouts input[value="${layout}"]');
  if (!input) return 'missing';
  if (input.disabled) return 'disabled';
  input.click();
  return input.checked ? 'ok' : 'not-checked';
})()`);
await sleep(250);

const state = await evaluate(`(() => {
  const checked = document.querySelector('.settings-layouts input:checked');
  return checked ? checked.value : null;
})()`);

// 关掉设置页，回到主界面触发复制。
await evaluate(`document.querySelector('.settings-head .ghost-btn')?.click(); true`);
await sleep(300);
await evaluate(`(() => {
  window.dispatchEvent(new KeyboardEvent('keydown', { key: '3', code: 'Digit3', keyCode: 51, ctrlKey: true, bubbles: true }));
  return true;
})()`);
await sleep(1200);

console.log(JSON.stringify({ layout, opened, enabled, picked, checked: state }));
ws.close();
process.exit(0);
