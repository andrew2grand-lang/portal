from multiprocessing import Pool

def square(x):
    return x * x

with Pool(4) as pool:
    print(pool.map(square, range(10)))
