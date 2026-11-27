# UX audit: technical feasibility probes

> *Historical report, kept as written under the working title ChipDAW. Since the rename (2026-11-27, [RENAME.md](../../../RENAME.md)) the app is Cartwave by Glintworks: projects save as `.cartwave`, the executable is `cartwave`, and environment variables are `CARTWAVE_*` (the `CHIPDAW_*` names still work).*

Date: 2026-11-25. Machine: Windows 11 Pro 10.0.26200, 2560×1440 at 100 %, 60 Hz.
Stack under test: the vizia fork pinned in the root `Cargo.toml` (rev `159bbe55…`, local
checkout `../vizia`), winit 0.30.13. No product code was changed. Each probe is its own Cargo
project (empty `[workspace]`, the root `Cargo.lock` copied in so versions match the app); they
share `probes/target/` (git-ignored). Raw logs and screenshots are in `probes/logs/`. Input was
synthetic: `tools/uxdrive.ps1` (clicks, keys, screenshots) and `tools/oledrag.ps1` (a real OLE
file drag via WinForms `DoDragDrop` + `SendInput`).

Citations: `vizia_winit/…` and `vizia_core/…` are in `../vizia/crates/`; `winit/…` is
`~/.cargo/registry/src/*/winit-0.30.13/src/platform_impl/…`; `muda/…` is
`~/.cargo/registry/src/*/muda-0.22.0/src/…`.

---

## 1. Native menus

**Answer: yes. muda 0.22 (MIT OR Apache-2.0) puts a native File/Edit menu bar on the vizia
window with no fork change. Menu clicks reach the vizia model in under 1 ms. Keyboard
accelerators are shown in the menu but do nothing unless we add a message hook, so the app's
own shortcuts keep working and nothing fires twice. Two costs: an open menu freezes all
rendering and timers (its modal loop), and on macOS the menu must take over Cmd shortcuts.**

### Evidence (probe `menu_vizia/`)

- **The HWND.** The fork's public `ModifyWindow::window()` on `EventContext` returns the root
  `Arc<winit::window::Window>` (vizia_winit/src/lib.rs:11-35). `HasWindowHandle` then gives
  the HWND (the probe adds `winit = { version = "0.30.13", features = ["rwh_06"] }` only for
  that trait). It works from the first model event, after `resumed` has created the window
  (application.rs:429-447). Nothing had to be added to the fork.
- **Integration.** `menu.init_for_hwnd(hwnd)` calls `SetMenu` plus a window subclass
  (muda/platform_impl/windows/mod.rs:292-345). The client area shrinks by the menu height
  (`WM_SIZE 800x480`), winit sends `Resized`, and vizia lays out to the new client rect. The
  probe's 4 px outline sits exactly on the client edges (logs/menu_open.png). In the probe the
  queued `.inner_size()` then resized the window back to an 800×500 client area. winit accounts
  for the menu in size calculations (`AdjustWindowRectEx(…, GetMenu(hwnd) != 0, …)`,
  winit/windows/window_state.rs:440-447), and Alt key-up enters the menu loop
  (event_loop.rs:1773-1778).
  ```
  130.2 ms Init: HWND=0xEC0874; client 800x500 (winit inner 800x500) before SetMenu
  134.0 ms WM_SIZE client 800x480
  137.1 ms WM_SIZE client 800x500
  143.1 ms layout: Pulse bounds x=0 y=24 w=800 h=452 (physical px); client rect 800x500
  ```
- **Menu event → vizia.** muda calls `MenuEvent::set_event_handler`'s closure synchronously,
  from its subclass's `WM_COMMAND`, on the UI thread (muda/platform_impl/windows/mod.rs:1020-1040,
  1098). The probe's handler holds a `Mutex<ContextProxy>` (from `EventContext::get_proxy`).
  `ContextProxy` is `Send` but not `Sync` (vizia_core/src/context/proxy.rs:13, 110). The
  handler calls `emit`, which goes through the fork's `EventLoopProxy` (application.rs:123-133).
  This needs no polling and no fork hook.
  ```
  19494.9 ms muda handler: MenuEvent id="file.save" -> ContextProxy::emit
  19495.2 ms vizia Model: ProbeEvent::Menu(file.save) received
  ```
