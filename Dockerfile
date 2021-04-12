FROM debian:sid AS janitor-base
MAINTAINER Jelmer Vernooij <jelmer@debian.org>

RUN apt update
RUN apt install --no-install-recommends -y python3 python3-aiohttp python3-configobj python3-jinja2 python3-debian python3-asyncpg python3-protobuf protobuf-compiler python3-yaml
ENV BRZ_PLUGINS_AT=debian@/code/breezy-debian
ENV PYTHONPATH=/code:/code/breezy:/code/dulwich:/code/lintian-brush:/code/debmutate
ADD . /code

FROM janitor-base AS janitor-site
EXPOSE 8082
# TODO: config
# TODO: service discovery
ENTRYPOINT ["python3", "-m", "janitor.site", "--port=8082", "--host=127.0.0.1"]
#--publisher-url http://[2a01:348:125:15::2]:9912/ --vcs-store-url http://[2a01:348:125:15::2]:9923/ --runner-url http://[2a01:348:125:15::2]:9911/ --config /home/janitor/janitor.conf --archiver-url http://[2a01:348:125:15::2]:9914/ --external-url=https://janitor.debian.net/ --differ-url=http://[2a01:348:125:15::1]:9920/ --debugtoolbar=2a01:348:125:8::11

FROM janitor-base AS janitor-archive
EXPOSE 9914
ENTRYPOINT ["python3", "-m", "janitor.debian.archive", "--port=9914", "--host=127.0.0.1"]

FROM janitor-base AS janitor-vcs-store
VOLUME /vcs
EXPOSE 9923
ENTRYPOINT ["python3", "-m", "janitor.vcs_store", "--port=9923", "--host=127.0.0.1"]

FROM janitor-base AS janitor-publisher
EXPOSE 9912
ENTRYPOINT ["python3", "-m", "janitor.publish", "--port=9912", "--host=127.0.0.1"]

FROM janitor-base AS janitor-differ
EXPOSE 9920
ENTRYPOINT ["python3", "-m", "janitor.differ", "--port=9920", "--host=127.0.0.1"]

FROM janitor-base AS janitor-runner
EXPOSE 9911
ENTRYPOINT ["python3", "-m", "janitor.runner", "--port=9911", "--host=127.0.0.1"]

FROM debian:sid AS janitor-worker
RUN apt update
RUN apt install --no-install-recommends -y python3 python3-aiohttp python3-configobj python3-jinja2 python3-debian python3-asyncpg python3-protobuf protobuf-compiler python3-yaml python3-apt python3-distro-info
ENV PYTHONPATH=/code:/code/breezy:/code/dulwich:/code/lintian-brush:/code/ognibuild:/code/silver-platter:/code/buildlog-consultant:/code/upstream-ontologist:/code/debmutate
ENV BRZ_PLUGINS_AT=debian@/code/breezy-debian
ADD . /code
ENTRYPOINT ["python3", "-m", "janitor.pull_worker"]
