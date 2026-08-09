# Baud — Minimal Serial Terminal GUI (v1 Design)

## Purpose

A lightweight desktop GUI serial terminal for debugging IoT devices (ESP32,
Arduino, bare-metal MCUs, etc.) over a serial/COM connection. The user selects
a detected COM port and baud rate, connects, and sees streamed device output
in a scrolling terminal view, with the ability to send text commands back.
Minimal interface, no unnecessary features.

## Non-goals (v1)

- Hex/binary view of data
- Logging/exporting terminal output to a file
- Auto-reconnect on device unplug/replug
- Multiple simultaneous port connections
- Macros, scripting, or command history/autocomplete
- Persisting settings (port/baud/theme) across app restarts

These may be considered for later iterations but are explicitly excluded to
keep the first working version small and easy to verify.

## Tech Stack

- **GUI:** `eframe` + `egui` (immediate-mode GUI, single window, no complex
  widget tree to maintain)
- **Serial I/O:** `serialport` crate — cross-platform port enumeration
  (`serialport::available_ports()`) and blocking read/write
- **Concurrency:** a dedicated background thread owns the open
  `serialport::SerialPort` handle and performs blocking reads; it forwards
  each received chunk to the UI thread via `std::sync::mpsc::channel`. The
  egui `update()` loop drains the channel each frame and requests a repaint
  when new data arrives (`ctx.request_repaint()`), so the UI thread never
  blocks on serial I/O regardless of baud rate or device silence.
- **Sending data:** writes happen synchronously from the UI thread through a
  `Mutex`-guarded clone/handle of the port (writes are small and infrequent,
  so this does not need to go through the background thread).

## UI Layout

Single window, top-to-bottom:

### Top bar (one row, wraps if needed)

- **Port dropdown** — populated from `serialport::available_ports()` at
  startup
- **Refresh button** — re-scans and repopulates the port dropdown (manual
  only, no background polling)
- **Baud dropdown** — standard rates: 9600, 19200, 38400, 57600, 115200
  (default), 230400 — plus an editable custom entry for non-standard rates
- **Connect / Disconnect button** — single toggle button; label and action
  change based on connection state
- **Timestamp toggle** — show/hide `[HH:MM:SS.mmm]` prefix on each received
  line
- **Dark/Light mode toggle**

### Terminal pane (fills remaining vertical space)

- Scrolling read-only text view of received data, rendered one line at a
  time as `\n`-delimited chunks arrive (partial lines buffered until a
  newline or flushed on disconnect)
- Each line optionally prefixed with a timestamp captured at receive time
- **Autoscroll toggle** — when on, view auto-scrolls to bottom on new data;
  when off, view position is left alone so the user can scroll back through
  history
- **Clear button** — wipes the in-memory buffer and view

### Bottom input bar

- Single-line text field for outgoing data
- **Line-ending dropdown**: None / `\n` / `\r` / `\r\n` (default `\n`)
- **Send button** — also triggered by pressing Enter in the text field
- Input field is disabled when not connected

## State Model

```
enum ConnectionState {
    Disconnected,
    Connected { port_name: String, baud: u32 },
}
```

The app holds one `ConnectionState`, one growable text buffer of received
lines (with timestamps stored alongside, formatted at render time based on
the toggle), and UI-only state (selected port/baud, autoscroll flag, theme,
line-ending choice, input text).

## Error Handling

- Failing to open a port (busy, permission denied, device unplugged between
  refresh and connect) shows a dismissible inline error banner at the top of
  the window; state remains `Disconnected`.
- If the background reader thread hits a read error (e.g., device
  unplugged mid-session), it sends an error/disconnect signal through the
  same channel; the UI transitions to `Disconnected` and shows a banner.
- Disconnecting cleanly signals the reader thread to stop and joins it
  before releasing the port handle, so the port is fully released for
  reuse (e.g., re-flashing the device) immediately after disconnect.
- Reconnecting after any failure or manual disconnect always goes through
  the normal Connect button flow — no automatic retry in v1.

## Testing Approach

Serial I/O against real hardware isn't practical to cover with automated
unit tests. Verification for v1 is:

- Manual end-to-end test against a real connected device (available at
  COM9 during development) — connect, observe streamed output, send a
  command, disconnect, reconnect.
- Unit tests for pure logic that doesn't touch hardware:
  - Line-ending formatting (`None`/`\n`/`\r`/`\r\n` produce correct bytes)
  - Timestamp formatting
  - Line-buffering logic (partial chunks correctly assembled into lines)

## Out-of-scope clarifications

- No settings persistence means every launch starts at: no port selected,
  115200 baud, dark mode (matches system if easily available via egui,
  otherwise defaults to dark), timestamps off, autoscroll on, line ending
  `\n`.
