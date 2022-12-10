#!/usr/bin/python3

import argparse
import os
import shutil
import subprocess
import tempfile

from iniparse import RawConfigParser
from janitor.config import read_config, get_distribution


def sbuild_schroot_name(suite, arch):
    return "%s-%s-sbuild" % (suite, arch)


def create_chroot(distro, sbuild_path, suites, sbuild_arch, include=[],  # noqa: B006
                  eatmydata=True, make_sbuild_tarball=None):
    cmd = ["sbuild-createchroot", distro.name, sbuild_path,
           distro.archive_mirror_uri]
    cmd.append("--components=%s" % ','.join(distro.component))
    if eatmydata:
        cmd.append("--command-prefix=eatmydata")
        include = list(include) + ["eatmydata"]
    if include:
        cmd.append("--include=%s" % ','.join(include))
    for suite in suites:
        cmd.append("--alias=%s" % sbuild_schroot_name(suite, sbuild_arch))
    if make_sbuild_tarball:
        cmd.append("--make-sbuild-tarball=%s" % make_sbuild_tarball)
    for name in distro.extra:
        cmd.append("--extra-repository=deb %s %s %s" % (
            distro.archive_mirror_uri, name, ' '.join(distro.component)))

    subprocess.check_call(cmd)


def get_sbuild_architecture():
    return subprocess.check_output(
        ["dpkg-architecture", "-qDEB_BUILD_ARCH"]).decode().strip()


parser = argparse.ArgumentParser()
parser.add_argument('--remove-old', action='store_true')
parser.add_argument(
    '--include', type=str, action='append', help='Include specified package.',
    default=[])
parser.add_argument(
    '--base-directory', type=str, help='Base directory for chroots')
parser.add_argument(
    '--user', type=str, help='User to create home directory for')
parser.add_argument(
    '--make-sbuild-tarball', action='store_true', help='Create sbuild tarball')
parser.add_argument(
    "--config", type=str, default="janitor.conf", help="Path to configuration."
)

parser.add_argument("distribution", type=str, nargs="*")
args = parser.parse_args()

with open(args.config, "r") as f:
    config = read_config(f)

if not args.distribution:
    args.distribution = [d.name for d in config.distribution]

for distribution in args.distribution:
    try:
        distro_config = get_distribution(config, distribution)
    except KeyError:
        parser.error('no such distribution: %s' % distribution)

    sbuild_arch = get_sbuild_architecture()
    if not args.base_directory:
        parser.print_usage()
        parser.exit()

    if args.make_sbuild_tarball:
        sbuild_path = tempfile.mkdtemp()
    else:
        sbuild_path = os.path.join(args.base_directory, distro_config.chroot)

    if args.remove_old:
        for entry in os.scandir('/etc/schroot/chroot.d'):
            cp = RawConfigParser()
            cp.read([entry.path])
            if distro_config.chroot in cp.sections():
                old_sbuild_path = cp.get(
                    sbuild_schroot_name(distro_config.name, sbuild_arch),
                    'directory')
                if old_sbuild_path != sbuild_path:
                    raise AssertionError(
                        'sbuild path has changed: %s != %s' % (
                            old_sbuild_path, sbuild_path))
                if os.path.isdir(old_sbuild_path):
                    shutil.rmtree(old_sbuild_path)
                os.unlink(entry.path)

    suites = []
    for campaign in config.campaign:
        if not campaign.HasField('debian_build'):
            continue
        if campaign.debian_build.base_distribution != distro_config.name:
            continue
        suites.append(campaign.debian_build.build_distribution)
    if args.make_sbuild_tarball:
        make_sbuild_tarball = os.path.join(
            args.base_directory, distro_config.chroot + '.tar.xz')
    else:
        make_sbuild_tarball = None
    create_chroot(
        distro_config, sbuild_path, suites, sbuild_arch, args.include,
        make_sbuild_tarball=make_sbuild_tarball,
        eatmydata=True)

    if args.user:
        subprocess.check_call(
            "install -d / --owner=%s \"~%s\" | sbuild-shell %s" % (
                args.user, args.user,
                sbuild_schroot_name(distro_config.name, sbuild_arch)),
            shell=True)
