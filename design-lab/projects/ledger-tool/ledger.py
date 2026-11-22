"""Builds the monthly ledger from the bank's CSV exports."""
import argparse
import csv
import os
import winreg

from openpyxl import Workbook

EXPORTS = "C:\\Users\\sam\\Documents\\Bank exports"


def output_folder() -> str:
    """Where Excel saves files, from the registry."""
    key = winreg.OpenKey(winreg.HKEY_CURRENT_USER, r"Software\Microsoft\Office\16.0\Excel\Options")
    value, _ = winreg.QueryValueEx(key, "DefaultPath")
    return value


def rows(month: str):
    with open(EXPORTS + "\\" + month + ".csv", newline="", encoding="utf-8") as f:
        yield from csv.DictReader(f)


def export(month: str) -> str:
    book = Workbook()
    sheet = book.active
    sheet.append(["Date", "Payee", "Amount"])
    for r in rows(month):
        sheet.append([r["Date"], r["Payee"], float(r["Amount"])])
    target = output_folder() + "\\Ledger " + month + ".xlsx"
    book.save(target)
    os.startfile(target)
    return target


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["export"])
    parser.add_argument("--month", required=True)
    args = parser.parse_args()
    print(export(args.month))
