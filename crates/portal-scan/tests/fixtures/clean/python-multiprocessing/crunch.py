import sys
from multiprocessing import Pool

try:
    import fcntl
except ImportError:
    fcntl = None

if sys.platform == "win32":
    import msvcrt


def square(x):
    return x * x


def main():
    with Pool(4) as pool:
        print(pool.map(square, range(10)))


if __name__ == "__main__":
    main()
