import mlx.core as mx
from mlx_lm import load, generate

model, tok = load("mlx-community/Llama-3.2-1B-Instruct-4bit")
