import torch
import flash_attn

model = torch.nn.Linear(4, 4).cuda()
x = torch.zeros(4).to("cuda:0")
y = torch.ones(4, device="cuda")
device = "cuda" if torch.cuda.is_available() else "cpu"
