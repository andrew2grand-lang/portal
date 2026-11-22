@echo off
cd /d %~dp0
python ledger.py export --month %1
pause
