export PYTHONPATH=ognibuild:buildlog-consultant:.:breezy:silver-platter:lintian-brush:python-debian/lib:debmutate

PB2_PY_OUTPUT = janitor/policy_pb2.py janitor/config_pb2.py janitor/package_overrides_pb2.py janitor/candidates_pb2.py janitor/package_metadata_pb2.py janitor/upstream_project_pb2.py

all: janitor/site/_static/pygments.css $(PB2_PY_OUTPUT)

PROTOC_ARGS = --python_out=.

ifneq ($(MYPY_PROTO),0)
PROTOC_ARGS += --mypy_out=.
endif

janitor/%_pb2.py: janitor/%.proto
	protoc $(PROTOC_ARGS) $<

check:: typing

check:: test

check:: style

test:
	PYTHONPATH=$(PYTHONPATH) python3 setup.py test

style:
	flake8

typing:
	PYTHONPATH=.:silver-platter:lintian-brush:breezy mypy janitor

janitor/site/_static/pygments.css:
	pygmentize -S default -f html > $@

clean:
	rm -f $(PB2_PY_OUTPUT)
