#!/usr/bin/env python3
"""Extract a local ZIP and verify CLI + first GUI process launch. Not Gatekeeper/IME QA."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile

archive = Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix="uml-installed-") as temp:
    root = Path(temp)
    subprocess.run(["ditto", "-x", "-k", str(archive), str(root)], check=True)
    app = root / "UML Studio.app"
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
    binary = app / "Contents/MacOS/uml-studio"
    env = dict(os.environ, UML_STUDIO_HOME=str(root / "user-data"))
    project = root / "한글 설치 테스트.umlproj"
    subprocess.run([str(binary), "--review-example", str(project)], env=env, cwd=root, check=True)
    for flag, suffix in [("--export", "png"), ("--export-svg", "svg")]:
        destination = root / suffix
        subprocess.run([str(binary), flag, str(project), str(destination)], env=env, cwd=root, check=True)
        files = list(destination.glob("*." + suffix))
        assert len(files) == 3 and all(p.stat().st_size > 100 for p in files)
        print(f"Installed CLI {suffix}: {len(files)} diagrams, Korean path, embedded resources OK")
    process = subprocess.Popen([str(binary)], env=env, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        stdout, stderr = process.communicate(timeout=5)
        raise RuntimeError(f"GUI exited before smoke timeout ({process.returncode}): {stdout} {stderr}")
    except subprocess.TimeoutExpired:
        process.terminate()
        stdout, stderr = process.communicate(timeout=10)
        print("Installed bundle executable remained running for 5s; smoke process terminated without edits.")
        if stderr:
            print(stderr)
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
    print("Local ZIP extraction/CLI/startup passed. Finder/Gatekeeper/download/IME/dialogs remain manual.")