- **Accelerators vs the app's shortcuts.** On Windows, muda accelerators only work if the
  message loop calls `TranslateAcceleratorW` (muda/lib.rs:21-23, items/menu.rs:324-345).
  winit's hook for that is `EventLoopBuilderExtWindows::with_msg_hook`, but vizia builds the
  EventLoop itself (`EventLoop::with_user_event().build()`, application.rs:160-161), so the
  hook can't be installed. Tested three ways (logs/menu_accel_*.txt):
  - `--accel none` (stock): Ctrl+S and Ctrl+Z reach vizia `KeyDown` only, so the app's
    shortcut path (crates/ui/src/lib.rs:643-670) fires **once**. The menu shows "Ctrl+S" but
    has no keyboard effect of its own.
  - `--accel hook` (a thread `WH_GETMESSAGE` hook calls `TranslateAcceleratorW` and lets the
    key through): **double fire.** Both the menu event and the vizia shortcut run:
    ```
    3523.4 ms muda handler: MenuEvent id="file.save" -> ContextProxy::emit
    3523.8 ms hook: TranslateAcceleratorW translated vk=0x53 (WM_COMMAND sent); swallow=false
    3524.3 ms vizia KeyDown KeyS ctrl=true ...
    3524.4 ms APP SHORTCUT fired via vizia KeyDown: Save
    ```
  - `--accel hook-swallow` (the translated message becomes `WM_NULL`): exactly one menu event
    and no vizia KeyDown, for Ctrl+S, Ctrl+Z, Ctrl+Shift+S and Ctrl+D. vizia still receives
    the KeyUp, and a text field with focus would lose Ctrl+Z to the menu.
- **Modal loop.** Opening a menu (Alt+F or a click) enters the Win32 modal menu loop. vizia's
  rendering and timers stop completely until the menu closes. winit documents this freeze for
  popup menus (winit/windows/event_loop.rs:1241-1244).
  ```
  17565.4 ms WM_ENTERMENULOOP (modal menu loop starts; frames so far 1044)
  19494.3 ms WM_EXITMENULOOP (frames so far 1044)
  19495.3 ms timer STALL: 1946 ms without a tick
  19495.5 ms draw STALL: 1962 ms without a frame (frame #1045)
  ```
  A quirk: the `KeyDown(KeyF)` with alt=true from Alt+F reaches vizia *after* the menu
  closes (19495.1 ms). An app shortcut on Alt+letter would fire late.
- **Theme.** `MenuTheme::Auto` drew a light menu bar on this machine (logs/menu_open.png).
  `init_for_hwnd_with_theme(hwnd, MenuTheme::Dark)` exists (muda/platform_impl/windows/mod.rs:292).
- **macOS (read from source; to test on the Mac).** `init_for_nsapp` is
  `NSApp.setMainMenu(menu)` (muda/platform_impl/macos/mod.rs:168-172). winit installs its own
  default menu (an App menu with About, Services, Hide, Quit) in `applicationDidFinishLaunching`
  (winit/macos/app_state.rs:139-142, menu.rs:12+). vizia never calls
  `with_default_menu(false)` (application.rs:160), so muda's menu must be set *after* launch.
  Setting it from the first vizia event, as the probe does, runs after `resumed` and replaces
  winit's menu. AppKit treats the first top-level submenu as the App menu, so ours must start
  with an App submenu built from muda's predefined `about`/`services`/`hide`/`quit` items
  (muda/items/predefined.rs:211-261). Cmd shortcuts: NSApp offers key equivalents to the
  main menu before `keyDown:`, and winit's view has no `performKeyEquivalent:`. A matched
  Cmd+S therefore fires the muda item, and vizia never sees the KeyDown. That means no double
  fire, but on the Mac the menu becomes the only path for those shortcuts. winit's
  `sendEvent:` override only forwards Cmd key-*up* (winit/macos/app.rs:25-38). muda's menu
  items dispatch through `MenuEvent::send` (muda/platform_impl/macos/mod.rs:855).

### Licences (cargo tree, muda's normal deps across all targets)
muda Apache-2.0 OR MIT; crossbeam-channel/-utils, keyboard-types, thiserror, windows-sys,
windows-link, png, flate2, crc32fast, bitflags, cfg-if: MIT OR Apache-2.0; dpi: Apache-2.0 AND
MIT; objc2, objc2-encode, objc2-foundation, simd-adler32: MIT; objc2-app-kit,
objc2-core-foundation: Zlib OR Apache-2.0 OR MIT; miniz_oxide: MIT OR Zlib OR Apache-2.0;
adler2: 0BSD OR MIT OR Apache-2.0. unicode-ident, (MIT OR Apache-2.0) AND Unicode-3.0, is a
build-time proc-macro dependency that the app's own `syn` already pulls in. With
`default-features = false`, gtk is not pulled in. On Linux, the gtk3 feature would add
gtk-rs (MIT).

