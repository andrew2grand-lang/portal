from concurrent.futures import ProcessPoolExecutor

def work(x):
    return x + 1

def main():
    with ProcessPoolExecutor() as ex:
        print(list(ex.map(work, range(3))))

main()
