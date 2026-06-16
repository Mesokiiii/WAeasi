"""waeasi.web.multipart — multipart/form-data parser.

Streaming-aware parser sufficient for typical AI ingest endpoints
(file uploads, transcription audio, image-to-text inputs).  Operates
on the already-buffered request body — for very large uploads users
should stream directly to blob storage instead.

Output is a list of ``Part`` records each carrying:
  * name      — form field name
  * filename  — original filename or None
  * content_type
  * headers   — full Headers
  * data      — bytes
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Optional

from ..runtime.headers import Headers
from .errors import BadRequest, UnprocessableEntity


@dataclass
class Part:
    name: str
    filename: Optional[str]
    content_type: str
    headers: Headers
    data: bytes = field(repr=False)


def parse_content_type(h: str) -> tuple[str, dict[str, str]]:
    """Split ``application/x-foo; charset=utf-8; boundary=xyz`` → (mime, params)."""
    main, _, rest = h.partition(";")
    mime = main.strip().lower()
    params: dict[str, str] = {}
    for kv in rest.split(";"):
        kv = kv.strip()
        if not kv: continue
        k, _, v = kv.partition("=")
        v = v.strip()
        if v.startswith('"') and v.endswith('"'):
            v = v[1:-1]
        params[k.strip().lower()] = v
    return mime, params


def parse(body: bytes, content_type: str) -> list[Part]:
    mime, params = parse_content_type(content_type)
    if mime != "multipart/form-data":
        raise BadRequest(f"expected multipart/form-data, got {mime}")
    boundary = params.get("boundary")
    if not boundary:
        raise BadRequest("missing boundary parameter")
    sep = ("--" + boundary).encode("ascii")
    end_sep = sep + b"--"

    parts: list[Part] = []
    # Split body on `--<boundary>\r\n`.
    chunks = body.split(sep)
    if len(chunks) < 2:
        raise UnprocessableEntity("body is not multipart")

    for chunk in chunks[1:]:
        if not chunk:
            continue
        if chunk.startswith(b"--"):
            break  # closing delimiter
        # Each chunk: \r\n<header lines>\r\n\r\n<data>\r\n
        chunk = chunk.lstrip(b"\r\n")
        sep_idx = chunk.find(b"\r\n\r\n")
        if sep_idx == -1:
            raise UnprocessableEntity("malformed part: no header terminator")
        header_block = chunk[:sep_idx].decode("utf-8", errors="replace")
        data = chunk[sep_idx + 4:]
        # Remove trailing CRLF before next boundary
        if data.endswith(b"\r\n"):
            data = data[:-2]

        headers = _parse_headers(header_block)
        cd = headers.get("content-disposition") or ""
        if "form-data" not in cd.lower():
            continue
        _, cd_params = parse_content_type(cd)
        name = cd_params.get("name")
        if name is None:
            raise UnprocessableEntity("part missing 'name' parameter")
        parts.append(Part(
            name=name,
            filename=cd_params.get("filename"),
            content_type=headers.get("content-type") or "application/octet-stream",
            headers=headers,
            data=data,
        ))
    return parts


_HEADER_LINE = re.compile(r"^([^:]+):\s*(.*)$")


def _parse_headers(block: str) -> Headers:
    h = Headers()
    for line in block.split("\r\n"):
        if not line.strip(): continue
        m = _HEADER_LINE.match(line)
        if not m:
            raise UnprocessableEntity(f"malformed header line: {line!r}")
        h.append(m.group(1).strip(), m.group(2).strip())
    return h


def field_dict(parts: list[Part]) -> dict[str, list[Part]]:
    """Group parts by name (preserving order within a name)."""
    out: dict[str, list[Part]] = {}
    for p in parts:
        out.setdefault(p.name, []).append(p)
    return out
