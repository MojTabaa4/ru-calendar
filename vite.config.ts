import { execFile } from "node:child_process";
import { fileURLToPath } from "node:url";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig, type Plugin } from "vite";

const root = fileURLToPath(new URL(".", import.meta.url));
const host = process.env.TAURI_DEV_HOST;

// Outside Tauri (a plain browser on the dev server), src/lib/api.ts posts commands
// to /__ruz/<command>. This dev-only plugin answers them with the `ruz` CLI's
// hidden `json` command, so the UI can be worked on without the tray app.
function ruzDevBridge(): Plugin {
  return {
    name: "ruz-dev-bridge",
    apply: "serve",
    configureServer(server) {
      server.middlewares.use("/__ruz/", (req, res) => {
        const command = (req.url ?? "").replace(/^\//, "").split("?")[0];
        let body = "";
        req.on("data", (chunk) => (body += chunk));
        req.on("end", () => {
          const args = ["run", "-q", "-p", "ruz-cli", "--", "json", command, body || "{}"];
          execFile("cargo", args, { cwd: root }, (error, stdout, stderr) => {
            res.setHeader("content-type", "application/json");
            if (error) {
              res.statusCode = 500;
              res.end(JSON.stringify(stderr || String(error)));
              return;
            }
            res.end(stdout);
          });
        });
      });
    },
  };
}

export default defineConfig({
  plugins: [svelte(), ruzDevBridge()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**"] },
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    rollupOptions: {
      input: {
        popover: fileURLToPath(new URL("popover.html", import.meta.url)),
        settings: fileURLToPath(new URL("settings.html", import.meta.url)),
        converter: fileURLToPath(new URL("converter.html", import.meta.url)),
      },
    },
  },
});
