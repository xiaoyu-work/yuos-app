"""Narrow test double for the unpublished `yuos_sdk.files` broker.

It checks only the adapter contract Yu Office relies on (literal path, host
binary, call deadline/cancellation, opaque revision, one replace) and backs
the file with a test-private path. It is not an OS broker and is not staged.
"""

import hashlib
from pathlib import Path
from types import SimpleNamespace


class FakeBroker:
    BINARY = "/usr/local/bin/yu"

    def __init__(self, deadline_unix_ms=None):
        self.deadline_unix_ms = deadline_unix_ms
        self.calls = []
        self.deny = set()

    def _check(self, verb, path, binary, deadline_unix_ms, check_cancelled):
        assert binary == self.BINARY and deadline_unix_ms == self.deadline_unix_ms
        check_cancelled()
        self.calls.append((verb, path))
        if verb in self.deny:
            raise PermissionError(f"{verb} denied for {path}")

    @staticmethod
    def _revision(path):
        return "broker:" + hashlib.sha256(Path(path).read_bytes()).hexdigest()

    def read_file(self, path, *, binary, deadline_unix_ms, check_cancelled):
        self._check("read", path, binary, deadline_unix_ms, check_cancelled)
        return SimpleNamespace(content=Path(path).read_bytes(), revision=self._revision(path))

    def replace_file(self, path, revision, data, *, binary, deadline_unix_ms, check_cancelled):
        self._check("replace", path, binary, deadline_unix_ms, check_cancelled)
        if revision != self._revision(path):
            raise RuntimeError("stale broker revision")
        Path(path).write_bytes(data)

    def install(self, monkeypatch, server):
        monkeypatch.setattr(server, "_broker", lambda: (self.read_file, self.replace_file))
        context = SimpleNamespace(deadline_unix_ms=self.deadline_unix_ms, raise_if_cancelled=lambda: None)
        monkeypatch.setattr(server, "current_context", lambda: context)
        return self
