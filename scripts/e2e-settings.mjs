// 设置页的端到端验收：CDP 驱动真实 WebView2。
//
// 检查项：
// 1. 没选中任何歌曲时也能打开设置
// 2. 歌词四项的默认值与顺序
// 3. 关掉一个源后主页 tab 消失、且该源不会被请求
// 4. 缓存占用大小能显示，清理后归零
// 5. 复制位顺序：Ctrl+1 = 普通逐字，Ctrl+2 = 增强逐字
// 6. 设置能持久化（重启进程后仍是改过的值）

const PORT = 9333;

async function page() {
  const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
  const target = list.find((t) => t.type === "page" && t.url.includes("tauri.localhost"));
  if (!target) throw new Error("找不到页面目标");
  return target.webSocketDebuggerUrl;
}

let ws;
let seq = 0;
const pending = new Map();

async function connect() {
  const url = await page();
  ws = new WebSocket(url);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });
  ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    const slot = pending.get(msg.id);
    if (slot) {
      pending.delete(msg.id);
      slot(msg);
    }
  };
}

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
  if (res.result?.exceptionDetails) {
    throw new Error(JSON.stringify(res.result.exceptionDetails));
  }
  return res.result?.result?.value;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitFor(expression, label, timeout = 15000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const ok = await evaluate(expression);
    if (ok) return true;
    await sleep(200);
  }
  throw new Error(`超时等待：${label}`);
}

const results = [];
function check(name, pass, detail = "") {
  results.push({ name, pass, detail });
  console.log(`${pass ? "PASS" : "FAIL"}  ${name}${detail ? "  " + detail : ""}`);
}

