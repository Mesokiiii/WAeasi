"""waeasi.build.errors — typed exception hierarchy used across stages."""

from __future__ import annotations


class BuildError(Exception):
    """Top-level error raised by any build stage.

    Carries a human-readable message; the CLI prints it verbatim.
    Lower-level cause chains are preserved via ``__cause__``.
    """


class ConfigError(BuildError):
    """User-facing config-file error (missing field, bad type, etc.)."""


class ToolchainError(BuildError):
    """An external binary (componentize-py, wizer, wasm-tools) failed
    or was not found."""


class SignatureError(BuildError):
    """Key load / signing failure."""


class BundleError(BuildError):
    """.waeasi-bundle assembly or layout invariant violated."""
