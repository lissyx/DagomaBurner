// Builds or serves the frontend, keeping npm dependencies out of the source
// tree: they are installed from package-lock.json into target/frontend/.
//
//   node scripts/frontend.mjs build   production build into target/frontend/dist
//   node scripts/frontend.mjs dev     Vite dev server, for `cargo tauri dev`
//
// It is run with the Node.js that tools/frontend installs into target/node/:
// by src-tauri/build.rs for `cargo build`, and through `cargo run -p
// dagomapp-frontend` for `cargo tauri dev`.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const frontendDir = join(root, "target", "frontend");
const nodeModules = join(frontendDir, "node_modules");

// The npm that ships with this Node.js, rather than whichever is on the PATH.
const npmCli = join(dirname(process.execPath), process.platform == "win32" ? "" : "../lib", "node_modules/npm/bin/npm-cli.js");

function run(command, args, options) {
  const result = spawnSync(command, args, { stdio: "inherit", ...options });
  if (result.error)
    throw new Error(`can't run ${command}: ${result.error.message}`);
  if (result.status != 0)
    process.exit(result.status ?? 1);
}

/** Runs `npm ci` in target/frontend/ unless package-lock.json is unchanged since the last time. */
function installDependencies() {
  const lock = readFileSync(join(root, "package-lock.json"));
  const hash = createHash("sha256").update(lock).digest("hex");
  const stamp = join(frontendDir, ".installed");
  if (existsSync(nodeModules) && existsSync(stamp) && readFileSync(stamp, "utf8") == hash)
    return;

  mkdirSync(frontendDir, { recursive: true });
  for (const file of ["package.json", "package-lock.json"])
    copyFileSync(join(root, file), join(frontendDir, file));
  if (existsSync(npmCli))
    run(process.execPath, [npmCli, "ci", "--no-audit", "--no-fund"], { cwd: frontendDir });
  else // A Node.js given with DAGOMAPP_NODE may have npm elsewhere.
    run("npm", ["ci", "--no-audit", "--no-fund"], { cwd: frontendDir, shell: process.platform == "win32" });
  writeFileSync(stamp, hash);
}

function vite(args) {
  run(process.execPath, [join(nodeModules, "vite", "bin", "vite.js"), ...args, "--config", join(root, "vite.config.js")], { cwd: root });
}

const command = process.argv[2];
installDependencies();
switch (command) {
  case "build":
    vite(["build"]);
    break;
  case "dev":
    vite([]);
    break;
  default:
    console.error("usage: node scripts/frontend.mjs build|dev");
    process.exit(2);
}
