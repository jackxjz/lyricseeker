// 把剪贴板内容与 LDDC 范例文件逐行比对
import { readFileSync } from "node:fs";

const refPath = process.argv[2];
const clipPath = process.argv[3];

const body = (text) =>
  text
    .split(/\r?\n/)
    .filter((line) => {
      if (!line.startsWith("[")) return line.trim() !== "";
      const second = line[1];
      return second >= "0" && second <= "9"; // 正文行 [00:...]，头部标签 [ti:...] 被过滤
    });

const want = body(readFileSync(refPath, "utf8"));
const got = body(readFileSync(clipPath, "utf8"));

console.log(`范例 ${want.length} 行，我们 ${got.length} 行`);
let diff = 0;
for (let i = 0; i < Math.max(want.length, got.length); i++) {
  if (want[i] !== got[i]) {
    if (diff < 3) {
      console.log(`行${i}:\n  范例: ${want[i] ?? "<缺>"}\n  我们: ${got[i] ?? "<缺>"}`);
    }
    diff++;
  }
}
console.log(diff === 0 ? "✅ 逐字节一致" : `❌ ${diff} 行不同`);
process.exit(diff === 0 ? 0 : 1);
