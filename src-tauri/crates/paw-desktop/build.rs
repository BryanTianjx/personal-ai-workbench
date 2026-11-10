// Tauri build script: validates tauri.conf.json and generates the config +
// capability code the WebView shell embeds. The frontendDist resources are
// produced by `pnpm build` (vite) into the repo-root `dist/` before this
// crate compiles; both CI and the Windows evidence build follow that order.

fn main() {
    tauri_build::build()
}
