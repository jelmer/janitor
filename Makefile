PB2_PY_OUTPUT = janitor/policy_pb2.py janitor/config_pb2.py janitor/package_overrides_pb2.py janitor/candidates_pb2.py

all: janitor/site/_static/pygments.css $(PB2_PY_OUTPUT)

PROTOC_ARGS = --python_out=.

ifneq ($(MYPY_PROTO),0)
PROTOC_ARGS += --mypy_out=.
endif

janitor/%_pb2.py: janitor/%.proto
	protoc $(PROTOC_ARGS) $<

check:
	PYTHONPATH=.:silver-platter:lintian-brush:breezy mypy janitor
	PYTHONPATH=.:silver-platter:lintian-brush:breezy python3 setup.py test
	flake8

janitor/site/_static/pygments.css:
	pygmentize -S default -f html > $@

clean:
	rm -f $(PB2_PY_OUTPUT)
