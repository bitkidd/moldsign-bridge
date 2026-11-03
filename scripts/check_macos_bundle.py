"""Reject accidental dependencies on the build machine's Homebrew libraries."""
import subprocess
from pathlib import Path

app = Path('src-tauri/target/aarch64-apple-darwin/release/bundle/macos/MoldSign Bridge Alpha.app')
for binary in (app / 'Contents/MacOS').iterdir():
    output = subprocess.check_output(['otool', '-L', str(binary)], text=True)
    for line in output.splitlines()[1:]:
        dependency = line.strip().split(' (')[0]
        if not dependency.startswith(('/usr/lib/', '/System/Library/', '@')):
            raise SystemExit(f'Non-system dependency in {binary.name}: {dependency}')
print('Bundle dependencies use system or bundled libraries.')
