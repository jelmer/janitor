all: html/index.html html/contact/index.html html/credentials/index.html

html:
	mkdir -p $@

html/index.html html/contact/index.html html/credentials/index.html: templates/index.html templates/contact.html templates/credentials.html templates/layout.html
	python3 generate.py

.PHONY: all
