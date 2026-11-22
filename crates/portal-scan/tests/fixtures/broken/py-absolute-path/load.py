import pandas as pd
DATA = "/Users/ana/projects/sales/data.csv"
MODELS = "/home/ana/models"

def load():
    """Reads /Users/ana/notes.txt in the docstring, which is fine."""
    return pd.read_csv(DATA)
