import os

with open("/proc/cpuinfo", encoding="utf-8") as f:
    cpu = f.read()
thermal = "/sys/class/thermal/thermal_zone0/temp"
hosts = open("/etc/hosts", encoding="utf-8").read()
