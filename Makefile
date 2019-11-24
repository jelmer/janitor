all: janitor/site/_static/pygments.css janitor/policy_pb2.py janitor/config_pb2.py janitor/package_overrides_pb2.py

janitor/%_pb2.py: janitor/%.proto
	protoc --python_out=. $<

check:
	flake8
	PYTHONPATH=.:silver-platter:lintian-brush:breezy python3 setup.py test

janitor/site/_static/pygments.css:
	pygmentize -S default -f html > $@
