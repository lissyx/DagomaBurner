import { join, resolve } from "node:path";

// npm dependencies live in target/frontend/ (see scripts/frontend.mjs), out of
// the sources' directory tree, so this config can't import "vite" itself.
const frontendDir = resolve(import.meta.dirname, "target/frontend");
const host = process.env.TAURI_DEV_HOST;

/** Resolves bare imports from target/frontend/node_modules. */
function targetNodeModules() {
  // Any file in target/frontend/ makes the resolver look in its node_modules.
  const importer = join(frontendDir, "index.js");
  return {
    name: "target-node-modules",
    enforce: "pre",
    resolveId(source, from, options) {
      if (/^[./\0]/.test(source) || source.includes(":") || from?.startsWith(frontendDir))
        return null;
      return this.resolve(source, importer, { ...options, skipSelf: true });
    },
  };
}

export default {
  root: "src",
  // Vendored scripts, images, fonts and the Animate iframes are served as-is from /static.
  publicDir: "public",
  cacheDir: join(frontendDir, "vite"),
  plugins: [targetNodeModules()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**", "**/target/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    outDir: join(frontendDir, "dist"),
    emptyOutDir: true,
    target: process.env.TAURI_ENV_PLATFORM == "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    rolldownOptions: {
      input: {
        main: resolve(import.meta.dirname, "src/index.html"),
      },
    },
  },
};
