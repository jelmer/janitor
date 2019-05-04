check:
	flake8
	PYTHONPATH=.:silver-platter:lintian-brush python3 setup.py test
