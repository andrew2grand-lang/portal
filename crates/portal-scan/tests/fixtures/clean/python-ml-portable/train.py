import torch


def pick_device() -> str:
    if torch.cuda.is_available():
        return "cuda"
    if torch.backends.mps.is_available():
        return "mps"
    return "cpu"


device = torch.device(pick_device())
model = torch.nn.Linear(4, 2).to(device)
