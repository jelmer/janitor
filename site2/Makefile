all: html/index.html html/contact/index.html html/credentials/index.html html/apt/index.html html/lintian-fixes/index.html html/lintian-fixes/merge-proposals.html html/fresh-releases/merge-proposals.html html/fresh-releases/index.html html/fresh-snapshots/index.html html/cupboard/index.html html/cupboard/history.html html/cupboard/queue.html html/cupboard/result-codes/index.html html/cupboard/pkg

html:
	mkdir -p $@

html/index.html html/cupboard/index.html html/apt/index.html html/contact/index.html html/credentials/index.html html/lintian-fixes/index.html: templates/index.html templates/apt.html templates/contact.html templates/credentials.html templates/layout.html templates/lintian-fixes.html templates/cupboard.html
	python3 generate.py

html/lintian-fixes/merge-proposals.html: templates/layout.html templates/merge-proposals.html
	mkdir -p $(@D)
	PYTHONPATH=.. python3 report-merge-proposals.py lintian-fixes > $@

html/fresh-releases/merge-proposals.html: templates/layout.html templates/merge-proposals.html
	mkdir -p $(@D)
	PYTHONPATH=.. python3 report-merge-proposals.py fresh-releases > $@

html/fresh-releases/index.html: templates/layout.html templates/fresh-releases.html
	mkdir -p $(@D)
	PYTHONPATH=.. python3 report-apt-repo.py fresh-releases > $@

html/fresh-snapshots/index.html: templates/layout.html templates/fresh-snapshots.html
	mkdir -p $(@D)
	PYTHONPATH=.. python3 report-apt-repo.py fresh-snapshots > $@

html/cupboard/history.html: templates/layout.html templates/history.html
	mkdir -p $(@D)
	PYTHONPATH=.. python3 report-history.py > $@

html/cupboard/queue.html: templates/layout.html templates/queue.html
	mkdir -p $(@D)
	PYTHONPATH=.. python3 report-queue.py > $@

html/cupboard/result-codes/index.html: templates/layout.html templates/result-code-index.html
	mkdir -p $(@D)
	PYTHONPATH=.. python3 report-result-codes.py $(@D)

html/cupboard/pkg:
	mkdir -p $@
	PYTHONPATH=..:../silver-platter python3 report-pkg.py $@

.PHONY: all html/cupboard/pkg
