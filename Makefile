export PYTHONPATH=$(shell pwd)/ognibuild:$(shell pwd)/buildlog-consultant:$(shell pwd):$(shell pwd)/breezy:$(shell pwd)/silver-platter:$(shell pwd)/lintian-brush:$(shell pwd)/python-debian/lib:$(shell pwd)/debmutate:$(shell pwd)/dulwich
export BRZ_PLUGINS_AT=debian@$(shell pwd)/breezy-debian

PB2_PY_OUTPUT = janitor/config_pb2.py janitor/package_metadata_pb2.py

core: janitor/site/_static/pygments.css $(PB2_PY_OUTPUT)

all: core

.PHONY: all check

PROTOC_ARGS = --python_out=.

ifneq (,$(shell which protoc-gen-mypy))
PROTOC_ARGS += --mypy_out=.
endif

janitor/%_pb2.py: janitor/%.proto
	protoc $(PROTOC_ARGS) $<

check:: typing

check:: test

check:: style

suite-references:
	git grep "\\(lintian-brush\|lintian-fixes\|debianize\|fresh-releases\|fresh-snapshots\\)" | grep -v .example

test:
	BRZ_PLUGINS_AT=$(BRZ_PLUGINS_AT) PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python PYTHONPATH=$(PYTHONPATH) pytest janitor

style:: flake8

flake8:
	flake8

style:: yamllint

yamllint:
	yamllint .github/ ansible/

style:: djlint

djlint:
	djlint -i J018,H030,H031,H021 --profile jinja janitor/site/templates

typing:
	mypy janitor

janitor/site/_static/pygments.css:
	pygmentize -S default -f html > $@

clean:
	rm -f $(PB2_PY_OUTPUT)

SHA=$(shell git rev-parse HEAD)

docker-%: core
	buildah build -t ghcr.io/jelmer/janitor/$*:latest -t ghcr.io/jelmer/janitor/$*:$(SHA) -f Dockerfile_$* .
	buildah push ghcr.io/jelmer/janitor/$*:latest
	buildah push ghcr.io/jelmer/janitor/$*:$(SHA)

docker-all: docker-base docker-site docker-runner docker-publish docker-archive docker-worker docker-git_store docker-bzr_store docker-irc_notify docker-mastodon_notify docker-xmpp_notify docker-differ docker-ognibuild_dep

reformat:: reformat-html

reformat-html:
	djlint --reformat --format-css janitor/site/templates/
