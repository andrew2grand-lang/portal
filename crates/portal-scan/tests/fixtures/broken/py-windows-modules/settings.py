import winreg
import win32api

def key_pressed():
    import msvcrt
    return msvcrt.kbhit()
