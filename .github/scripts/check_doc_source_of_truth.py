#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
FILES = [
    ROOT / 'README.md',
    ROOT / 'USAGE.md',
    ROOT / 'PARITY.md',
    ROOT / 'PHILOSOPHY.md',
    ROOT / 'ROADMAP.md',
    ROOT / '.github' / 'FUNDING.yml',
]
FILES.extend(sorted((ROOT / 'docs').rglob('*.md')) if (ROOT / 'docs').exists() else [])

FORBIDDEN = {
    r'github\.com/Yeachan-Heo/rimfrost(?!-parity)': 'replace old rimfrost GitHub links with Fei2-Labs/rimfrost',
    r'github\.com/code-yeongyu/rimfrost': 'replace stale alternate rimfrost GitHub links with Fei2-Labs/rimfrost',
    r'discord\.gg/6ztZB9jvWq': 'replace the stale Fei2-Labs Discord invite with the current invite',
    r'api\.star-history\.com/svg\?repos=Yeachan-Heo/rimfrost': 'update star-history embeds to Fei2-Labs/rimfrost',
    r'star-history\.com/#Yeachan-Heo/rimfrost': 'update star-history links to Fei2-Labs/rimfrost',
    r'assets/rimfrost-hero\.jpeg': 'rename stale hero asset references to assets/rimfrost-hero.jpeg',
    r'assets/instructkr\.png': 'remove stale instructkr image references',
}

errors: list[str] = []
for path in FILES:
    if not path.exists():
        continue
    text = path.read_text(encoding='utf-8')
    for pattern, message in FORBIDDEN.items():
        for match in re.finditer(pattern, text):
            line = text.count('\n', 0, match.start()) + 1
            errors.append(f'{path.relative_to(ROOT)}:{line}: {message}')

if errors:
    print('doc source-of-truth check failed:', file=sys.stderr)
    for error in errors:
        print(f'  - {error}', file=sys.stderr)
    sys.exit(1)

print('doc source-of-truth check passed')