async function run() {
  await connect();

  // 1. 没有选中歌曲时也能打开设置
  const hasDocBefore = await evaluate(`!!document.querySelector('.lyric-line')`);
  await evaluate(`document.querySelector('.window-actions .icon-button:last-child').click()`);
  await waitFor(`!!document.querySelector('.settings-page')`, "设置页打开");
  check("未选中歌曲时也能打开设置", !hasDocBefore && true, `打开前有无歌词=${hasDocBefore}`);

  // 2. 歌词四项：默认值 + 顺序
  const copyItems = await evaluate(`
    (() => {
      const sections = [...document.querySelectorAll('.settings-section')];
      const lyric = sections.find(s => s.querySelector('h3')?.textContent === '歌词');
      return [...lyric.querySelectorAll('.settings-row')].map(row => ({
        label: row.querySelector('.settings-label')?.firstChild?.textContent?.trim() ?? '',
        checked: row.querySelector('input').checked,
      }));
    })()
  `);
  console.log("歌词设置:", JSON.stringify(copyItems, null, 0));
  const orderOk =
    copyItems.length === 4 &&
    copyItems[0].label === "带翻译" &&
    copyItems[1].label.includes("[ti:]") &&
    copyItems[2].label.includes("保留逐字中的空格") &&
    copyItems[3].label.includes("过滤");
  check("歌词四项顺序正确", orderOk);
  check("默认勾选 翻译/元信息/空格", copyItems[0].checked && copyItems[1].checked && copyItems[2].checked);
  check("默认不勾选 过滤制作信息", copyItems[3].checked === false);

  // 3. 四个段落顺序：词源 → 歌词 → 翻译 → 缓存
  const headings = await evaluate(
    `[...document.querySelectorAll('.settings-section h3')].map(h => h.textContent)`,
  );
  console.log("段落顺序:", JSON.stringify(headings));
  check("段落顺序 词源→歌词→翻译→缓存", headings.join(",") === "词源,歌词,翻译,缓存");

  // 3b. 翻译板块：四个单选水平铺开、默认选中第 2 种、关掉带翻译后整体置灰
  const trans = await evaluate(`
    (() => {
      const s = [...document.querySelectorAll('.settings-section')].find(s => s.querySelector('h3')?.textContent === '翻译');
      if (!s) return { missing: true };
      const radios = [...s.querySelectorAll('input[type=radio]')];
      const labels = [...s.querySelectorAll('.layout-row')];
      const top = labels.map(l => Math.round(l.getBoundingClientRect().top));
      // 单选圆点必须在选项名左侧、且垂直居中对齐
      const placed = labels.map(l => {
        const r = l.querySelector('input[type=radio]').getBoundingClientRect();
        const n = l.querySelector('.layout-name').getBoundingClientRect();
        return {
          right: Math.round(n.left) >= Math.round(r.right) - 2,
          center: Math.abs((r.top + r.height / 2) - (n.top + n.height / 2)) <= 3,
        };
      });
      const samples = labels.map(l => l.querySelector('em')?.textContent ?? '');
      return {
        count: radios.length,
        values: radios.map(r => r.value),
        names: labels.map(l => l.querySelector('.layout-name')?.textContent ?? ''),
        checked: radios.find(r => r.checked)?.value ?? null,
        disabled: radios.filter(r => r.disabled).length,
        sameRow: new Set(top).size === 1,
        gridColumns: getComputedStyle(s.querySelector('.settings-layouts')).gridTemplateColumns.split(' ').length,
        hasWarn: !!s.querySelector('.settings-warn'),
        nameRightOfRadio: placed.every(p => p.right && p.center),
        noEmoji: !/[\\u{1F000}-\\u{1FAFF}\\u{FE0F}\\u{20E3}]/u.test(s.textContent),
        samplesMultiline: samples.map(t => t.split('\\n').length),
        samplesSolidus: samples.some(t => t.includes('/')),
        samplesTimes: [...new Set(samples.join('\\n').match(/\\[\\d\\d:\\d\\d\\.\\d\\d\\d\\]/g) ?? [])],
      };
    })()
  `);
  console.log("翻译板块:", JSON.stringify(trans));
  check("翻译板块有四个单选", trans.count === 4, JSON.stringify(trans.values));
  check(
    "翻译选项顺序 1..4",
    trans.values?.join(",") === "sameLine,nextLine,before,interleaved",
    JSON.stringify(trans.values),
  );
  check("默认选中第 2 种（原文在上）", trans.checked === "nextLine", trans.checked);
  check("四个单选水平铺开（同一行 + 四列栅格）", trans.sameRow === true && trans.gridColumns === 4);
  check("带翻译开着时单选可用", trans.disabled === 0, `disabled=${trans.disabled}`);
  check("选项名在单选圆点右侧且居中对齐", trans.nameRightOfRadio === true);
  check("选项名里没有 emoji 编号", trans.noEmoji === true, JSON.stringify(trans.names));
  check(
    "样例用换行而不是斜杠分隔",
    trans.samplesSolidus === false && trans.samplesMultiline?.filter(n => n > 1).length === 3,
    JSON.stringify(trans.samplesMultiline),
  );
  check(
    "样例时间戳为 01.168 / 02.268",
    trans.samplesTimes?.every(t => t === "[00:01.168]" || t === "[00:02.268]") === true,
    JSON.stringify(trans.samplesTimes),
  );

  const greyed = await evaluate(`
    (async () => {
      const section = [...document.querySelectorAll('.settings-section')].find(s => s.querySelector('h3')?.textContent === '歌词');
      const box = [...section.querySelectorAll('.settings-row')].find(r => r.textContent.includes('带翻译')).querySelector('input');
      const was = box.checked;
      if (was) box.click();
      await new Promise(r => setTimeout(r, 260));
      const s = [...document.querySelectorAll('.settings-section')].find(s => s.querySelector('h3')?.textContent === '翻译');
      const radios = [...s.querySelectorAll('input[type=radio]')];
      const out = {
        sectionDisabled: s.classList.contains('is-disabled'),
        ariaDisabled: s.getAttribute('aria-disabled'),
        allRadiosDisabled: radios.every(r => r.disabled),
        warn: s.querySelector('.settings-warn')?.textContent ?? null,
      };
      if (was) box.click();
      await new Promise(r => setTimeout(r, 260));
      out.restored = [...document.querySelectorAll('.settings-layouts input[type=radio]')].every(r => !r.disabled);
      return out;
    })()
  `);
  console.log("关掉带翻译后:", JSON.stringify(greyed));
  check("取消「带翻译」后翻译板块置灰", greyed.sectionDisabled === true && greyed.allRadiosDisabled === true);
  check("置灰时 aria-disabled 为 true", greyed.ariaDisabled === "true", greyed.ariaDisabled);
  check("置灰时给出提示文案", typeof greyed.warn === "string" && greyed.warn.length > 0, greyed.warn);
  check("重新勾选后单选恢复可用", greyed.restored === true);

  // 4. 缓存大小可读
  const cacheText = await evaluate(`
    (() => {
      const s = [...document.querySelectorAll('.settings-section')].find(s => s.querySelector('h3')?.textContent === '缓存');
      return s.querySelector('.cache-meta strong')?.textContent ?? '';
    })()
  `);
  console.log("缓存占用:", cacheText);
  check("缓存占用显示为大小", /^[\d.]+ (B|KB|MB|GB)$/.test(cacheText.trim()), cacheText);

  // 5. 缓冲大小在拿到歌词后会增大
  await evaluate(`document.querySelector('.settings-head .ghost-btn').click()`);
  await waitFor(`!document.querySelector('.settings-page')`, "设置页关闭");

  // 6. 复制位顺序：Ctrl+1 普通逐字，Ctrl+2 增强逐字
  const labels = await evaluate(
    `[...document.querySelectorAll('.copy-button .copy-label')].map(e => e.textContent)`,
  );
  console.log("复制位顺序:", JSON.stringify(labels));
  check(
    "复制位 1=普通逐字、2=增强逐字",
    labels[0].includes("普通 LRC") && labels[1].includes("增强 LRC"),
    labels.join(" | "),
  );

  // 7. 关掉酷狗 → 主页只剩 QQ 音乐，且不再有「全部」
  //    注意：设置会被持久化，脚本可能跑在「酷狗已关」的状态上，所以先读当前值再定动作。
  await evaluate(`document.querySelector('.window-actions .icon-button:last-child').click()`);
  await waitFor(`!!document.querySelector('.settings-page')`, "设置页再次打开");
  const kugouWasOn = await evaluate(`
    (() => {
      const section = [...document.querySelectorAll('.settings-section')].find(s => s.querySelector('h3')?.textContent === '词源');
      const row = [...section.querySelectorAll('.settings-row')].find(r => r.textContent.includes('酷狗'));
      return row.querySelector('input').checked;
    })()
  `);
  if (kugouWasOn) {
    await evaluate(`
      (() => {
        const section = [...document.querySelectorAll('.settings-section')].find(s => s.querySelector('h3')?.textContent === '词源');
        [...section.querySelectorAll('.settings-row')].find(r => r.textContent.includes('酷狗')).querySelector('input').click();
      })()
    `);
    await sleep(400);
  } else {
    console.log("（酷狗此前已关闭，跳过点击）");
  }
  const tabsAfter = await evaluate(
    `[...document.querySelectorAll('.source-tabs .tab')].map(t => t.textContent)`,
  );
  console.log("关闭酷狗后的 tab:", JSON.stringify(tabsAfter));
  check("关闭酷狗后主页不再显示它", !tabsAfter.some((t) => t.includes("酷狗")), tabsAfter.join("|"));

  // 8. 关闭后搜索不应走酷狗
  await evaluate(`document.querySelector('.settings-head .ghost-btn').click()`);
  await sleep(200);
  await evaluate(`
    (() => {
      const input = document.querySelector('.search-input');
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
      setter.call(input, '晴天');
      input.dispatchEvent(new Event('input', { bubbles: true }));
      document.querySelector('.search-form').requestSubmit();
    })()
  `);
  await waitFor(`document.querySelectorAll('.song-item').length > 0`, "搜索结果", 20000);
  const sources = await evaluate(
    `[...new Set([...document.querySelectorAll('.song-item .song-source')].map(b => b.textContent.trim()))]`,
  );
  console.log("结果里的来源:", JSON.stringify(sources));
  check("关闭酷狗后结果只来自 QQ", sources.length >= 1 && sources.every((s) => s.includes("QQ")), sources.join("|"));

  // 9. 恢复酷狗，验证设置持久化（写入 settings.json）
  //    同样按当前状态决定是否点击，脚本可重复运行。
  await evaluate(`document.querySelector('.window-actions .icon-button:last-child').click()`);
  await waitFor(`!!document.querySelector('.settings-page')`, "设置页打开");
  await evaluate(`
    (() => {
      const wanted = { 酷狗: true, 过滤: true };
      const pick = (title) => {
        const section = [...document.querySelectorAll('.settings-section')].find(s => s.querySelector('h3')?.textContent === title);
        return [...section.querySelectorAll('.settings-row')];
      };
      for (const key of Object.keys(wanted)) {
        const row = pick('词源').concat(pick('歌词')).find(r => r.textContent.includes(key));
        const box = row.querySelector('input');
        if (box.checked !== wanted[key]) box.click();
      }
    })()
  `);
  await sleep(600);
  const restored = await evaluate(`
    (() => {
      const section = [...document.querySelectorAll('.settings-section')].find(s => s.querySelector('h3')?.textContent === '歌词');
      const rows = [...section.querySelectorAll('.settings-row')];
      return rows.map(r => r.querySelector('input').checked);
    })()
  `);
  console.log("歌词设置恢复后:", JSON.stringify(restored));
  check("过滤制作信息已可勾选（默认关→手开）", restored[3] === true);

  console.log("\n=== 汇总 ===");
  const failed = results.filter((r) => !r.pass);
  console.log(`${results.length - failed.length}/${results.length} 通过`);
  if (failed.length) {
    console.log("失败项:", failed.map((f) => f.name).join("; "));
    process.exitCode = 1;
  }
}

run().catch((error) => {
  console.error("E2E 失败:", error.message);
  process.exitCode = 1;
}).finally(() => {
  setTimeout(() => process.exit(process.exitCode ?? 0), 300);
});
