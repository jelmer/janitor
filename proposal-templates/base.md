{# vim: ft=jinja
#}
{# Maximum number of lines of debdiff to inline in the merge request
   description. If this threshold is reached, we'll just include a link to the
   debdiff.
#}
{% set DEBDIFF_INLINE_THRESHOLD = 40 %}

{{ runner }}

This merge proposal was created automatically by the [Janitor bot]({{ external_url }}/{{ suite }}).
For more information, including instructions on how to disable
these merge proposals, see {{ external_url }}/{{ suite }}.

You can follow up to this merge proposal as you normally would.

The bot will automatically update the merge proposal to resolve merge conflicts
or close the merge proposal when all changes are applied through other means
(e.g. cherry-picks). Updates may take several hours to propagate.

Build and test logs for this branch can be found at
{{ external_url }}/{{ suite }}/pkg/{{ package }}/{{ log_id }}.

{% if role == 'main' and debdiff %}
{% if not debdiff_is_empty(debdiff) %}
These changes have no impact on the [binary debdiff](
{{ external_url }}/api/run/{{ log_id }}/debdiff?filter_boring=1).
{% elif len(debdiff.splitlines(False)) < DEBDIFF_INLINE_THRESHOLD %}
## Debdiff

These changes affect the binary packages:

{{ markdownify_debdiff(debdiff) }}
{% else %}
These changes affect the binary packages; see the
[debdiff]({{ external_url }}/api/run/\
{{ log_id }}/debdiff?filter_boring=1)
{% endif %}

You can also view the [diffoscope diff](\
{{ external_url }}/api/run/{{ log_id }}/diffoscope?filter_boring=1) \
([unfiltered]({{ external_url }}/api/run/{{ log_id }}/diffoscope)).
{% endif %}
