"""waeasi.build — build pipeline public surface."""

from .bundle import BundleArtifact, BundleEntry, assemble, inspect
from .compose import ComposeInput, ComposeResult, compose, verify_digest
from .componentize import ComponentizeInput, ComponentizeResult, componentize, is_component
from .errors import (
    BuildError, BundleError, ConfigError, SignatureError, ToolchainError,
)
from .manifest import ManifestInput, ManifestResult, ManifestSpec, write_manifest
from .pipeline import (
    BuildOptions, BuildResult, StageTiming, build_all, format_result,
)
from .sign import SignInput, SignResult, sign
from .wizer import WizerInput, WizerResult, passthrough, wizer

__all__ = [
    "BuildError", "BundleError", "ConfigError", "SignatureError", "ToolchainError",
    "BuildOptions", "BuildResult", "StageTiming",
    "BundleArtifact", "BundleEntry",
    "ComposeInput", "ComposeResult",
    "ComponentizeInput", "ComponentizeResult",
    "ManifestInput", "ManifestResult", "ManifestSpec",
    "SignInput", "SignResult",
    "WizerInput", "WizerResult",
    "assemble", "inspect", "compose", "verify_digest",
    "componentize", "is_component",
    "write_manifest", "sign", "wizer", "passthrough",
    "build_all", "format_result",
]
