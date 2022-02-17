#!/bin/sh
{% for distro in janitor_distributions %}
( {{ distro.package_metadata_command }} ) | python3 -m janitor.package_metadata \
	--config={{ janitor_conf_path}} \
	--distribution={{ distro.name }} \
	--package-overrides={% if janitor_package_overrides %}{{ janitor_package_overrides }}{% else %}/dev/null{% endif %}\
	"$@"
{% endfor %}
( {{ janitor_candidates_command }} ) | python3 -m janitor.candidates --config={{ janitor_conf_path}} "$@"
python3 -m janitor.schedule \
	--config={{ janitor_conf_path}} \
	"$@"
