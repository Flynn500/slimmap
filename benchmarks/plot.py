"""Plot the CSVs produced by `cargo run --release` in this directory.

Usage: python plot.py
Reads results/results.csv, results/string_keys.csv,
results/many_small.csv, results/shuffled.csv and
results/half_miss.csv, and writes PNGs
next to them.
"""

import csv
from collections import defaultdict
from pathlib import Path

import matplotlib.pyplot as plt

RESULTS = Path(__file__).parent / "results"

# (csv path, x column, x label, [(metric column, y label, output file)])
CHARTS = [
    (
        RESULTS / "results.csv",
        "n",
        "Number of entries",
        [
            ("mem_bytes", "Heap memory (bytes)", "memory.png"),
            ("insert_ns_per_op", "Insert time (ns/op)", "insert.png"),
            ("get_ns_per_op", "Lookup time (ns/op)", "lookup.png"),
        ],
    ),
    (
        RESULTS / "string_keys.csv",
        "n",
        "Number of entries",
        [
            ("mem_bytes", "Heap memory, String keys (bytes)", "string_memory.png"),
            ("insert_ns_per_op", "Insert time, String keys (ns/op)", "string_insert.png"),
            ("get_ns_per_op", "Lookup time, String keys (ns/op)", "string_lookup.png"),
        ],
    ),
    (
        RESULTS / "many_small.csv",
        "entries_per_map",
        "Entries per map (10,000 maps)",
        [
            ("bytes_per_map", "Heap per map (bytes)", "many_small_memory.png"),
            ("insert_ns_per_op", "Insert time (ns/entry)", "many_small_insert.png"),
            ("get_ns_per_op", "Lookup time (ns/lookup)", "many_small_lookup.png"),
        ],
    ),
    (
        RESULTS / "shuffled.csv",
        "n",
        "Number of entries",
        [
            ("get_ns_per_op", "Lookup time, shuffled order (ns/op)", "shuffled_lookup.png"),
        ],
    ),
    (
        RESULTS / "half_miss.csv",
        "n",
        "Number of entries",
        [
            ("get_ns_per_op", "Lookup time, 50% misses (ns/op)", "half_miss_lookup.png"),
        ],
    ),
]


def load(path, x_col, metrics):
    """Returns {metric: {map name: sorted [(x, y)]}}."""
    series = defaultdict(lambda: defaultdict(list))
    with open(path, newline="") as f:
        for row in csv.DictReader(f):
            name, x = row["map"], int(row[x_col])
            for metric, _, _ in metrics:
                series[metric][name].append((x, float(row[metric])))
    for by_name in series.values():
        for points in by_name.values():
            points.sort()
    return series


def plot_chart(series, metric, y_label, x_label, out):
    fig, ax = plt.subplots(figsize=(7, 4.5))
    for name, points in series[metric].items():
        xs, ys = zip(*points)
        ax.plot(xs, ys, marker="o", label=name)
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel(x_label)
    ax.set_ylabel(y_label)
    ax.set_title(f"{y_label} vs {x_label.lower()}")
    ax.grid(True, which="both", alpha=0.3)
    ax.legend()
    fig.tight_layout()
    fig.savefig(out, dpi=150)
    plt.close(fig)
    print(f"Wrote {out}")


def main():
    for csv_path, x_col, x_label, metrics in CHARTS:
        if not csv_path.exists():
            print(f"Skipping {csv_path.name}: not found (run `just bench` first)")
            continue
        series = load(csv_path, x_col, metrics)
        for metric, y_label, filename in metrics:
            plot_chart(series, metric, y_label, x_label, RESULTS / filename)


if __name__ == "__main__":
    main()
