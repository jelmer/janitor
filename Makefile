export PYTHONPATH=$(shell pwd)/ognibuild:$(shell pwd)/buildlog-consultant:$(shell pwd):$(shell pwd)/breezy:$(shell pwd)/silver-platter:$(shell pwd)/lintian-brush:$(shell pwd)/python-debian/lib:$(shell pwd)/debmutate:$(shell pwd)/dulwich

PB2_PY_OUTPUT = janitor/policy_pb2.py janitor/config_pb2.py janitor/package_overrides_pb2.py janitor/candidates_pb2.py janitor/package_metadata_pb2.py

all: janitor/site/_static/pygments.css $(PB2_PY_OUTPUT)
	$(MAKE) -C breezy
	$(MAKE) -C dulwich

PROTOC_ARGS = --python_out=.

ifneq ($(MYPY_PROTO),0)
PROTOC_ARGS += --mypy_out=.
endif

janitor/%_pb2.py: janitor/%.proto
	protoc $(PROTOC_ARGS) $<

check:: typing

check:: test

check:: style

check-it-all:: test-it-all

test-it-all:: 
	PYTHONPATH=$(PYTHONPATH) python3 -m unittest dulwich.tests.test_suite
	PYTHONPATH=$(PYTHONPATH) python3 -m unittest breezy.tests.test_suite
	PYTHONPATH=$(PYTHONPATH) python3 -m unittest janitor.tests.test_suite
	PYTHONPATH=$(PYTHONPATH) python3 -m unittest lintian_brush.tests.test_suite
	PYTHONPATH=$(PYTHONPATH) python3 -m unittest silver_platter.tests.test_suite
	PYTHONPATH=$(PYTHONPATH) python3 -m unittest ognibuild.tests.test_suite
	PYTHONPATH=$(PYTHONPATH) python3 -m unittest buildlog_consultant.tests.test_suite
	PYTHONPATH=$(PYTHONPATH) python3 -m unittest upstream_ontologist.tests.test_suite
	PYTHONPATH=$(PYTHONPATH) python3 -m unittest debmutate.tests.test_suite

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
