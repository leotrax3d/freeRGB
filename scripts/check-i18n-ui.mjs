import { readFileSync, readdirSync } from "node:fs";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
const require = createRequire(new URL("../apps/desktop/package.json", import.meta.url));
const ts = require("typescript");
const root = fileURLToPath(new URL("../apps/desktop/src/", import.meta.url));
// Technical tokens and branding deliberately stay unchanged in the pseudo-locale.
const technical = new Set(["freeRGB", "50-60, 1200-1300 rpm", "C:\\temps\\gpu.txt", "Esc", "freergbd", "Ctrl K", "freergbteam@gmail.com", "RPM", "ARGB", "fps", "API", "freeRGB desktop", "sudo modprobe nct6775", "sudo modprobe it87", "packaging/linux/60-freergb.rules"]);
export function hardcodedUi(source, filename = "view.tsx") {
  const ast = ts.createSourceFile(filename, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const found = [];
  function check(text, node) {
    text = text.trim();
    if (/[a-z]{2}/i.test(text) && !technical.has(text)) found.push(`${filename}:${ast.getLineAndCharacterOfPosition(node.getStart(ast)).line + 1}: ${text}`);
  }
  function visit(node) {
    if (ts.isJsxText(node)) check(node.text, node);
    if (ts.isJsxAttribute(node) && ["title", "placeholder", "aria-label", "alt"].includes(node.name.getText(ast)) && node.initializer) {
      if (ts.isStringLiteral(node.initializer)) check(node.initializer.text, node);
      if (ts.isJsxExpression(node.initializer) && node.initializer.expression && ts.isStringLiteral(node.initializer.expression)) check(node.initializer.expression.text, node);
    }
    ts.forEachChild(node, visit);
  }
  visit(ast);
  return found;
}
export function scanHardcodedUi() {
  return readdirSync(root, { recursive: true }).filter((p) => p.endsWith(".tsx") && !p.includes(".test.")).flatMap((p) => hardcodedUi(readFileSync(`${root}/${p}`, "utf8"), p));
}
