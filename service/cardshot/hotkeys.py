"""全局热键监听（读取 evdev，与前台应用共享事件流，不做独占）"""
import struct
import threading

EVENT = "/dev/input/event3"
EV_KEY = 0x01
KEY_PRESS = 1
KEY_RELEASE = 0

SHIFT_CODES = {42, 54}
NAME_TO_CODE = {
    "space": 57, "enter": 28, "esc": 1, "tab": 15, "backspace": 14,
    "z": 44, "x": 45, "c": 46, "f": 33,
}


def parse_chord(text):
    """'shift+space' -> ('shift', 57)"""
    parts = [p.strip().lower() for p in text.split("+") if p.strip()]
    mod = None
    code = None
    for p in parts:
        if p in ("shift", "ctrl", "alt"):
            mod = p
        elif p in NAME_TO_CODE:
            code = NAME_TO_CODE[p]
        elif len(p) == 1:
            code = ord(p) - ord("a") + 44 if p.isalpha() else NAME_TO_CODE.get(p)
    return mod, code


class HotkeyListener(threading.Thread):
    def __init__(self, handlers):
        """handlers: [(chord_str, callable)]"""
        super().__init__(daemon=True)
        self._chords = []
        self._shift = False
        for text, cb in handlers:
            mod, code = parse_chord(text)
            if code is not None:
                self._chords.append((mod, code, cb))

    def run(self):
        try:
            f = open(EVENT, "rb")
        except Exception:
            return
        try:
            while True:
                data = f.read(24)
                if len(data) < 24:
                    break
                _s, _u, etype, code, value = struct.unpack("@llHHi", data)
                if etype != EV_KEY:
                    continue
                if code in SHIFT_CODES:
                    self._shift = value != KEY_RELEASE
                    continue
                if value != KEY_PRESS:
                    continue
                for mod, hcode, cb in self._chords:
                    if hcode == code and ((mod == "shift") == self._shift or mod is None):
                        try:
                            cb()
                        except Exception:
                            pass
        finally:
            f.close()
