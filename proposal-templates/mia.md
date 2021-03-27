{% extends "base.md" %}
{% block runner %}
Remove MIA uploaders:

{% for uploader in removed_uploaders %}
* {{ uploader }}
{% endfor %}
{% endblock %}
