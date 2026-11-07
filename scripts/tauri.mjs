import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';

const env = { ...process.env };
// A terminal inherited from the VS Code Snap can point host GTK at the Snap's
// incompatible schemas and libraries. Keep the desktop session and display.
if (process.platform === 'linux' && env.SNAP_NAME === 'code') {
  for (const key of Object.keys(env)) {
    if (key === 'SNAP' || key.startsWith('SNAP_')) delete env[key];
  }
  for (const key of ['GSETTINGS_SCHEMA_DIR', 'GTK_EXE_PREFIX', 'GTK_PATH', 'GTK_IM_MODULE_FILE', 'GTK_MODULES', 'GDK_PIXBUF_MODULE_FILE', 'GDK_PIXBUF_MODULEDIR', 'GIO_MODULE_DIR', 'GIO_LAUNCHED_DESKTOP_FILE', 'GIO_LAUNCHED_DESKTOP_FILE_PID']) delete env[key];
  for (const key of ['XDG_DATA_DIRS', 'XDG_CONFIG_DIRS']) {
    const original = env[`${key}_VSCODE_SNAP_ORIG`];
    if (original !== undefined) env[key] = original;
    else delete env[key];
  }
}
const require = createRequire(import.meta.url);
const cli = require.resolve('@tauri-apps/cli/tauri.js');
const child = spawn(process.execPath, [cli, ...process.argv.slice(2)], { env, stdio: 'inherit' });
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal));
child.on('error', error => { console.error(`Could not start Tauri: ${error.message}`); process.exitCode = 1; });
child.on('exit', (code, signal) => { process.exitCode = code ?? (signal ? 1 : 0); });
