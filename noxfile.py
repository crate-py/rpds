from pathlib import Path
from tempfile import TemporaryDirectory

import nox

ROOT = Path(__file__).parent
TESTS = ROOT / "tests"
PYPROJECT = ROOT / "pyproject.toml"


nox.options.sessions = []


def session(default=True, **kwargs):
    def _session(fn):
        if default:
            nox.options.sessions.append(kwargs.get("name", fn.__name__))
        return nox.session(**kwargs)(fn)

    return _session


@session(python=["3.8", "3.9", "3.10", "3.11", "3.12", "pypy3"])
def tests(session):
    session.install(ROOT, "-r", TESTS / "requirements.txt")
    if session.posargs == ["coverage"]:
        session.install("coverage[toml]")
        session.run("coverage", "run", "-m", "pytest")
        session.run("coverage", "report")
    else:
        session.run("pytest", *session.posargs, TESTS)


@session(tags=["build"])
def build(session):
    session.install("build", "twine")
    with TemporaryDirectory() as tmpdir:
        session.run("python", "-m", "build", ROOT, "--outdir", tmpdir)
        session.run("twine", "check", "--strict", tmpdir + "/*")
