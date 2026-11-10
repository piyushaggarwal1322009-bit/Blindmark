import sys
import requests

P = 2**61 - 1

def main():
    r1 = requests.get("http://localhost:8001/total")
    r1.raise_for_status()
    d1 = r1.json()

    r2 = requests.get("http://localhost:8002/total")
    r2.raise_for_status()
    d2 = r2.json()

    if d1["count"] != d2["count"]:
        print(f"Error: Server counts do not match ({d1['count']} vs {d2['count']})", file=sys.stderr)
        sys.exit(1)

    count = d1["count"]
    if count == 0:
        print("Error: No shares submitted yet", file=sys.stderr)
        sys.exit(1)

    mean = ((d1["total"] + d2["total"]) % P) / count
    print(mean)

if __name__ == "__main__":
    main()