### Recommended approach
- Native menu bar in the **standalone app only**, behind a small `menu` module in the `app`
  crate (not `ui`). Install it from the first model event via `cx.window()` + HWND (Windows)
  or `init_for_nsapp` (macOS). Map each menu id to the existing `AppEvent`s via
  `ContextProxy::emit`.
- Windows: **no accelerator hook.** Keep the shortcut text in the menu labels and let
  crates/ui/src/lib.rs handle the keys as today: one path, no double fire, text fields keep
  Ctrl+Z. Use `MenuTheme::Dark`.
- macOS: the menu owns the Cmd shortcuts. Route menu events to the same `AppEvent`s, and keep
  the KeyDown shortcut code for non-menu keys.
- Keep the in-app menu (the app already has one: `self.s.menu`, lib.rs:668) for the plugin
  build. Under baseview a plugin editor is a child of the host's window and cannot own a
  menu bar.

### Risks
- Rendering and timers freeze while a menu is open (≈2 s in the log). Playhead and meters
  stop moving. Audio is on its own thread and is not affected.
- muda's `Menu` holds a raw pointer used by the window subclass. It must outlive the window
  (the probe leaks it; the app should keep it in a model or a thread-local).
- The Alt+letter key event arrives after the menu closes (see above).
- `set_event_handler` is process-global: only one handler at a time (muda/lib.rs:227-229).

---

## 2. OS file drag-and-drop with cursor position

**Answer: no, not with the stock stack. winit 0.30.13 reports `HoveredFile` once on enter and
`DroppedFile` on drop, with no position, and sends no `CursorMoved` during the hover. The fork
also discards `HoveredFile`/`HoveredFileCancelled`. Both fixes probed here work. One is to poll
the cursor while a file hovers (smallest change, cross-platform). The other is to replace
winit's OLE `IDropTarget` with our own, whose `DragOver` reports the position (Windows only;
more control).**

### Evidence
- **winit Windows source.** `DragEnter` sends `HoveredFile(path)` once per file and ignores
  `_pt`. `DragOver` ignores `_pt` and only returns the cached effect. `DragLeave` sends
  `HoveredFileCancelled`. `Drop` sends `DroppedFile` and ignores `_pt`
  (winit/windows/drop_handler.rs:79-155). The target is registered when the window is
  created (window.rs:1167-1195) and revoked on `WM_DESTROY` (event_loop.rs:1262). Every
  `CF_HDROP` is accepted with `DROPEFFECT_COPY` (drop_handler.rs:96-98), so a drop of an
  unsupported file type can't show the "no-drop" cursor.
- **winit macOS source.** `draggingEntered:` queues `HoveredFile`, `performDragOperation:`
  queues `DroppedFile`, and `draggingExited:` queues `HoveredFileCancelled`. There is **no
  `draggingUpdated:`**, and no location is read (winit/macos/window_delegate.rs:367-430). So
  macOS has no hover position either.
- **The fork.** `DroppedFile` becomes `WindowEvent::Drop(DropData::File)` aimed at the window.
  `HoveredFile(_) => {}` and `HoveredFileCancelled => {}` are discarded
  (vizia_winit/src/application.rs:632-637). `Drop` sets `cx.drop_data`
  (vizia_core/src/events/event_manager.rs:256-257). A view's `.on_drop` runs on the next
  MouseMove or MouseUp (vizia_core/src/modifiers/actions.rs:262-288).
