# Mandelbrot: @micropython.viper against a Rust native .mpy, on the board.
#
# Copy this file, `pixels.mpy` (viper) and `pixels_rs.mpy` (Rust) for the
# board's architecture to CIRCUITPY. Do not copy `src/pixels.py`.
#
# The two are timed alternately rather than one after the other, so that a chip
# warming up over the run cannot be mistaken for one variant being slower.

import gc
import time
from array import array

try:
    import microcontroller
except ImportError:
    microcontroller = None

# (width, height, max_iter, expected checksum)
WORKLOADS = (
    (160, 120, 64, 407644),  # turbo's published size, for comparing against its table
    (320, 240, 128, 2914242),
)
RUNS = 500
CHECK_RUNS = 5
PROGRESS_EVERY = 50


def sweep(row_fn, row, width, height, max_iter):
    dx = (3 << 12) // width
    for r in range(height):
        row_fn(row, width, dx, ((r * 2) << 12) // height - (1 << 12), max_iter)


def checksum(row_fn, row, width, height, max_iter):
    dx = (3 << 12) // width
    total = 0
    for r in range(height):
        row_fn(row, width, dx, ((r * 2) << 12) // height - (1 << 12), max_iter)
        total += sum(row)
    return total


def cpu_temperature():
    if microcontroller is None:
        return None
    try:
        return microcontroller.cpu.temperature
    except (AttributeError, NotImplementedError, RuntimeError):
        return None


def variants():
    found = []

    try:
        import pixels

        found.append(
            (
                "viper",
                lambda row, w, h, it: sweep(pixels.mandel_row, row, w, h, it),
                lambda row, w, h, it: checksum(pixels.mandel_row, row, w, h, it),
            )
        )
    except ImportError:
        print("**pixels.mpy (viper) not found**")

    try:
        import pixels_rs

        found.append(
            (
                "rust",
                lambda row, w, h, it: sweep(pixels_rs.mandel_row, row, w, h, it),
                lambda row, w, h, it: checksum(pixels_rs.mandel_row, row, w, h, it),
            )
        )
        # Same arithmetic with the row loop moved into Rust too, so the
        # difference against "rust" is the per-row boundary cost.
        found.append(
            (
                "rust all",
                lambda row, w, h, it: pixels_rs.mandel_sum(w, h, it),
                lambda row, w, h, it: pixels_rs.mandel_sum(w, h, it),
            )
        )
    except ImportError:
        print("**pixels_rs.mpy (Rust) not found**")

    return found


def summarize(name, times, temp_before, temp_after, baseline):
    times = sorted(times)
    n = len(times)
    median = times[n // 2] if n % 2 else (times[n // 2 - 1] + times[n // 2]) / 2
    print(
        "| %s | %.1f | %.1f | %.1f | %.1f | %s | %s |"
        % (
            name,
            median / 1000,
            sum(times) / n / 1000,
            times[0] / 1000,
            times[-1] / 1000,
            "-" if temp_before is None else "%.1f -> %.1f" % (temp_before, temp_after),
            "-" if baseline is None else "%.2fx" % (baseline / median),
        )
    )
    return median


def main():
    print("# Mandelbrot: viper against Rust")
    found = variants()
    if not found:
        print()
        print("Nothing to benchmark.")
        return

    for width, height, max_iter, expected in WORKLOADS:
        print()
        print("## %dx%d, %d iterations" % (width, height, max_iter))
        print()
        print("%d checked runs, then %d timed runs." % (CHECK_RUNS, RUNS))
        print()

        row = bytearray(width)

        # Correctness first, over its own runs. Summing the rows is output
        # handling, so the timed section below discards the pixels and measures
        # only the computation.
        for name, _, verify in found:
            for _ in range(CHECK_RUNS):
                value = verify(row, width, height, max_iter)
                if value != expected:
                    print(
                        "- **%s CHECKSUM %d != %d**" % (name, value, expected)
                    )
                    break
            else:
                print("- `%s` checksum %d ok" % (name, expected))
            time.sleep(0.2)  # let USB drain, so a later crash cannot eat this line

        times = {}
        temps = {}
        for name, _, _ in found:
            # Preallocated, and microseconds as ints, so that 500 samples do not
            # grow the heap while the thing they measure is running.
            times[name] = array("l", (0 for _ in range(RUNS)))
            temps[name] = [None, None]

        for run in range(RUNS):
            for name, fn, _ in found:
                gc.collect()
                before = cpu_temperature()
                start = time.monotonic_ns()
                fn(row, width, height, max_iter)
                elapsed = (time.monotonic_ns() - start) // 1000
                after = cpu_temperature()

                times[name][run] = elapsed
                if temps[name][0] is None:
                    temps[name][0] = before
                temps[name][1] = after

            if run == 0:
                one = sum(times[each][0] for each, _, _ in found)
                print()
                print("- roughly %d s for this workload" % (one * RUNS // 1000000))
            elif (run + 1) % PROGRESS_EVERY == 0:
                print("- %d/%d" % (run + 1, RUNS))

        print()
        print(
            "| variant | median | mean | min | max | temp C | vs %s |" % found[0][0]
        )
        print("| :-- | --: | --: | --: | --: | :-: | --: |")
        baseline = None
        for name, _, _ in found:
            median = summarize(
                name, times[name], temps[name][0], temps[name][1], baseline
            )
            if baseline is None:
                baseline = median

        gc.collect()
        print()
        print("All times in milliseconds. Free memory %d bytes." % gc.mem_free())


main()
