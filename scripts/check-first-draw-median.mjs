import { median } from "./perf/first-draw.mjs";

const fixtures = [
  { name: "odd count", samples: [5000, 1000, 3000], expected: 3000 },
  { name: "single sample", samples: [7000], expected: 7000 },
  { name: "even count", samples: [4000, 1000, 3000, 2000], expected: 2500 },
  { name: "two samples are averaged, not the slower", samples: [10000, 6000], expected: 8000 },
  { name: "even count, unsorted input", samples: [9, 1, 8, 2], expected: 5 },
];

const upperMiddle = (samples) => {
  const sorted = [...samples].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
};

const rejected = (implementation) => fixtures.filter((f) => implementation(f.samples) !== f.expected).map((f) => f.name);

const wrong = rejected(median);
if (wrong.length) throw new Error("median is wrong for: " + wrong.join(", "));

const controlRejected = rejected(upperMiddle);
if (controlRejected.length === 0) throw new Error("negative control: the fixtures do not reject the upper-middle median");

let emptyRefused = false;
try {
  median([]);
} catch (err) {
  emptyRefused = err instanceof RangeError;
}
if (!emptyRefused) throw new Error("median of no samples must throw a RangeError");

console.log(`first-draw median ok (${fixtures.length} fixtures; upper-middle control rejected on: ${controlRejected.join(", ")})`);
