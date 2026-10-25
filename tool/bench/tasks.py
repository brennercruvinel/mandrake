#!/usr/bin/env python3
"""Real tasks, without and with Mandrake: success, quality, tool calls, tokens and cost.

Each task in tool/bench/tasks/<name>/ is a small repository (repo/), a prompt (task.toml) and a
check (check.sh) that prints one rubric line per item; the first item is success, the rest quality.

  tool/bench/tasks.py verify             spends nothing: every check fails on the untouched repo
                                         and passes on the reference solution
  tool/bench/tasks.py run --yes [-n N]   each task N times (default 1) through `claude -p`, without
                                         Mandrake and with it; spends 2 x N sessions per task
  tool/bench/tasks.py run --only a,b     some tasks
  tool/bench/tasks.py run --api URL      every session against a stand-in API (spends nothing; the
                                         tasks fail, the pipeline and the report are what it shows)

`run` uses the Claude Code login of whoever runs it. Without Mandrake means no prompt-trim and no
proxy; with it, the installed prompt-trim (--trim to choose another). The report goes to
$XDG_STATE_HOME/mandrake/bench/tasks-<stamp>/ (report.json and report.md).
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
TASKS = HERE / "tasks"
SIDES = ("sem", "com")
# Git in a task folder answers as nobody in particular, and never signs with the user's key.
GIT_ENV = {
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_AUTHOR_NAME": "bench",
    "GIT_AUTHOR_EMAIL": "bench@localhost",
    "GIT_COMMITTER_NAME": "bench",
    "GIT_COMMITTER_EMAIL": "bench@localhost",
}


def tasks(only):
    names = sorted(p.name for p in TASKS.iterdir() if (p / "task.toml").is_file())
    if only:
        missing = set(only) - set(names)
        if missing:
            sys.exit(
                f"no such task: {', '.join(sorted(missing))} (have: {', '.join(names)})"
            )
        names = [n for n in names if n in only]
    return [(n, tomllib.loads((TASKS / n / "task.toml").read_text())) for n in names]


def sh(cmd, cwd, env=None):
    return subprocess.run(
        ["bash", "-c", cmd],
        check=False,
        cwd=cwd,
        env={**os.environ, **GIT_ENV, **(env or {})},
        capture_output=True,
        text=True,
    )


def prepare(name, spec, root):
    """The work folder the session changes, and an untouched copy to compare with."""
    w, orig = root / "w", root / "orig"
    for d in (w, orig):
        shutil.copytree(TASKS / name / "repo", d, symlinks=True)
        if spec.get("setup"):
            r = sh(spec["setup"], d)
            if r.returncode:
                sys.exit(f"{name}: setup failed: {r.stderr.strip()}")
    return w, orig


def check(name, w, orig, answer=""):
    """[(passed, item)], the first one being success."""
    env = {"task": str(TASKS / name), "w": str(w), "orig": str(orig), "answer": answer}
    r = sh(f'. "{HERE}/lib.sh"; . "{TASKS / name}/check.sh"', w, env)
    items = [
        (line[:4] == "pass", line[5:])
        for line in r.stdout.splitlines()
        if line[:4] in ("pass", "fail")
    ]
    return items or [(False, f"check printed nothing: {r.stderr.strip()[:200]}")]


def solve(name, spec, w):
    sol = TASKS / name / "solution"
    if sol.is_dir():
        shutil.copytree(sol, w, dirs_exist_ok=True)
    if spec.get("solution"):
        sh(spec["solution"], w)
    return spec.get("answer", "")


def verify(only):
    bad = 0
    for name, spec in tasks(only):
        with tempfile.TemporaryDirectory() as t:
            w, orig = prepare(name, spec, Path(t))
            before = check(name, w, orig)
            answer = solve(name, spec, w)
            after = check(name, w, orig, answer)
        ok = not before[0][0] and all(p for p, _ in after)
        bad += not ok
        print(
            f"{'ok  ' if ok else 'FAIL'}  {name}: untouched {'passes' if before[0][0] else 'fails'}, "
            f"solution {sum(p for p, _ in after)}/{len(after)}"
        )
        for p, item in after if not ok else []:
            print(f"        {'pass' if p else 'fail'} {item}")
    print(
        f"\nverify: {'every task is sound' if not bad else f'{bad} task(s) with a broken check'}"
    )
    return 1 if bad else 0


def session(spec, w, trim, timeout, api=None):
    """Runs one `claude -p` in the work folder; returns what the stream said."""
    settings = {"env": {"CLAUDE_CODE_PLUGIN_DIRS": trim}}
    cmd = [
        "claude",
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--max-turns",
        str(spec.get("max_turns", 30)),
        "--settings",
        json.dumps(settings),
        "--allowedTools",
        *spec.get("allow", []),
        "--",
        spec["prompt"],
    ]
    env = {
        k: v
        for k, v in os.environ.items()
        if k not in ("ANTHROPIC_BASE_URL", "MANDRAKE_SOUL", "GH_CONFIG_DIR")
    }
    env.update(GIT_ENV, MANDRAKE_PROXY="0")
    if api:
        env.update(
            ANTHROPIC_BASE_URL=api, ANTHROPIC_API_KEY="sk-ant-bench-local-placeholder"
        )
    out = {
        "tools": 0,
        "turns": 0,
        "cost_usd": 0.0,
        "answer": "",
        "error": None,
        "tokens": {"input": 0, "output": 0, "cache_read": 0, "cache_write": 0},
    }
    start = time.monotonic()
    try:
        p = subprocess.run(
            cmd,
            check=False,
            cwd=w,
            env=env,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        out["error"] = f"passou de {timeout} s"
        return out
    out["seconds"] = round(time.monotonic() - start, 1)
    for line in p.stdout.splitlines():
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            continue
        if ev.get("type") == "assistant":
            out["tools"] += sum(
                1
                for b in ev.get("message", {}).get("content", [])
                if b.get("type") == "tool_use"
            )
        elif ev.get("type") == "result":
            u = ev.get("usage", {})
            out["tokens"] = {
                "input": u.get("input_tokens", 0),
                "output": u.get("output_tokens", 0),
                "cache_read": u.get("cache_read_input_tokens", 0),
                "cache_write": u.get("cache_creation_input_tokens", 0),
            }
            out.update(
                turns=ev.get("num_turns", 0),
                cost_usd=ev.get("total_cost_usd", 0.0),
                answer=ev.get("result", "") or "",
            )
            if ev.get("is_error"):
                out["error"] = ev.get("subtype", "error")
    if p.returncode and not out["error"]:
        out["error"] = (p.stderr.strip().splitlines() or ["claude -p falhou"])[-1][:200]
    return out


def installed_trim():
    base = (
        Path(os.environ.get("MANDRAKE_HOME", "")) / "data"
        if os.environ.get("MANDRAKE_HOME")
        else Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share"))
        / "mandrake"
    )
    return base / "src/data/plugins/prompt-trim"


def run(only, reps, trim, timeout, api=None):
    if not (trim / "hooks/register.js").is_file():
        sys.exit(
            f"prompt-trim não instalado em {trim}: rode mandrake setup, ou passe --trim"
        )
    if "CORE = ''" in (trim / "hooks/core.js").read_text():
        print(
            f"aviso: o core está vazio em {trim}; o lado com Mandrake roda sem ele",
            file=sys.stderr,
        )
    state = (
        Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state"))
        / "mandrake/bench"
    )
    out_dir = state / time.strftime("tasks-%Y%m%d-%H%M%S")
    out_dir.mkdir(parents=True)
    rows = []
    for name, spec in tasks(only):
        for rep in range(reps):
            for side in SIDES:
                with tempfile.TemporaryDirectory() as t:
                    w, orig = prepare(name, spec, Path(t))
                    s = session(
                        spec, w, str(trim) if side == "com" else "", timeout, api
                    )
                    items = check(name, w, orig, s["answer"])
                row = {
                    "task": name,
                    "rep": rep,
                    "side": side,
                    "success": items[0][0],
                    "quality": [p for p, _ in items[1:]],
                    "items": [i for _, i in items],
                    **s,
                }
                rows.append(row)
                print(
                    f"{name:<18} {side}  {'ok  ' if row['success'] else 'fail'}  "
                    f"q {sum(row['quality'])}/{len(row['quality'])}  tools {s['tools']:>2}  "
                    f"tokens {sum(s['tokens'].values()):>7}  ${s['cost_usd']:.4f}"
                    + (f"  {s['error']}" if s["error"] else ""),
                    flush=True,
                )
    (out_dir / "report.json").write_text(json.dumps(rows, indent=2, ensure_ascii=False))
    (out_dir / "report.md").write_text(summary(rows))
    print("\n" + summary(rows) + f"\nrelatório: {out_dir}")


def summary(rows):
    lines = ["| | sem Mandrake | com Mandrake |", "| - | - | - |"]
    by = {side: [r for r in rows if r["side"] == side] for side in SIDES}

    def cell(side, f):
        rs = by[side]
        return f(rs) if rs else "-"

    def pct(rs):
        return f"{100 * sum(r['success'] for r in rs) / len(rs):.0f}% ({sum(r['success'] for r in rs)}/{len(rs)})"

    def qual(rs):
        # Quality counts only where the task succeeded: an untouched repo passes "changed nothing else".
        q = [x for r in rs if r["success"] for x in r["quality"]]
        return f"{100 * sum(q) / len(q):.0f}%" if q else "-"

    metrics = [
        ("sucesso", pct),
        ("qualidade (rubrica, onde deu certo)", qual),
        ("chamadas de ferramenta", lambda rs: str(sum(r["tools"] for r in rs))),
        ("turnos", lambda rs: str(sum(r["turns"] for r in rs))),
        (
            "tokens de entrada (com cache)",
            lambda rs: (
                f"{sum(r['tokens']['input'] + r['tokens']['cache_read'] + r['tokens']['cache_write'] for r in rs):,}"
            ),
        ),
        ("tokens de saída", lambda rs: f"{sum(r['tokens']['output'] for r in rs):,}"),
        (
            "custo (USD, o que o Claude Code informa)",
            lambda rs: f"{sum(r['cost_usd'] for r in rs):.4f}",
        ),
        ("erros", lambda rs: str(sum(1 for r in rs if r["error"]))),
    ]
    lines += [
        f"| {label} | {cell('sem', f)} | {cell('com', f)} |" for label, f in metrics
    ]
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("what", choices=["verify", "run"])
    ap.add_argument("--only", default="", help="task names, comma-separated")
    ap.add_argument("-n", type=int, default=1, help="runs per task and side")
    ap.add_argument(
        "--trim",
        type=Path,
        default=installed_trim(),
        help="the prompt-trim folder for the side with Mandrake",
    )
    ap.add_argument("--timeout", type=int, default=600, help="seconds per session")
    ap.add_argument(
        "--yes", action="store_true", help="run spends real sessions: say yes"
    )
    ap.add_argument("--api", help="a stand-in API for every session; spends nothing")
    a = ap.parse_args()
    only = [x for x in a.only.split(",") if x]
    if a.what == "verify":
        return verify(only)
    n = len(tasks(only)) * a.n * 2
    if not a.yes and not a.api:
        print(
            f"run gasta {n} sessões reais do Claude Code ({len(tasks(only))} tarefas x {a.n} x 2 lados). Repita com --yes."
        )
        return 2
    run(only, a.n, a.trim, a.timeout, a.api)
    return 0


if __name__ == "__main__":
    sys.exit(main())
