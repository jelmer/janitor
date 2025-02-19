DOCKER_TAG ?= latest
PYTHON ?= python3
SHA = $(shell git rev-parse HEAD)
DOCKERFILES = $(shell ls Dockerfile_* | sed 's/Dockerfile_//' )
DOCKER_TARGETS := $(patsubst %,docker-%,$(DOCKERFILES))
BUILD_TARGETS := $(patsubst %,build-%,$(DOCKERFILES))
PUSH_TARGETS := $(patsubst %,push-%,$(DOCKERFILES))

.PHONY: all check

build-inplace:
	$(PYTHON) setup.py build_ext -i
	$(PYTHON) setup.py build_protobuf

all: core

core: py/janitor/site/_static/pygments.css build-inplace

check:: typing

check:: test

check:: style

check:: ruff

check:: check-format

check-format::
	ruff format --check py tests

check-format::
	cargo fmt --check --all

ruff:
	ruff check py tests

fix:: ruff-fix

fix:: cargo-fix

cargo-fix:
	cargo clippy --fix

ruff-fix:
	ruff check --fix .

reformat::
	ruff format py tests

reformat::
	cargo fmt --all

suite-references:
	git grep "\\(lintian-brush\|lintian-fixes\|debianize\|fresh-releases\|fresh-snapshots\\)" | grep -v .example

test:: build-inplace
	PYTHONPATH=$(shell pwd)/py:$(PYTHONPATH) PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python $(PYTHON) -m pytest -vv tests

test::
	cargo test

style:: yamllint

yamllint:
	yamllint .github/

style:: djlint

djlint:
	djlint -i J018,H030,H031,H021 --profile jinja py/janitor/site/templates

typing:
	$(PYTHON) -m mypy py/janitor tests

py/janitor/site/_static/pygments.css:
	pygmentize -S default -f html > $@

clean:

docker-%:
	$(MAKE) build-$*
	$(MAKE) push-$*

build-%:
	buildah build --no-cache -t ghcr.io/jelmer/janitor/$*:$(DOCKER_TAG) -t ghcr.io/jelmer/janitor/$*:$(SHA) -f Dockerfile_$* .

push-%:
	buildah push ghcr.io/jelmer/janitor/$*:$(DOCKER_TAG)
	buildah push ghcr.io/jelmer/janitor/$*:$(SHA)

docker-all: $(DOCKER_TARGETS)

build-all: $(BUILD_TARGETS)

push-all: $(PUSH_TARGETS)

reformat:: reformat-html

reformat-html:
	djlint --reformat --format-css py/janitor/site/templates/

codespell:
	codespell
