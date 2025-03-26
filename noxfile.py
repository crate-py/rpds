from pathlib import Path
from tempfile import TemporaryDirectory
import os

import nox

ROOT = Path(__file__).parent
PYPROJECT = ROOT / "pyproject.toml"
DOCS = ROOT / "docs"
TESTS = ROOT / "tests"

REQUIREMENTS = dict(
    docs=DOCS / "requirements.txt",
    tests=TESTS / "requirements.txt",
)
REQUIREMENTS_IN = [  # this is actually ordered, as files depend on each other
    (path.parent / f"{path.stem}.in", path) for path in REQUIREMENTS.values()
]

SUPPORTED = [
    "3.9",
    "3.10",
    "pypy3.10",
    "3.11",
    "pypy3.11",
    "3.12",
    "3.13",
    "3.13t",
]
LATEST = "3.13"

nox.options.default_venv_backend = "uv|virtualenv"
nox.options.sessions = []


def session(default=True, python=LATEST, **kwargs):  # noqa: D103
    def _session(fn):
        if default:
            nox.options.sessions.append(kwargs.get("name", fn.__name__))
        return nox.session(python=python, **kwargs)(fn)

    return _session


@session(python=SUPPORTED)
def tests(session):
    """
    Run the test suite with a corresponding Python version.
    """
    # Really we want --profile=test here (for
    # https://github.com/crate-py/rpds/pull/87#issuecomment-2291409297)
    # but it produces strange symbol errors saying:
    #   dynamic module does not define module export function (PyInit_rpds)
    # so OK, dev it is.
    session.install(
        "--config-settings",
        "build-args=--profile=dev",
        "--no-cache",
        "-r",
        REQUIREMENTS["tests"],
    )

    if session.posargs and session.posargs[0] == "coverage":
        if len(session.posargs) > 1 and session.posargs[1] == "github":
            github = Path(os.environ["GITHUB_STEP_SUMMARY"])
        else:
            github = None

        session.install("coverage[toml]")
        session.run("coverage", "run", "-m", "pytest", TESTS)
        if github is None:
            session.run("coverage", "report")
        else:
            with github.open("a") as summary:
                summary.write("### Coverage\n\n")
                summary.flush()  # without a flush, output seems out of order.
                session.run(
                    "coverage",
                    "report",
                    "--format=markdown",
                    stdout=summary,
                )
    else:
        session.run("pytest", "--parallel-threads=10", *session.posargs, TESTS)


@session()
def audit(session):
    """
    Audit dependencies for vulnerabilities.
    """
    session.install("pip-audit", ROOT)
    session.run("python", "-m", "pip_audit")


@session(tags=["build"])
def build(session):
    """
    Build a distribution suitable for PyPI and check its validity.
    """
    session.install("build", "twine")
    with TemporaryDirectory() as tmpdir:
        session.run("python", "-m", "build", ROOT, "--outdir", tmpdir)
        session.run("twine", "check", "--strict", tmpdir + "/*")


@session(tags=["style"])
def style(session):
    """
    Check Python code style.
    """
    session.install("ruff")
    session.run("ruff", "check", TESTS, __file__)


@session()
def typing(session):
    """
    Check the codebase using pyright by type checking the test suite.
    """
    session.install("pyright", ROOT, "-r", REQUIREMENTS["tests"])
    session.run("pyright", TESTS)


@session(tags=["docs"])
@nox.parametrize(
    "builder",
    [
        nox.param(name, id=name)
        for name in [
            "dirhtml",
            "doctest",
            "linkcheck",
            "man",
            "spelling",
        ]
    ],
)
def docs(session, builder):
    """
    Build the documentation using a specific Sphinx builder.
    """
    session.install("-r", REQUIREMENTS["docs"])
    with TemporaryDirectory() as tmpdir_str:
        tmpdir = Path(tmpdir_str)
        argv = ["-n", "-T", "-W"]
        if builder != "spelling":
            argv += ["-q"]
        posargs = session.posargs or [tmpdir / builder]
        session.run(
            "python",
            "-m",
            "sphinx",
            "-b",
            builder,
            DOCS,
            *argv,
            *posargs,
        )


@session(tags=["docs", "style"], name="docs(style)")
def docs_style(session):
    """
    Check the documentation style.
    """
    session.install(
        "doc8",
        "pygments",
        "pygments-github-lexers",
    )
    session.run("python", "-m", "doc8", "--config", PYPROJECT, DOCS)


@session(default=False)
def requirements(session):
    """
    Update the project's pinned requirements.

    You should commit the result afterwards.
    """
    if session.venv_backend == "uv":
        cmd = ["uv", "pip", "compile"]
    else:
        session.install("pip-tools")
        cmd = ["pip-compile", "--resolver", "backtracking", "--strip-extras"]

    for each, out in REQUIREMENTS_IN:
        # otherwise output files end up with silly absolute path comments...
        relative = each.relative_to(ROOT)
        session.run(*cmd, "--upgrade", "--output-file", out, relative)
