all: html html/index.html

html:
	mkdir -p $@

html/index.html: templates/index.html templates/layout.html
	python3 generate.py

.PHONY: all
