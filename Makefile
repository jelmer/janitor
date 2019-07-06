check:
	flake8
	PYTHONPATH=.:silver-platter:lintian-brush python3 setup.py test

janitor/site/_static/pygments.css:
	pygmentize -S default -f html > $@
