import os
import subprocess

subprocess.run(["ls", "-la"], check=True)
os.system("clear")
subprocess.run(["python", "-m", "pytest"])
