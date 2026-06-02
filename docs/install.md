# Install HelmBench

HelmBench can be installed from source or from GitHub release artifacts.

## From Source

```bash
cargo install --git https://github.com/thromel/helmbench --locked
```

Verify the install:

```bash
helmbench --help
helmbench doctor --repo .
```

## From Release Tarball

Download the release asset for your platform from:

```text
https://github.com/thromel/helmbench/releases
```

Assets are named:

```text
helmbench-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
helmbench-vX.Y.Z-aarch64-apple-darwin.tar.gz
helmbench-vX.Y.Z-x86_64-apple-darwin.tar.gz
```

Verify the checksum:

```bash
shasum -a 256 -c helmbench-vX.Y.Z-aarch64-apple-darwin.tar.gz.sha256
```

Unpack and install:

```bash
tar -xzf helmbench-vX.Y.Z-aarch64-apple-darwin.tar.gz
sudo cp helmbench-vX.Y.Z-aarch64-apple-darwin/helmbench /usr/local/bin/helmbench
helmbench --help
```

## Provenance Attestations

Release builds include GitHub artifact provenance attestations for the packaged
tarballs. Verify an asset with GitHub CLI:

```bash
gh attestation verify \
  helmbench-vX.Y.Z-aarch64-apple-darwin.tar.gz \
  --repo thromel/helmbench
```

The attestation proves the artifact was produced by the HelmBench GitHub
Actions release workflow for this repository. The `.sha256` file verifies the
downloaded bytes match the published checksum.

## Local Verification

For development checkouts:

```bash
./scripts/verify.sh
```

This runs formatting, tests, clippy, CLI help checks, demo benchmark generation,
matrix verification, matrix history generation, evidence-bundle verification,
and whitespace checks.
