import fcntl
import sys

try:
    import pwd
except ImportError:
    pwd = None

if sys.platform != "win32":
    import resource

def raw_mode():
    import termios
    return termios
