FROM debian:sid
MAINTAINER Jelmer Vernooij <jelmer@debian.org>
RUN mkdir -p /srv/chroot
RUN apt-get update
RUN apt-get install --no-install-recommends -y sbuild schroot autopkgtest debootstrap piuparts lintian devscripts debhelper dpkg-dev build-essential
COPY 04tmpfs /etc/schroot/setup.d/04tmpfs
RUN chmod 0755 /etc/schroot/setup.d/04tmpfs
RUN sbuild-createchroot --include=eatmydata,ccache,gnupg unstable /srv/chroot/unstable-amd64-sbuild http://deb.debian.org/debian
COPY sbuildrc /sbuildrc
ENV SBUILD_CONFIG=/sbuildrc

# Install debian-janitor dependencies
RUN apt-get install --no-install-recommends -y brz git python3-debian python3-dulwich sudo python3-distro-info python3-apt devscripts patchutils pristine-tar quilt python3-yaml fakeroot dpkg-dev libalgorithm-merge-perl python3-launchpadlib python3-github python3-gitlab libdistro-info-perl
RUN brz branch lp:brz-debian /opt/brz-debian
# Check out debian-janitor
RUN git clone --recurse-submodules https://salsa.debian.org/jelmer/debian-janitor /opt/debian-janitor
ENV PYTHONPATH=/opt/debian-janitor:/opt/debian-janitor/silver-platter:/opt/debian-janitor/lintian-brush
ENV BRZ_PLUGINS_AT=debian@/opt/brz-debian
ENV BRZ_HOME=/brzhome
RUN mkdir -p /brzhome
ENTRYPOINT ["python3", "-m", "janitor.worker", "--output-directory=/workspace", "--tgz-repo"]
