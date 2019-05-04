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

pkg:
	python3 ../report-pkg.py pkg/

history.rst:
	python3 ../report-history.py > $@

queue.rst:
	python3 ../report-queue.py > $@

upstream-releases:
	python3 ../report-apt-repo.py upstream-releases

upstream-snapshots:
	python3 ../report-apt-repo.py upstream-snapshots

lintian-fixes:
	python3 ../report-apt-repo.py lintian-fixes

cupboard/result-codes:
	python3 ../report-result-codes.py

logs:
	for I in $$(find pkg -name '*.log' -o -name 'result.json'); do { mkdir -p _build/html/$$(dirname $$I); ln -f $$I _build/html/$$I; } done

.PHONY: help Makefile pkg upstream-releases upstream-snapshots

# Catch-all target: route all unknown targets to Sphinx using the new
# "make mode" option.  $(O) is meant as a shortcut for $(SPHINXOPTS).
%: Makefile
	@$(SPHINXBUILD) -M $@ "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)
