DOCKER_TAG ?= latest
PYTHON ?= python3

core: janitor/site/_static/pygments.css build-inplace

build-inplace:
	$(PYTHON) setup.py build_ext -i
	$(PYTHON) setup.py build_protobuf

all: core

.PHONY: all check

check:: typing

check:: test

check:: style

check:: ruff

ruff:
	ruff check .

fix:: ruff-fix

fix:: cargo-fix

cargo-fix:
	cargo clippy --fix

ruff-fix:
	ruff check --fix .

suite-references:
	git grep "\\(lintian-brush\|lintian-fixes\|debianize\|fresh-releases\|fresh-snapshots\\)" | grep -v .example

test: build-inplace
	PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python $(PYTHON) -m pytest tests
	cargo test

style:: flake8

flake8:
	$(PYTHON) -m flake8 janitor tests

style:: yamllint

yamllint:
	yamllint .github/

style:: djlint

djlint:
	djlint -i J018,H030,H031,H021 --profile jinja janitor/site/templates

typing:
	$(PYTHON) -m mypy janitor tests

janitor/site/_static/pygments.css:
	pygmentize -S default -f html > $@

clean:

SHA=$(shell git rev-parse HEAD)

docker-%: core
	buildah build --no-cache -t ghcr.io/jelmer/janitor/$*:$(DOCKER_TAG) -t ghcr.io/jelmer/janitor/$*:$(SHA) -f Dockerfile_$* .
	buildah push ghcr.io/jelmer/janitor/$*:$(DOCKER_TAG)
	buildah push ghcr.io/jelmer/janitor/$*:$(SHA)

docker-all: docker-site docker-runner docker-publish docker-archive docker-worker docker-git_store docker-bzr_store docker-differ docker-ognibuild_dep

reformat:: reformat-html

reformat-html:
	djlint --reformat --format-css janitor/site/templates/
