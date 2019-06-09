all: html/index.html html/contact/index.html html/credentials/index.html html/apt/index.html

html:
	mkdir -p $@

html/index.html html/apt/index.html html/contact/index.html html/credentials/index.html: templates/index.html templates/apt.html templates/contact.html templates/credentials.html templates/layout.html
	python3 generate.py

.PHONY: all
