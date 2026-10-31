#!/usr/bin/env python3
"""Collect cached Cargo package license files, without fetching or changing the lockfile."""
import csv
import json
from pathlib import Path
import shutil
import subprocess
import sys

out = Path(sys.argv[1] if len(sys.argv) > 1 else "dist/ThirdPartyLicenses")
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--locked", "--offline", "--filter-platform",
    "aarch64-apple-darwin", "--format-version", "1",
]))
out.mkdir(parents=True, exist_ok=True)
missing = []
with (out / "inventory.csv").open("w", newline="") as stream:
    writer = csv.writer(stream)
    writer.writerow(["crate", "version", "license", "collected_files"])
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        if package["source"] is None:
            continue
        root = Path(package["manifest_path"]).parent
        files = set()
        for pattern in ["LICENSE*", "LICENCE*", "COPYING*", "NOTICE*", "license*", "licence*"]:
            files.update(f for f in root.glob(pattern) if f.is_file())
        if package.get("license_file"):
            files.add(root / package["license_file"])
        destination = out / (package["name"] + "-" + package["version"])
        destination.mkdir(exist_ok=True)
        collected = []
        for file in sorted(files):
            if file.is_file():
                shutil.copy2(file, destination / file.name)
                collected.append(file.name)
        if not collected:
            missing.append(package["name"] + "-" + package["version"])
        writer.writerow([package["name"], package["version"], package["license"], ";".join(collected)])
(out / "needs-review.txt").write_text("\n".join(missing) + "\n")
print(f"Collected {len(metadata['packages']) - 1} dependency entries; {len(missing)} need license-text review. See {out}")
