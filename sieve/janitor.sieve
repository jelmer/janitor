require ["vnd.dovecot.execute", "envelope"];

# Hand off all e-mails to the janitor from salsa.
if allof(header :contains "To" "janitor@jelmer.uk",
         envelope "from" "gitlab@salsa.debian.org") {
  execute :pipe "janitor-reprocess";
}
