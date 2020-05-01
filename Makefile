PB2_PY_OUTPUT = janitor/policy_pb2.py janitor/config_pb2.py janitor/package_overrides_pb2.py janitor/candidates_pb2.py

all: janitor/site/_static/pygments.css $(PB2_PY_OUTPUT)

janitor/%_pb2.py: janitor/%.proto
	protoc --python_out=. --mypy_out=. $<

check:
	flake8
	mypy janitor
	PYTHONPATH=.:silver-platter:lintian-brush:breezy python3 setup.py test

janitor/site/_static/pygments.css:
	pygmentize -S default -f html > $@

clean:
	rm -f $(PB2_PY_OUTPUT)