- **Raw winit (probe `dnd_winit/`, logs/dnd_winit_drop.txt):** HoveredFile, then nothing
  (the `poll` lines are the probe's own `GetCursorPos` calls), then DroppedFile. The first
  `CursorMoved` arrives only *after* the drop:
  ```
  14:03:51.523 WindowEvent::HoveredFile("…probe_drop.mid")
  14:03:51.523 poll (GetCursorPos while hovered): client (10, 150)
  14:03:51.578 poll (GetCursorPos while hovered): client (78, 150)
     … 13 more polls, one per helper move, matching oledrag's client coords …
  14:03:52.388 poll (GetCursorPos while hovered): client (340, 300)
  14:03:52.947 WindowEvent::DroppedFile("…probe_drop.mid")
  14:03:52.952 WindowEvent::Focused(true)
  14:03:52.957 WindowEvent::CursorEntered
  14:03:52.957 WindowEvent::CursorMoved (340, 300)
  ```
  Leave and Esc (logs/dnd_winit_drop_leave_escape.txt) give
  `HoveredFile → HoveredFileCancelled`. On Esc the target window then also gets
  `KeyboardInput Escape Pressed` after the cancel.
- **vizia, stock (probe `dnd_vizia/`, logs/dnd_vizia_stock_drop.txt):** nothing at all
  during the hover. At the drop, the cursor is unknown (-1, -1). The post-drop MouseMove then
  delivers `.on_drop` to the pane under the cursor:
  ```
  14:07:26.620 vizia WindowEvent::Drop(File("…probe_drop.mid")); cx.mouse() cursor at that moment (-1, -1)
  14:07:26.633 vizia .on_drop on LEFT pane (.on_drop): File(…) (cursor (340, 300))
  14:07:26.633 vizia WindowEvent::MouseMove(340, 300)
  ```
  So drop-on-a-specific-view works today, but hover feedback (an insertion line, lane
  highlight or ghost clip) is impossible.
- **Polling while hovered works.** The UI thread is not blocked during a cross-process OLE
  drag: the probe's 16 ms `WaitUntil` wake-ups ran throughout and matched every helper move
  (above).
- **Own IDropTarget works (`dnd_vizia --own-droptarget`, logs/dnd_vizia_own_droptarget.txt).**
  About 120 lines with windows-sys: `RevokeDragDrop(hwnd)` (winit's), then
  `RegisterDragDrop(hwnd, ours)`, both returning 0. Every `DragOver` carries screen coords,
  which `ScreenToClient` converts. They are emitted through `ContextProxy` and the label
  follows the cursor live:
  ```
  14:10:47.983 IDropTarget::DragEnter screen (1618, 381) client (10, 150) files=Some([…probe_drop.mid])
  14:10:47.984 vizia Model: OsDrag::Enter { files: […], x: 10, y: 150 }
  14:10:48.03x IDropTarget::DragOver  screen (1686, 381) client (78, 150)   (60 DragOvers in 2 drags)
  …
  14:10:49.43x IDropTarget::Drop      screen (1948, 531) client (340, 300) files=[…]
  ```
  `leave` gives `DragLeave` and `OsDrag::Leave`. During the hover, vizia's own cursor stays
  stale: the layout system re-emits `MouseMove` at the last known position
  (vizia_core/src/systems/layout.rs:131), visible as repeated `MouseMove(400, 270)` lines. Hover
  logic must use the drag coordinates, not `cx.mouse()`.

### Recommended approach
1. **Fork, small and upstream-friendly:** forward `HoveredFile`/`HoveredFileCancelled` as new
   vizia `WindowEvent`s (e.g. `DragEnter(DropData)` / `DragLeave`). While a file hovers,
   poll the cursor once per frame in `about_to_wait` and emit a `DragOver(x, y)`. On Windows
   use `GetCursorPos`+`ScreenToClient`. On macOS use
   `NSWindow.mouseLocationOutsideOfEventStream` (needs testing on the Mac: does the target's
   run loop keep ticking during a Finder drag?). Set the cursor before `Drop`, so `Drop` has
   a valid position. That is roughly 40 lines in vizia_winit, and the app's existing drop path
   (crates/ui/src/lib.rs:631) is unchanged.
2. **If per-type feedback is wanted** (no-drop cursor for non-.mid/.chipdaw files; copy vs
   link), use the own-`IDropTarget` route on Windows. It is proven by the probe, but then the
   app (or the fork) must also deliver the drop itself, because winit's handler is gone. The
   macOS equivalent needs `draggingUpdated:` in a winit fork; we don't fork winit today.

### Risks
- Own IDropTarget: hand-written COM (refcounting, `ReleaseStgMedium`). winit's `WM_DESTROY`
  revokes whatever is registered, which releases ours (safe). If a future winit
  re-registers its own target, ours would be silently replaced.
- Polling gives positions at frame rate, not per OS event. That's fine for highlighting.
- Esc that cancels a drag is also delivered to the window as a KeyDown after the cancel (raw
  winit log). The app's Escape handler (closes the menu / piano roll) could fire.
- The test helper drags from a WinForms window, not Explorer. The data format (`CF_HDROP`) and
  the OLE path are the same. Two of about ten helper runs lost the initial button press. The
  helper detects that and logs "NO DRAG".

---

## 3. Settings and a second window

**Answer: yes on winit, with fork bugs. The fork's `Window::new` opens a real second OS window
that renders at full rate, takes input, shares the model and closes cleanly. But baseview (the
plugin backend) has no multi-window support at all. Recommendation: an in-app modal panel.**

### Evidence (probe `settings_window/`, logs/settings_window.txt)
- **Source.** `Window::new(cx, content)` registers a `WindowState`
  (vizia_winit/src/window.rs:389-410). `about_to_wait` creates OS windows for new entries and
  removes closed ones (application.rs:935-994). The fork's `examples/multiwindow.rs` uses the
  pattern the probe uses (`Binding` on a bool signal, `on_close` resets it) and says
  "This example is not supported on baseview". The `Window` view is exported only under the
  `winit` feature (src/lib.rs prelude).
- **Opens, renders, gets input, shares the model, closes:**
  ```
  14:09:00.275 AppEvent::OpenSettings (from Entity (index: 4 …))
  14:09:00.320 Settings window on_create (OS window exists)
  14:09:01.943 frames in the last 1.00 s: main=60 settings=60
  14:09:38.189 AppEvent::SetVolume(0.94) from Entity (index: 47 …), handled by the main window's model
  14:09:52.702 AppEvent::ToggleDark -> false from Entity (index: 60 …)
  14:09:09.999 AppEvent::SettingsClosed (Window::on_close)
  14:09:11.952 frames in the last 1.00 s: main=60 settings=0
  ```
  Events from the settings window bubble to the root model, the same instance as the main
  window. Signals bound in both windows update both. The fork redraws every window on each
  `RedrawRequested` (application.rs:782-806), with one vsync'd swap per window. On this
  machine both windows still held 60/60 fps.
- **Fork bugs found:**
  - **The second window's `.title()` renamed the main window.** `SetTitle` is not consumed
    by the sub-window's `Window` view (vizia_winit/src/window.rs:488-496), so it bubbles up to
    the root `Window`, which calls `set_title` on the main OS window. Both windows end up
    titled "Settings" (logs/settings_title_bug.png). This is a one-line fix (`meta.consume()`).
  - **Keyboard focus is global.** Right after the settings window opened (and before
    anything in it was clicked), key presses typed into it went to the *main window's*
    focused view: `KeyDown(Numpad6) target Entity(index: 4)` = the main window's button. For
    the app, that means Space typed into Settings would toggle playback until a settings
    control takes focus. Once the checkbox had focus, Space toggled it as expected.
  - In one run the first click in the new window did not register (the next did). Not
    investigated further.
- **baseview.** A plugin editor is a single window parented to the host. vizia_baseview has no
  `Window` view and no multi-window path. vizia_core's views do include `popup.rs`, and the
  app already builds overlays.

### Recommendation
**An in-app modal panel (overlay inside the main window) for Settings**, not a second window:
- It works identically in the standalone app and in a future baseview plugin. A second
  window can't exist there, and most hosts frown on plugins opening extra top-level windows.
- No multi-window fork bugs (title, global focus), and no second GL context and swap per
  frame.
- Space, Escape and the app's global shortcuts behave predictably. The panel should consume
  keys while open and close on Escape.
- Trade-off: it can't be moved beside the arrangement or onto another monitor, and it covers
  part of the UI. For settings (audio device, theme, MIDI) that's normal.

If a separate window is ever wanted (standalone only), the fork needs the `SetTitle` consume
fix plus focus-per-window handling first.

### Risks
- The panel needs its own focus trap and key consumption, so arrangement shortcuts don't fire
  under it.
- On the Mac, the native Settings menu item (Cmd+,), if a muda menu is added, should open the
  same panel.

---

## Files
- `probes/menu_vizia/` (muda + vizia), `probes/dnd_winit/` (raw winit logger + cursor poll),
  `probes/dnd_vizia/` (vizia drop logging + own IDropTarget), `probes/settings_window/`
  (second window). Each has a README with build and run lines.
- `tools/uxdrive.ps1` (click/keys/screenshot driver), `tools/oledrag.ps1` (OLE file-drag
  helper: drop / leave / escape).
- `probes/logs/` (raw logs and screenshots cited above). `probes/.gitignore` ignores `target/`.
