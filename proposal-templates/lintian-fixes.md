{# vim: ft=jinja
#}
{% extends "base.md" %}
{% block runner %}
{% if len(applied) > 1 %}
Fix some issues reported by lintian
{% for entry in applied %}
{% if len(entries) > 1 %}* {% endif %}{{ entry.summary }}{% if entry.fixed_lintian_tags %} ({% for tag in entry.fixed_lintian_tags %}[{{ tag }}](https://lintian.debian.org/tags/{{ tag }}.html){% if not loop.last %}, {% endif %}{% endfor %}){% endif %}
{% endfor %}
{% endblock %}
