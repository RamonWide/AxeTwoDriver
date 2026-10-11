# Releasing

How a version of AxeTwoDriver gets from `main` to a download. The work is done by
[`.github/workflows/release.yml`](../.github/workflows/release.yml); a maintainer starts it and
reviews the result.

## What a release contains

| File | Contents |
|---|---|
| `AxeTwoDriver-<ver>-windows-x64.zip` | `axetwo-setup.exe`, `axetwo.exe`, `axetwo-panel.exe`, `axetwo_daw.dll`, both licences, `THIRD-PARTY-LICENSES.txt`, attribution, a README and `SOURCE.txt` |
| `SHA256SUMS.txt` | a checksum for the zip |

The zip also gets a GitHub build provenance attestation, which ties it to the commit and workflow
run that produced it.

`axetwo_daw.dll` comes from the separate, GPLv3
[AxeTwoDriver-DAW](https://github.com/RamonWide/AxeTwoDriver-DAW) repository; the workflow builds
it from that repository's `main`. `SOURCE.txt` in the zip names the exact commit of both
repositories, which is what the GPL's source requirement points at. **Do not delete or rewrite a
commit that a published release names.**

The programs are **not** code-signed, so Windows SmartScreen warns on first launch;
[`install.md`](install.md) is where users are told what to expect. Signing needs a paid
certificate; add it to the workflow when the project has one.

## Steps

1. Make sure `main` is green in CI, in both repositories.
2. Choose the version. Below 1.0: a new minor version for new features or any change to what the
   installer does to a system, a patch version for fixes only.
3. In one pull request:
   - set `version` under `[workspace.package]` in `Cargo.toml` and run `cargo build` so that
     `Cargo.lock` follows;
   - in `CHANGELOG.md`, rename "Unreleased" to the version with today's date, start a new empty
     "Unreleased", and update the links at the bottom;
   - update `ROADMAP.md` (the "Last reviewed" line) and `docs/gaps.md`.
   Then, in AxeTwoDriver-DAW, run `cargo build` and commit its `Cargo.lock`: it records the
   version of the crates here, and the release build refuses a lock file that is out of date.
4. Merge it, then tag that commit and push the tag:

   ```sh
   git tag -a v0.2.0 -m "AxeTwoDriver 0.2.0"
   git push origin v0.2.0
   ```

   The workflow refuses to run if the tag does not match the version in `Cargo.toml`.
5. The workflow builds the zip and creates a **draft** release. Download it and, on a machine with
   a unit: run the installer, open the control panel, play and record in a DAW, then uninstall
   from Installed apps and install again.
6. Edit the draft's notes (the changelog section is a good start), then publish it.

## If something goes wrong

A draft release can be deleted and the tag moved while nobody has downloaded anything. Once a
release is published, do not replace its files: fix the problem and publish a new patch version.

## Trying the packaging without a release

Run the workflow by hand from the Actions tab ("Run workflow"), or locally:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package.ps1
```

Both build the zip and skip creating a release.
