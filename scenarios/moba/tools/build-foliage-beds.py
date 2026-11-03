import argparse
import json
import os
import sys
from fractions import Fraction
from pathlib import Path

import numpy as np
import scipy.signal as ss
import soundfile as sf

ROOT = Path(__file__).resolve().parents[3]
SOUNDS = ROOT / "scenarios" / "moba" / "assets" / "sounds"
RATE = 22050
BED_N = RATE * 6
XFADE_N = round(0.040 * RATE)
EDGE_N = round(0.020 * RATE)
BIRD_FADE_N = round(0.5 * RATE)
RUSTLE_BEDS = 6
BIRD_BEDS = 4
RNG_SEED = 20261103
CEILINGS = (0.9, 0.7, 0.55, 0.45, 0.35)
DECODED_PEAK_MAX = 0.95
BIRD_NAME = "birds-isaiah658_0.ogg"


def parse_args(argv):
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True)
    parser.add_argument("--out", default=str(SOUNDS))
    parser.add_argument("--bird", default=BIRD_NAME)
    parser.add_argument(
        "--reference",
        nargs=2,
        default=[str(SOUNDS / "ambient_forest.ogg"), str(SOUNDS / "ambient_wind.ogg")],
        metavar=("FOREST", "WIND"),
    )
    return parser.parse_args(argv)


def resample_mono(x, src_rate):
    if x.ndim > 1:
        x = x.mean(axis=1)
    if src_rate == RATE:
        return x
    f = Fraction(RATE, src_rate)
    return ss.resample_poly(x, f.numerator, f.denominator)


def load_mono(path):
    x, src_rate = sf.read(str(path), dtype="float64")
    return resample_mono(x, src_rate)


def rms(x):
    return float(np.sqrt(np.mean(x ** 2)))


def find_rustles(source):
    found = []
    for dirpath, _, names in os.walk(source):
        for name in names:
            if name.startswith("rustle") and name.endswith(".flac"):
                found.append(os.path.join(dirpath, name))
    return sorted(found, key=os.path.basename)


def soft_ceiling(x, gain, ceiling):
    return ceiling * np.tanh(gain * x / ceiling)


def fit_level(x, target, ceiling):
    lo, hi = 1e-6, 1e6
    for _ in range(120):
        mid = (lo * hi) ** 0.5
        if rms(soft_ceiling(x, mid, ceiling)) < target:
            lo = mid
        else:
            hi = mid
    return soft_ceiling(x, (lo * hi) ** 0.5, ceiling)


def join(parts):
    out = parts[0].copy()
    fade_in = np.linspace(0.0, 1.0, XFADE_N)
    for p in parts[1:]:
        out[-XFADE_N:] = out[-XFADE_N:] * (1.0 - fade_in) + p[:XFADE_N] * fade_in
        out = np.concatenate([out, p[XFADE_N:]])
    return out


def edge_fade(x, n):
    x = x.copy()
    ramp = np.linspace(0.0, 1.0, n)
    x[:n] *= ramp
    x[-n:] *= ramp[::-1]
    return x


def rustle_beds(rustles):
    rng = np.random.default_rng(RNG_SEED)
    beds = []
    for i in range(RUSTLE_BEDS):
        parts = []
        chosen = []
        joined = None
        while joined is None or len(joined) < BED_N:
            k = int(rng.integers(len(rustles)))
            if k in chosen and len(chosen) < len(rustles):
                continue
            chosen.append(k)
            parts.append(rustles[k][1])
            joined = join(parts)
        beds.append({
            "name": f"foliage_bed_{i + 1:02d}",
            "audio": edge_fade(joined[:BED_N], EDGE_N),
            "sources": [rustles[k][0] for k in chosen],
        })
    return beds


def bird_beds(bird, bird_name):
    beds = []
    starts = np.linspace(0, len(bird) - BED_N, BIRD_BEDS).astype(int)
    for j, s in enumerate(starts):
        seg = bird[s:s + BED_N].copy()
        ramp = np.linspace(0.0, 1.0, BIRD_FADE_N)
        seg[:BIRD_FADE_N] *= ramp
        seg[-BIRD_FADE_N:] *= ramp[::-1]
        beds.append({"name": f"foliage_bed_{RUSTLE_BEDS + 1 + j:02d}", "audio": seg, "sources": [bird_name]})
    return beds


def target_rms(reference_paths):
    levels = [rms(load_mono(p)) for p in reference_paths]
    return sum(levels) / len(levels)


def write_bed(bed, out, target):
    path = out / f"{bed['name']}.ogg"
    for ceiling in CEILINGS:
        goal = target
        for _ in range(4):
            shaped = fit_level(bed["audio"], goal, ceiling)
            sf.write(str(path), shaped.astype(np.float32), RATE, format="OGG", subtype="VORBIS")
            back, _ = sf.read(str(path), dtype="float64")
            goal *= target / rms(back)
        if float(np.max(np.abs(back))) <= DECODED_PEAK_MAX:
            break
    else:
        raise RuntimeError(f"{bed['name']}: decoded peak above {DECODED_PEAK_MAX} at every ceiling")
    return {
        "name": bed["name"],
        "sources": bed["sources"],
        "ceiling": ceiling,
        "decoded_peak": round(float(np.max(np.abs(back))), 4),
        "decoded_rms": round(rms(back), 6),
        "seconds": round(len(back) / RATE, 4),
    }


def main():
    args = parse_args(sys.argv[1:])
    rustle_paths = find_rustles(args.source)
    if not rustle_paths:
        raise SystemExit(f"no rustle*.flac files under {args.source}")
    rustles = [(os.path.basename(p), load_mono(p)) for p in rustle_paths]
    bird = load_mono(os.path.join(args.source, args.bird))
    target = target_rms(args.reference)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    print(json.dumps({"target_rms": round(target, 6), "rustle_files": len(rustles)}))
    for bed in rustle_beds(rustles) + bird_beds(bird, args.bird):
        print(json.dumps(write_bed(bed, out, target)))


if __name__ == "__main__":
    main()
