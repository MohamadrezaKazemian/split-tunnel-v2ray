# Windows 10/11 x64 manual test plan

Status: **not yet run**. Use a disposable VM or test machine, a trusted proxy profile, and an administrator who can restore networking. Record Windows version, account type, installer hash, app/engine versions, polkit is not used on Windows, and observations. UWP/Store apps and shared DNS resolvers are not reliably attributable.

## Install and UAC

- [ ] On a clean Windows 10/11 x64 machine with internet, install the NSIS artifact from the Windows workflow. Confirm WebView2 Runtime is available or the installer bootstraps it. Check that the install directory contains both the app executable and `sing-box.exe`; run `sing-box.exe version` and confirm 1.14.4.
- [ ] Start the app as a standard user. Accept the UAC prompt with administrator credentials and confirm the GUI opens. Confirm settings, pidfile and generated config are in the elevated account's `%APPDATA%` application directory. Paths containing spaces and non-ASCII characters must work. When alternate credentials are used, Windows may use the administrator account's profile.
- [ ] Close the app, start it again and **decline** UAC. Confirm no app or sing-box process starts. Windows cancels execution before the app can display its own message; record any launcher/system message.
- [ ] Record Windows Defender and SmartScreen behavior, including the unsigned installer warning. Do not disable security controls just to pass the test.

## Inclusive routing and pickers

- [ ] Record baseline public IP in a browser and another networked app. Save `ipconfig /all` and `route print` output, and note existing adapters, routes and DNS servers.
- [ ] Add a valid proxy profile. Find the browser via Installed apps, Running apps, and Browse (`.exe` and Start Menu `.lnk`); verify name and full executable path. Confirm uninstallers are absent and Store/UWP limitations are visible.
- [ ] Add only the browser. Connect and wait for Connected. Check its public IP changes to the proxy IP, including a fresh private window and browser helper processes. Check the other app still uses its normal public IP. Repeat with Chrome/Edge, Firefox and an Electron app where installed; directory matches include other executables in that directory.
- [ ] Test DNS with a controlled domain and resolver logs. Compare selected and unselected apps; record shared-resolver or encrypted-DNS leaks. The current implementation does not guarantee per-app DNS isolation.
- [ ] Disconnect normally. Confirm the log does not report forced termination and sing-box exits after CTRL+BREAK. Check `Get-NetAdapter -IncludeHidden` for the app's `SplitTunnel-<pid>-<timestamp>` adapter, compare `ipconfig /all` and `route print` with the saved baseline, and verify both apps regain normal connectivity. Record differences and any adapter warning.
- [ ] Exercise the forced-kill fallback on a disposable VM using a test engine that logs `sing-box started (` but ignores CTRL+BREAK. Disconnect and verify the warning appears after roughly five seconds. Check `Get-NetAdapter -IncludeHidden`, `ipconfig /all`, and `route print` against the baseline; restore anything that remains only after identifying it.

## Crash and recovery

- [ ] Connect using the real sing-box engine, identify its PID and app-named TUN adapter, then force-stop **that sing-box process** in Task Manager on the disposable VM. Confirm the app reports failure. Compare `Get-NetAdapter -IncludeHidden`, `ipconfig /all`, and `route print` with the baseline, and verify connectivity and any adapter warning. This tests forced termination with an actual TUN; the fallback fixture above tests the five-second UI path.
- [ ] Connect again, then terminate the GUI with Task Manager. Confirm its Job Object terminates sing-box. Check the app's `SplitTunnel-<pid>-<timestamp>` adapter with `Get-NetAdapter -IncludeHidden`, compare DNS and routes to baseline using `ipconfig /all` and `route print`, and verify connectivity. Save the output.
- [ ] Restart the app after the crash. Check that stale pidfile recovery does not kill unrelated processes and investigate any cleanup-unconfirmed message. Never kill a process based only on a PID in a stale file.
- [ ] Repeat the crash scenario, reboot, and check sing-box is absent, no stale adapter or routes remain, and DNS and internet work normally.

## Result

Record every pass/failure, logs with secrets redacted, installer and sidecar hashes, `ipconfig /all`/`route print` comparisons, and any manual recovery performed. A failed cleanup or isolation check blocks a claim that Windows is working end to end.
