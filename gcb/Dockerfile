FROM debian:sid
MAINTAINER Jelmer Vernooij <jelmer@debian.org>
RUN mkdir -p /srv/chroot
RUN apt-get update
RUN apt-get install -y sbuild
RUN sbuild-createchroot --include=eatmydata,ccache,gnupg unstable /srv/chroot/unstable-amd64-sbuild http://deb.debian.org/debian
COPY sbuildrc /root/.sbuildrc

# Install debian-janitor dependencies
RUN apt-get install -y brz git python3-debian python3-dulwich sudo python3-distro-info python3-apt devscripts patchutils pristine-tar quilt python3-yaml fakeroot dpkg-dev libalgorithm-merge-perl
RUN brz branch lp:brz-debian /opt/brz-debian
# Check out debian-janitor
RUN git clone --recurse-submodules https://salsa.debian.org/jelmer/debian-janitor /opt/debian-janitor
RUN mkdir -p /output
ENV PYTHONPATH=/opt/debian-janitor:/opt/debian-janitor/silver-platter:/opt/debian-janitor/lintian-brush
ENV BRZ_PLUGINS_AT=debian@/opt/brz-debian
ENV BRZ_HOME=/brzhome
RUN mkdir -p /brzhome
ENTRYPOINT ["python3", "-m", "janitor.worker"]
