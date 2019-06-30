PYTHONPATH=..:../lintian-brush:../silver-platter:../breezy
export PYTHONPATH
OUTPUT_DIR = ../public_html
TEMPLATESDIR = ../janitor/site/templates/

all: $(OUTPUT_DIR)/index.html $(OUTPUT_DIR)/contact/index.html $(OUTPUT_DIR)/credentials/index.html $(OUTPUT_DIR)/apt/index.html $(OUTPUT_DIR)/lintian-fixes/index.html $(OUTPUT_DIR)/lintian-fixes/merge-proposals.html $(OUTPUT_DIR)/fresh-releases/merge-proposals.html $(OUTPUT_DIR)/fresh-releases/index.html $(OUTPUT_DIR)/fresh-snapshots/index.html $(OUTPUT_DIR)/cupboard/index.html $(OUTPUT_DIR)/cupboard/history.html $(OUTPUT_DIR)/cupboard/queue.html $(OUTPUT_DIR)/cupboard/result-codes/index.html $(OUTPUT_DIR)/cupboard/pkg

html:
	mkdir -p $@

$(OUTPUT_DIR)/index.html $(OUTPUT_DIR)/cupboard/index.html $(OUTPUT_DIR)/apt/index.html $(OUTPUT_DIR)/contact/index.html $(OUTPUT_DIR)/credentials/index.html $(OUTPUT_DIR)/lintian-fixes/index.html: $(TEMPLATESDIR)/index.html $(TEMPLATESDIR)/apt.html $(TEMPLATESDIR)/contact.html $(TEMPLATESDIR)/credentials.html $(TEMPLATESDIR)/layout.html $(TEMPLATESDIR)/lintian-fixes-start.html $(TEMPLATESDIR)/cupboard.html
	python3 -m janitor.site.generate $(OUTPUT_DIR)

$(OUTPUT_DIR)/lintian-fixes/merge-proposals.html: $(TEMPLATESDIR)/layout.html $(TEMPLATESDIR)/merge-proposals.html
	mkdir -p $(@D)
	python3 -m janitor.site.merge_proposals lintian-fixes > $@

$(OUTPUT_DIR)/fresh-releases/merge-proposals.html: $(TEMPLATESDIR)/layout.html $(TEMPLATESDIR)/merge-proposals.html
	mkdir -p $(@D)
	python3 -m janitor.site.merge_proposals fresh-releases > $@

$(OUTPUT_DIR)/fresh-releases/index.html: $(TEMPLATESDIR)/layout.html $(TEMPLATESDIR)/fresh-releases.html
	mkdir -p $(@D)
	python3 -m janitor.site.apt_repo fresh-releases > $@

$(OUTPUT_DIR)/fresh-snapshots/index.html: $(TEMPLATESDIR)/layout.html $(TEMPLATESDIR)/fresh-snapshots.html
	mkdir -p $(@D)
	python3 -m janitor.site.apt_repo fresh-snapshots > $@

$(OUTPUT_DIR)/cupboard/history.html: $(TEMPLATESDIR)/layout.html $(TEMPLATESDIR)/history.html
	mkdir -p $(@D)
	python3 -m janitor.site.history > $@

$(OUTPUT_DIR)/cupboard/queue.html: $(TEMPLATESDIR)/layout.html $(TEMPLATESDIR)/queue.html
	mkdir -p $(@D)
	python3 -m janitor.site.queue > $@

$(OUTPUT_DIR)/cupboard/result-codes/index.html: $(TEMPLATESDIR)/layout.html $(TEMPLATESDIR)/result-code-index.html
	mkdir -p $(@D)
	python3 -m janitor.site.result_codes $(@D)

$(OUTPUT_DIR)/cupboard/pkg:
	mkdir -p $@
	python3 -m janitor.site.pkg $(abspath pkg) $@

.PHONY: all $(OUTPUT_DIR)/cupboard/pkg
