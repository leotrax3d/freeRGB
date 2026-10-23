import { execSync } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

const root = fileURLToPath(new URL("../", import.meta.url));
for (const [directory, command] of [
  ["apps/desktop", "npm run build:demo"],
  ["website", "npm run build"],
  ["apps/admin", "npm run build"],
]) execSync(command, { cwd: join(root, directory), stdio: "inherit" });

const demo = join(root, "apps/desktop/dist-demo");
// Fail closed if a future import pulls native IPC or a network client into the demo.
function audit(directory) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) audit(path);
    else if (entry.name.endsWith(".js")) {
      const source = readFileSync(path, "utf8");
      if (/__TAURI_INTERNALS__|\bfetch\s*\(|\bWebSocket\b|\bXMLHttpRequest\b/.test(source)) {
        throw new Error(`Public demo contains a forbidden native/network API: ${entry.name}`);
      }
      if (/\b(?:Razer|Corsair|iCUE|Basilisk|Chroma|K70)\b/i.test(source)) {
        throw new Error(`Public demo contains a real hardware brand/model: ${entry.name}`);
      }
    }
  }
}
audit(demo);
const output = join(root, "website/dist");
mkdirSync(join(output, "demo"), { recursive: true });
cpSync(demo, join(output, "demo"), { recursive: true });
mkdirSync(join(output,"admin"),{recursive:true});
cpSync(join(root,"apps/admin/dist"),join(output,"admin"),{recursive:true});
writeFileSync(join(output, ".nojekyll"), "");
writeFileSync(join(output, "demo/version.json"), JSON.stringify({
  revision: process.env.GITHUB_SHA || "local",
  builtAt: new Date().toISOString(),
}) + "\n");
console.log("Website and shared desktop demo assembled in website/dist");
