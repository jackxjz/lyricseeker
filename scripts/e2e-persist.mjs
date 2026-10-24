// 持久化验收：改动设置 → 杀掉进程 → 重启 → 检查设置还在。
//
// 这是用户明确问的「所有的设置能否做到持久化」，必须真的重启进程验证，
// 不能只看内存里的 state。

const PORT = 9333;
const STEP = process.argv[2]; // "set" 或 "verify"

const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
const target = list.find((t) => t.type === "page" && t.url.includes("tauri.localhost"));
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

function evaluate(expression) {
  const id = ++seq;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    ws.send(
      JSON.stringify({
        id,
        method: "Runtime.evaluate",
        params: { expression, returnByValue: true, awaitPromise: true },
      }),
    );
  }).then((res) => res.result?.result?.value);
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const snapshot = `(function () {
  var sections = Array.prototype.slice.call(document.querySelectorAll('.settings-section'));
  var find = function (name) {
    return sections.filter(function (s) { return s.querySelector('h3').textContent === name; })[0];
  };
  var lyric = find('歌词');
  var rows = Array.prototype.slice.call(lyric.querySelectorAll('.settings-row'));
  return JSON.stringify({
    copy: rows.map(function (r) {
      return { label: r.querySelector('.settings-label').firstChild.textContent.trim(), on: r.querySelector('input').checked };
    }),
    tabs: Array.prototype.slice.call(document.querySelectorAll('.source-tabs .tab')).map(function (t) { return t.textContent.trim(); })
  });
})()`;

async function openSettings() {
  const open = await evaluate(`!!document.querySelector('.settings-page')`);
  if (!open) {
    await evaluate(`document.querySelectorAll('.icon-button')[1].click()`);
    await sleep(500);
  }
}

await openSettings();

if (STEP === "set") {
  // 关掉酷狗、打开「过滤制作信息」，制造一个明显的非默认状态。
  await evaluate(
    `(function(){
      var sections = Array.prototype.slice.call(document.querySelectorAll('.settings-section'));
      var find = function (n) { return sections.filter(function (s) { return s.querySelector('h3').textContent === n; })[0]; };
      var kugou = Array.prototype.slice.call(find('词源').querySelectorAll('.settings-row')).filter(function (r) { return r.textContent.indexOf('酷狗') >= 0; })[0];
      if (kugou.querySelector('input').checked) kugou.querySelector('input').click();
      var filter = Array.prototype.slice.call(find('歌词').querySelectorAll('.settings-row')).filter(function (r) { return r.textContent.indexOf('过滤') >= 0; })[0];
      if (!filter.querySelector('input').checked) filter.querySelector('input').click();
    })()`,
  );
  await sleep(800);
  console.log("已设置为:", await evaluate(snapshot));
} else {
  console.log("重启后读到:", await evaluate(snapshot));
}

ws.close();
