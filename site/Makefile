# Minimal makefile for Sphinx documentation
#

# You can set these variables from the command line.
SPHINXOPTS    =
SPHINXBUILD   = sphinx-build
SPHINXPROJ    = debian-janitor
SOURCEDIR     = .
BUILDDIR      = _build

# Put it first so that "make" without argument is like "make help".
help:
	@$(SPHINXBUILD) -M help "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)

prereqs: lintian-brush-tags.txt status.rst

lintian-brush-tags.txt::
	lintian-brush --list-tags 2>&1 | sed -e 's/^\(.*\)/\* `\1 <https:\/\/lintian.debian.org\/tags\/\1.html>`_/' > $@

status.rst:
	python3 ../report-state.py > $@

history.rst:
	python3 ../report-history.py > $@

.PHONY: help Makefile

# Catch-all target: route all unknown targets to Sphinx using the new
# "make mode" option.  $(O) is meant as a shortcut for $(SPHINXOPTS).
%: Makefile
	@$(SPHINXBUILD) -M $@ "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)
