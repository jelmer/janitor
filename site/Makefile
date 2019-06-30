PYTHONPATH=..:../lintian-brush:../silver-platter:../breezy
export PYTHONPATH
OUTPUT_DIR = ../public_html

all: $(OUTPUT_DIR)/index.html $(OUTPUT_DIR)/contact/index.html $(OUTPUT_DIR)/credentials/index.html $(OUTPUT_DIR)/apt/index.html $(OUTPUT_DIR)/lintian-fixes/index.html $(OUTPUT_DIR)/lintian-fixes/merge-proposals.html $(OUTPUT_DIR)/fresh-releases/merge-proposals.html $(OUTPUT_DIR)/fresh-releases/index.html $(OUTPUT_DIR)/fresh-snapshots/index.html $(OUTPUT_DIR)/cupboard/index.html $(OUTPUT_DIR)/cupboard/history.html $(OUTPUT_DIR)/cupboard/queue.html $(OUTPUT_DIR)/cupboard/result-codes/index.html $(OUTPUT_DIR)/cupboard/pkg

html:
	mkdir -p $@

$(OUTPUT_DIR)/index.html $(OUTPUT_DIR)/cupboard/index.html $(OUTPUT_DIR)/apt/index.html $(OUTPUT_DIR)/contact/index.html $(OUTPUT_DIR)/credentials/index.html $(OUTPUT_DIR)/lintian-fixes/index.html: templates/index.html templates/apt.html templates/contact.html templates/credentials.html templates/layout.html templates/lintian-fixes-start.html templates/cupboard.html
	python3 generate.py $(OUTPUT_DIR)

$(OUTPUT_DIR)/lintian-fixes/merge-proposals.html: templates/layout.html templates/merge-proposals.html
	mkdir -p $(@D)
	python3 report-merge-proposals.py lintian-fixes > $@

$(OUTPUT_DIR)/fresh-releases/merge-proposals.html: templates/layout.html templates/merge-proposals.html
	mkdir -p $(@D)
	python3 report-merge-proposals.py fresh-releases > $@

$(OUTPUT_DIR)/fresh-releases/index.html: templates/layout.html templates/fresh-releases.html
	mkdir -p $(@D)
	python3 report-apt-repo.py fresh-releases > $@

$(OUTPUT_DIR)/fresh-snapshots/index.html: templates/layout.html templates/fresh-snapshots.html
	mkdir -p $(@D)
	python3 report-apt-repo.py fresh-snapshots > $@

$(OUTPUT_DIR)/cupboard/history.html: templates/layout.html templates/history.html
	mkdir -p $(@D)
	python3 report-history.py > $@

$(OUTPUT_DIR)/cupboard/queue.html: templates/layout.html templates/queue.html
	mkdir -p $(@D)
	python3 report-queue.py > $@

$(OUTPUT_DIR)/cupboard/result-codes/index.html: templates/layout.html templates/result-code-index.html
	mkdir -p $(@D)
	python3 report-result-codes.py $(@D)

$(OUTPUT_DIR)/cupboard/pkg:
	mkdir -p $@
	python3 report-pkg.py $(abspath pkg) $@

.PHONY: all $(OUTPUT_DIR)/cupboard/pkg
