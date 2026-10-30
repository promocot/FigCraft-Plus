#!/usr/bin/env python3
"""Drive a running FigCraft (Plus) through its control channel: run commands, click, type, take
screenshots. For checking add-ons by hand and for README screenshots and GIFs.

Start the app with the channel open (Linux, no display: under Xvfb):

    Xvfb :99 -screen 0 1440x900x24 &
    DISPLAY=:99 HOME=/tmp/fc-home ./target/debug/figcraft --control 7979 &

Then, from Python:

    import sys; sys.path.insert(0, "scripts/dev"); import fc
    c = fc.FC()                                   # waits for the app
    c.call("ui.set", theme="dark")
    c.ex("file.new")
    f = c.ex("frame.create", x=0, y=0, width=320, height=200, name="Card")["id"]
    c.ex("select.set", ids=[f])
    c.call("ui.set", colorPicker={"fills": True, "index": 0})   # open the colour picker
    c.shot("/abs/path/shot.png")                  # absolute paths only
    fc.pclick(c, 1260, 506)                       # click at a screenshot pixel

Notes
- `engine.execute` commands (`c.ex`) are FigCraft's command set: `figcraft-cli` or the engine's
  `spec!(...)` lists name them (`node.set`, `variable.create`, `select.set`, …).
- `ui.input` takes egui events: {"move": [x, y]}, {"down": [x, y], "button": "secondary"},
  {"up": [x, y]}, {"text": "abc"}, {"key": "Enter"}.
- Input coordinates are in UI points; screenshots are in pixels. With an interface size other than
  100% set SCALE to the factor (1.0833 at 108%); `pclick`/`pmove` divide by it.
- `ui.inspect` returns the UI state as JSON; `ui.menu.invoke {"id": "app.preferences"}` opens a
  menu item.
"""

import json
import socket
import time

SCALE = 1.0


class FC:
    def __init__(self, port: int = 7979, wait: float = 60.0):
        end = time.time() + wait
        while True:
            try:
                self.s = socket.create_connection(("127.0.0.1", port))
                break
            except OSError:
                if time.time() > end:
                    raise
                time.sleep(1)
        self.f = self.s.makefile("rw")
        self.i = 0

    def call(self, method: str, **params):
        """A control-channel method (`ui.inspect`, `ui.set`, `ui.input`, `ui.screenshot`, …)."""
        self.i += 1
        self.f.write(json.dumps({"id": self.i, "method": method, "params": params}) + "\n")
        self.f.flush()
        r = json.loads(self.f.readline())
        if not r.get("ok"):
            raise RuntimeError(f"{method}: {r.get('error')}")
        return r.get("result")

    def ex(self, command: str, **params):
        """An engine command, as the CLI and plugins run them."""
        return self.call("engine.execute", command=command, params=params)

    def shot(self, path: str):
        """A screenshot of the window to `path` (absolute)."""
        return self.call("ui.screenshot", path=path)


def pmove(c: FC, x: float, y: float):
    """Move the pointer to screenshot pixel (x, y)."""
    c.call("ui.input", events=[{"move": [x / SCALE, y / SCALE]}])


def pclick(c: FC, x: float, y: float, pause: float = 0.3):
    """Click at screenshot pixel (x, y), as a person would (move, then press and release)."""
    p = [x / SCALE, y / SCALE]
    for ev in ([{"move": p}], [{"move": p}], [{"down": p}], [{"up": p}]):
        c.call("ui.input", events=ev)
        time.sleep(pause)
