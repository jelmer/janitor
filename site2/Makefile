all: html/index.html html/contact/index.html html/credentials/index.html html/apt/index.html html/lintian-fixes/index.html

html:
	mkdir -p $@

html/index.html html/apt/index.html html/contact/index.html html/credentials/index.html html/lintian-fixes/index.html: templates/index.html templates/apt.html templates/contact.html templates/credentials.html templates/layout.html templates/lintian-fixes.html
	python3 generate.py

.PHONY: all
