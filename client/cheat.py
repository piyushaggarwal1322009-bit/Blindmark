import secrets
import requests

P = 2**61 - 1

def split(x: int):
    r = secrets.randbelow(P)
    return r, (x - r) % P

def main():
    val = 10000000
    s1, s2 = split(val)

    r1 = requests.post("http://localhost:8001/submit", json={"share": s1})
    r1.raise_for_status()

    r2 = requests.post("http://localhost:8002/submit", json={"share": s2})
    r2.raise_for_status()

if __name__ == "__main__":
    main()

