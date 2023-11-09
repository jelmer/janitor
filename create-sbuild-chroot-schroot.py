#!/usr/bin/python3

import argparse
import os
import shutil
import subprocess
import tempfile

from iniparse import RawConfigParser

from janitor.config import get_distribution, read_config


def sbuild_schroot_name(suite, arch):
    return f"{suite}-{arch}-sbuild"


def create_chroot(
    distro,
    sbuild_path,
    suites,
    sbuild_arch,
    include=[],  # noqa: B006
    eatmydata=True,
    make_sbuild_tarball=None,
    aliases=None,
    chroot_mode: str | None = None,
):
    cmd = ["sbuild-createchroot", distro.name, sbuild_path, distro.archive_mirror_uri]
    cmd.append("--components=%s" % ",".join(distro.component))
    if eatmydata:
        cmd.append("--command-prefix=eatmydata")
        include = list(include) + ["eatmydata"]
    if include:
        cmd.append("--include=%s" % ",".join(include))
    if aliases is None:
        aliases = []
    aliases = list(aliases) + [
        sbuild_schroot_name(suite, sbuild_arch) for suite in suites
    ]
    for alias in aliases:
        cmd.append("--alias=%s" % alias)
    if make_sbuild_tarball:
        cmd.append("--make-sbuild-tarball=%s" % make_sbuild_tarball)
    if chroot_mode:
        cmd.append(f"--chroot-mode={chroot_mode}")
    for name in distro.extra:
        cmd.append(
            "--extra-repository=deb {} {} {}".format(
                distro.archive_mirror_uri, name, " ".join(distro.component)
            )
        )

    subprocess.check_call(cmd)


def get_sbuild_architecture():
    return (
        subprocess.check_output(["dpkg-architecture", "-qDEB_BUILD_ARCH"])
        .decode()
        .strip()
    )


parser = argparse.ArgumentParser()
parser.add_argument("--remove-old", action="store_true")
parser.add_argument(
    "--include",
    type=str,
    action="append",
    help="Include specified package.",
    default=[],
)
parser.add_argument("--base-directory", type=str, help="Base directory for chroots")
parser.add_argument("--user", type=str, help="User to create home directory for")
parser.add_argument(
    "--make-sbuild-tarball", action="store_true", help="Create sbuild tarball"
)
parser.add_argument(
    "--config", type=str, default="janitor.conf", help="Path to configuration."
)
parser.add_argument(
    "--chroot-mode",
    type=str,
    choices=["schroot", "sudo", "unshare"],
    default="schroot",
    help="sbuild chroot mode",
)
parser.add_argument("--run-command", type=str, action="append")

parser.add_argument("distribution", type=str, nargs="*")
args = parser.parse_args()

with open(args.config) as f:
    config = read_config(f)

if not args.distribution:
    args.distribution = [d.name for d in config.distribution]

for distribution in args.distribution:
    try:
        distro_config = get_distribution(config, distribution)
    except KeyError:
        parser.error("no such distribution: %s" % distribution)

    sbuild_arch = get_sbuild_architecture()
    if not args.base_directory:
        parser.print_usage()
        parser.exit()

    if args.make_sbuild_tarball:
        sbuild_path = tempfile.mkdtemp()
    else:
        sbuild_path = os.path.join(args.base_directory, distro_config.chroot)

    if args.remove_old:
        for entry in os.scandir("/etc/schroot/chroot.d"):
            cp = RawConfigParser()
            cp.read([entry.path])
            if distro_config.chroot in cp.sections():
                old_sbuild_path = cp.get(
                    sbuild_schroot_name(distro_config.name, sbuild_arch), "directory"
                )
                if old_sbuild_path != sbuild_path:
                    raise AssertionError(
                        f"sbuild path has changed: {old_sbuild_path} != {sbuild_path}"
                    )
                if os.path.isdir(old_sbuild_path):
                    shutil.rmtree(old_sbuild_path)
                os.unlink(entry.path)

    suites = []
    for campaign in config.campaign:
        if not campaign.HasField("debian_build"):
            continue
        if campaign.debian_build.base_distribution != distro_config.name:
            continue
        suites.append(campaign.debian_build.build_distribution)
    make_sbuild_tarball: str | None
    if args.make_sbuild_tarball:
        make_sbuild_tarball = os.path.join(
            args.base_directory, distro_config.chroot + ".tar.xz"
        )
    else:
        make_sbuild_tarball = None
    create_chroot(
        distro_config,
        sbuild_path,
        suites,
        sbuild_arch,
        args.include,
        make_sbuild_tarball=make_sbuild_tarball,
        eatmydata=True,
        aliases=distro_config.chroot_alias,
        chroot_mode=args.chroot_mode,
    )

    if args.run_command:
        for cmd in args.run_command:
            p = subprocess.Popen(
                f"sbuild-shell {sbuild_schroot_name(distro_config.name, sbuild_arch)}",
                shell=True,
                stdin=subprocess.PIPE,
            )
            p.communicate(cmd.encode())
            if p.returncode != 0:
                raise Exception(f"command {cmd} failed to run: {p.returncode}")
