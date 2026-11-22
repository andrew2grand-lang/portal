import os
import signal
from os import geteuid

uid = os.getuid()
pid = os.fork()
signal.signal(signal.SIGHUP, lambda *a: None)
os.kill(pid, signal.SIGKILL)
if hasattr(os, "uname"):
    print(os.uname())
