# Security Policy

AxeTwoDriver parses USB descriptors and audio packets from a device, calls
Windows device-installation and USB APIs through `unsafe` code, and has one
command that runs with administrator rights. Treat everything a device sends
as potentially malicious.

## Supported versions

AxeTwoDriver is experimental. Only the latest release and the `main` branch
receive fixes.

## Reporting a vulnerability

Please report privately, and give the maintainers a reasonable opportunity to
investigate and release a fix before publishing details.

Use GitHub's private reporting: on the repository page open **Security ›
Report a vulnerability**
([direct link](https://github.com/RamonWide/AxeTwoDriver/security/advisories/new)).
Do not open a public issue containing a working exploit.

If private reporting is unavailable, open a public issue that says only that
you have a security report and need a private channel, with no technical
detail.

You can expect an acknowledgement within seven days. This is a volunteer
project; there is no bug bounty.

## What to include

- the AxeTwoDriver version or commit;
- Windows version and architecture;
- the affected component (see the scope below);
- the security impact and what access an attacker needs;
- exact steps to reproduce, with expected and actual behaviour.

## Scope

Security reports may cover:

- **Parsing:** descriptors and packets from the device (`axetwo-usb`,
  `axetwo-stream`). A crafted device must not be able to crash the program or
  read or write memory out of bounds.
- **`unsafe` code:** anything in `axetwo-win`, in particular the lifetime of
  buffers handed to the host controller.
- **The elevated command:** `axetwo setup` must only ever act on the device
  node it names and only ever select Windows' own `winusb.inf`.
- **The supply chain:** dependencies and the release workflow.

Out of scope: anything that requires an attacker to already have
administrator rights on the machine, and the behaviour of Windows' own
drivers.
