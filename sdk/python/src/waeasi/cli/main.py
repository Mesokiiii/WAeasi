"""waeasi.cli.main — entry point dispatched by the ``waeasi`` console script."""

from __future__ import annotations

import sys

from . import ui
from .commands import cmd_build, cmd_deploy, cmd_init

SDK_VERSION = "0.1.0"

USAGE = (
    f"{ui.bold('waeasi')} — official Python SDK for WAeasi components\n"
    "\n"
    "Usage: waeasi <command> [options]\n"
    "\n"
    "Commands:\n"
    "  init    create a new component project from a template\n"
    "  build   componentize, wizer, sign, bundle\n"
    "  deploy  push a built bundle to a WAeasi node\n"
    "\n"
    "Run 'waeasi <command> --help' for command-specific options.\n"
)


def main(argv: list[str] | None = None) -> int:
    argv = list(argv if argv is not None else sys.argv[1:])
    if not argv or argv[0] in ("-h", "--help"):
        sys.stderr.write(USAGE)
        return 0 if argv else 2
    if argv[0] in ("-V", "--version"):
        sys.stdout.write(SDK_VERSION + "\n"); return 0

    cmd = argv[0]
    rest = argv[1:]
    try:
        if cmd == "init":   return cmd_init(rest)
        if cmd == "build":  return cmd_build(rest)
        if cmd == "deploy": return cmd_deploy(rest)
    except SystemExit as e:
        return int(e.code) if isinstance(e.code, int) else 1
    sys.stderr.write(f"unknown command: {cmd}\n{USAGE}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
