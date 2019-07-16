all: janitor/site/_static/pygments.css janitor/policy_pb2.py

janitor/policy_pb2.py: janitor/policy.proto
	protoc --python_out=. $<

check:
	flake8
	PYTHONPATH=.:silver-platter:lintian-brush python3 setup.py test

janitor/site/_static/pygments.css:
	pygmentize -S default -f html > $@
