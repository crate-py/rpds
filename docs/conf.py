import importlib.metadata
import re

from url import URL

GITHUB = URL.parse("https://github.com/")
HOMEPAGE = GITHUB / "crate-py/rpds"

project = "rpds.py"
author = "Julian Berman"
copyright = f"2023, {author}"

release = importlib.metadata.version("rpds.py")
version = release.partition("-")[0]

language = "en"
default_role = "any"

extensions = [
    "sphinx.ext.autodoc",
    "sphinx.ext.autosectionlabel",
    "sphinx.ext.coverage",
    "sphinx.ext.doctest",
    "sphinx.ext.extlinks",
    "sphinx.ext.intersphinx",
    "sphinx.ext.napoleon",
    "sphinx.ext.todo",
    "sphinx.ext.viewcode",
    "sphinx_copybutton",
    "sphinxcontrib.spelling",
    "sphinxext.opengraph",
]

pygments_style = "lovelace"
pygments_dark_style = "one-dark"

html_theme = "furo"


def entire_domain(host):
    return r"http.?://" + re.escape(host) + r"($|/.*)"


linkcheck_ignore = [
    entire_domain("img.shields.io"),
    f"{GITHUB}.*#.*",
    str(HOMEPAGE / "actions"),
    str(HOMEPAGE / "workflows/CI/badge.svg"),
]

# = Extensions =

# -- autodoc --

autodoc_default_options = {
    "members": True,
    "member-order": "bysource",
}

# -- autosectionlabel --

autosectionlabel_prefix_document = True

# -- intersphinx --

intersphinx_mapping = {
    "python": ("https://docs.python.org/", None),
}

# -- extlinks --

extlinks = {
    "gh": (str(HOMEPAGE) + "/%s", None),
    "github": (str(GITHUB) + "/%s", None),
}
extlinks_detect_hardcoded_links = True

# -- sphinx-copybutton --

copybutton_prompt_text = r">>> |\.\.\. |\$"
copybutton_prompt_is_regexp = True

# -- sphinxcontrib-spelling --

spelling_word_list_filename = "spelling-wordlist.txt"
spelling_show_suggestions = True
