"""ACT LED 闪烁反馈"""
import threading
import time

CANDIDATES = [
    "/sys/class/leds/ACT/brightness",
    "/sys/class/leds/led0/brightness",
]


class Feedback:
    def __init__(self, enabled=True):
        self.enabled = enabled
        self.path = None
        for p in CANDIDATES:
            if _writable(p):
                self.path = p
                break
        self._lock = threading.Lock()

    def blink(self, times=1, on=0.06, off=0.08):
        if not self.enabled or not self.path:
            return
        threading.Thread(target=self._run, args=(times, on, off), daemon=True).start()

    def _run(self, times, on, off):
        with self._lock:
            try:
                for _ in range(times):
                    _write(self.path, "1")
                    time.sleep(on)
                    _write(self.path, "0")
                    time.sleep(off)
            except Exception:
                pass


def _writable(p):
    try:
        return __import__("os").access(p, __import__("os").W_OK)
    except Exception:
        return False


def _write(path, value):
    with open(path, "w") as f:
        f.write(value)
