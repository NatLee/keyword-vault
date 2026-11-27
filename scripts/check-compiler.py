#!/usr/bin/env python3
"""Exercise the public CLI contract without starting the Vault application."""
import json
import os
import subprocess

env = {**os.environ, "CARGO_BUILD_JOBS": "2"}
base = ["cargo", "run", "--quiet", "--locked", "-p", "es-keyword-query", "--example", "inspect", "--"]

def run(args, status):
    result = subprocess.run(base + args, env=env, capture_output=True, text=True)
    assert result.returncode == status, result.stderr
    return json.loads(result.stdout) if result.stdout.strip() else None

assert run(["(a|b)&c"], 0)["query_string"] == '("a" OR "b") AND "c"'
assert run(["a*", "--raw"], 0)["query_string"] == "a*"
assert run(["(a b ~)", "--slop=5"], 0)["query_string"] == '("a b"~5)'
assert run(["a&&b"], 1)["valid"] is False
run(["a", "--unknown"], 2)
print("Compiler CLI contract: 5 cases passed")
