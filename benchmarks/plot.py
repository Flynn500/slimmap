"""Plot the CSV produced by `cargo run --release` in this directory.

Usage: python plot.py
Writes PNGs next to results/results.csv.
"""

import csv
from collections import defaultdict
from pathlib import Path

import matplotlib.pyplot as plt

RESULTS = Path(__file__).parent / "results"
CSV_PATH = RESULTS / "results.csv"

METRICS = [
    ("mem_bytes", "Heap memory (bytes)", "memory.png"),
    ("insert_ns_per_op", "Insert time (ns/op)", "insert.png"),
    ("get_ns_per_op", "Lookup time (ns/op)", "lookup.png"),
]


def load(path):
    series = defaultdict(lambda: defaultdict(list))
    with open(path, newline="") as f:
        for row in csv.DictReader(f):
            name, n = row["map"], int(row["n"])
            for metric, _, _ in METRICS:
                series[metric][name].append((n, float(row[metric])))
    return series


def main():
    series = load(CSV_PATH)
    for metric, label, filename in METRICS:
        fig, ax = plt.subplots(figsize=(7, 4.5))
        for name, points in series[metric].items():
            points.sort()
            xs, ys = zip(*points)
            ax.plot(xs, ys, marker="o", label=name)
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.set_xlabel("Number of entries")
        ax.set_ylabel(label)
        ax.set_title(f"{label} vs size")
        ax.grid(True, which="both", alpha=0.3)
        ax.legend()
        fig.tight_layout()
        out = RESULTS / filename
        fig.savefig(out, dpi=150)
        plt.close(fig)
        print(f"Wrote {out}")


if __name__ == "__main__":
    main()
