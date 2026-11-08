#!/usr/bin/env python3
"""Build import-free Rust WASM effects and size/SHA-256-bound catalogs.

The default build preserves the three bundled examples. --collection builds the
25 additional effects into effects/development for the local directory transport.
No upload, publishing, network access or Appwrite calls are performed here.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent.parent
SYSROOT = subprocess.run(['rustc', '--print', 'sysroot'], check=True,
                         capture_output=True, text=True).stdout.strip()


def compile_guest(source, ident, destination, bounded=False):
    command = [
        'rustc', str(source), '--crate-name', 'effect_' + ident.replace('-', '_'),
        '--edition', '2021', '--crate-type', 'cdylib', '--target',
        'wasm32-unknown-unknown', '-C', 'opt-level=3', '-C', 'panic=abort',
        '-C', 'strip=symbols', '--cfg', f'effect="{ident}"',
        # Keep local paths (checkout, sysroot, home) out of the artefacts.
        '--remap-path-prefix', f'{ROOT}=.',
        '--remap-path-prefix', f'{SYSROOT}=/rustc',
        '--remap-path-prefix', f'{Path.home()}=~',
    ]
    if bounded:
        command += [
            '-C', 'link-arg=-zstack-size=65536',
            '-C', 'link-arg=--initial-memory=262144',
            '-C', 'link-arg=--max-memory=262144',
        ]
    subprocess.run(command + ['-o', str(destination)], check=True)
    destination.chmod(0o644)
    data = destination.read_bytes()
    if len(data) > 512 * 1024:
        raise ValueError(f'{ident} exceeds the sandbox package limit')
    return dict(target='wasm32-unknown-unknown', url=destination.name,
                size=len(data), sha256=hashlib.sha256(data).hexdigest())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version', default='1.0.0')
    parser.add_argument('--sequence', type=int)
    parser.add_argument('--out-dir', type=Path)
    parser.add_argument('--collection', action='store_true',
                        help='Build the 25 local effects alongside the examples')
    args = parser.parse_args()
    sequence = args.sequence if args.sequence is not None else (2 if args.collection else 1)
    output = args.out_dir or (ROOT / 'effects/development' if args.collection else ROOT / 'effects')
    if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+', args.version) or sequence < 1:
        parser.error('Use numeric major.minor.patch and a positive catalog sequence')
    if args.collection and output.resolve() == (ROOT / 'effects').resolve():
        parser.error('The development collection must use a separate --out-dir')
    # Directory transport uses assets beside the catalog; bundled assets retain packages/.
    packages = output if args.collection else output / 'packages'
    packages.mkdir(parents=True, exist_ok=True)
    examples = [
        ('aurora', 'Aurora', 'Flowing teal and violet curtains.', 'Ambient', ['#10a37f', '#8b5cf6']),
        ('comet', 'Comet', 'A warm comet crossing the shared device canvas.', 'Motion', ['#ffb454', '#3344ff']),
        ('prism', 'Prism', 'A vivid two-dimensional field of shifting colour.', 'Colour', ['#f43f5e', '#3b82f6']),
    ]
    definitions = [dict(id=ident, name=name, description=description, category=category,
                        preview=preview, parameters=[
                            dict(key='speed', label='Speed', min=0.0, max=3.0, default=1.0, step=0.05),
                            dict(key='brightness', label='Intensity', min=0.0, max=1.0, default=1.0, step=0.01),
                        ]) for ident, name, description, category, preview in examples]
    if args.collection:
        collection = json.loads((ROOT / 'effects/collection.json').read_text(encoding='utf-8'))
        if len(collection) != 25:
            raise ValueError('Expected exactly 25 development effects')
        definitions += collection
    entries = []
    for index, definition in enumerate(definitions):
        ident = definition['id']
        source = ROOT / ('effects/examples/effect.rs' if index < 3 else 'effects/collection/effect.rs')
        artifact = compile_guest(source, ident, packages / f'{ident}-{args.version}.wasm', index >= 3)
        entries.append(dict(definition, version=args.version, api=1,
                            publisher=dict(id='leotrax3d', name='freeRGB Team'),
                            permissions=[], artifact=artifact))
    # Write the catalog only after every compilation succeeded.
    (output / 'catalog.json').write_text(
        json.dumps(dict(schema=1, sequence=sequence, effects=entries), indent=2) + '\n',
        encoding='utf-8')
    print(f'Built {len(entries)} Rust WebAssembly effects in {output}')


if __name__ == '__main__':
    main()
