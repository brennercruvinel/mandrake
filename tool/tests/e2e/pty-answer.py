"""Runs ./install.sh --cli --from <profile> in a pseudo-terminal and answers each `[default]: `
prompt like a person: with the bracketed default, or with the answer given for a prompt substring.
    pty-answer.py <src> <profile> ["substring=answer;..."]
"""
import os, pty, sys, re, select
src, prof, answers = sys.argv[1], sys.argv[2], sys.argv[3]
special = [tuple(p.split("=", 1)) for p in answers.split(";") if "=" in p]
pid, fd = pty.fork()
if pid == 0:
    os.chdir(src); os.execvp("bash", ["bash", "./install.sh", "--cli", "--from", prof])
buf = b""; out = sys.stdout.buffer
while True:
    r, _, _ = select.select([fd], [], [], 300)
    if not r: print("\n  ERRO install.sh sem saída por 300 s"); break
    try: chunk = os.read(fd, 4096)
    except OSError: break
    if not chunk: break
    out.write(chunk); out.flush(); buf += chunk
    tail = buf.decode("utf8", "replace").rsplit("\n", 1)[-1]
    m = re.search(r"\[([^\]]*)\]: $", tail)
    if m:
        # Type the bracketed default (or the given answer), so the log shows every answer.
        reply = next((a for q, a in special if q in tail), m.group(1))
        os.write(fd, (reply + "\r").encode()); buf = b""
_, st = os.waitpid(pid, 0)
sys.exit(os.waitstatus_to_exitcode(st))
