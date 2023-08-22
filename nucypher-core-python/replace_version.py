"""
This script is used to build a source distribution in CI.
During development it is convenient to have a relative path to `nucypher-core` in `Cargo.toml`,
but since the source distribution will not include the main crate,
we need to re-set it to the published version (and then change it back).

See https://github.com/nucypher/rust-umbral/issues/80
"""

import sys
import re


def get_version():

    with open('Cargo.toml') as f:
        lines = f.readlines()

    for line in lines:
        if m := re.match(r'^version = "(\d+\.\d+\.\d+)"$', line):
            version = m[1]
            break
    else:
        raise RuntimeError("Cannot find the package version")

    return version

def relative_to_published():

    version = get_version()

    with open('Cargo.toml') as f:
        lines = f.readlines()

    for i, line in enumerate(lines):
        if line.startswith('nucypher-core = { path = "../nucypher-core"'):
            new_line = line.replace('path = "../nucypher-core"', f'version = "{version}"')
            lines[i] = new_line
            break
    else:
        raise RuntimeError("Cannot find the nucypher-core dependency")

    with open('Cargo.toml', 'w') as f:
        f.write(''.join(lines))


def published_to_relative():

    version = get_version()

    with open('Cargo.toml') as f:
        lines = f.readlines()

    for i, line in enumerate(lines):
        if line.startswith(f'nucypher-core = {{ version = "{version}"'):
            new_line = line.replace(f'version = "{version}"', 'path = "../nucypher-core"')
            lines[i] = new_line
            break
    else:
        raise RuntimeError("Cannot find the nucypher-core dependency")

    with open('Cargo.toml', 'w') as f:
        f.write(''.join(lines))


if __name__ == '__main__':
    if sys.argv[1] == 'relative-to-published':
        relative_to_published()
    elif sys.argv[1] == 'published-to-relative':
        published_to_relative()
    else:
        raise RuntimeError(f"Unknown command: {sys.argv[1]}")
