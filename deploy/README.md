# Deployment

Ansible playbook for deploying a full jelmer/janitor instance (all
services, Postgres with the debversion extension, Caddy, rootless
podman/Quadlet) onto a single Debian host, plus a Vagrant VM for exercising
the playbook locally before running it against a real instance.

## Layout

- `ansible/site.yml` - the playbook entry point. Roles run in order:
  `base`, `janitor_source` (clone + build/pull the service images),
  `postgres`, `redis`, `caddy`, `janitor_config`, `janitor_quadlet`
  (systemd user units for every service, plus sbuild/schroot setup inside
  the worker container).
- `ansible/inventory.ini` - fill in the real host/IP, SSH user, and key
  path for the target instance.
- `ansible/group_vars/all.yml` - deployment variables and the list of
  required secrets (see the comment block at the top of that file for
  exactly what to supply and how).
- `ansible/group_vars/vagrant.yml` - fake, local-dev-only values applied
  only when running against the Vagrant VM below.
- `Vagrantfile` - a local VM that runs the same playbook via
  `ansible_local`, for validating changes without needing a real instance.

## Usage

Against a real instance:

```
cd ansible
# edit inventory.ini and group_vars/all.yml (or supply -e/--ask-vault-pass)
ansible-galaxy install -r requirements.yml
ansible-playbook site.yml
```

Real instances default to `use_prebuilt_images: false` (source builds).
`roles/janitor_source`'s image-build task has no change-detection, so
every re-run - even just to fix an unrelated variable - rebuilds all 11
images from scratch (`make build-all`, `buildah build --no-cache`,
several hours). Set `use_prebuilt_images: true` in `group_vars/all.yml`
(or pass `-e use_prebuilt_images=true`) for a re-run that only needs to
pick up config/service changes, not a fresh image build.

Locally, via Vagrant (requires a Vagrant provider - libvirt is what this
Vagrantfile is set up for):

```
vagrant up                              # fast path: pull pre-built images from ghcr.io
JANITOR_USE_PREBUILT=false vagrant up   # slow path: build all images from source (~hours)
vagrant provision                       # re-run the playbook without recreating the VM
vagrant ssh
vagrant destroy
```

### Vagrant environment variables

- `JANITOR_USE_PREBUILT` (default `true`) - pull pre-built `ghcr.io`
  images instead of building from source. This VM's purpose is validating
  the playbook itself, not iterating on janitor's own source, so pre-built
  is the right default; switch to `false` only when actually testing a
  change to a Dockerfile or to `janitor_repo`/`janitor_branch`.
- `JANITOR_VM_CPUS` (default `4`) / `JANITOR_VM_MEMORY` (default `4096`,
  or `8192` when building from source) - VM sizing.
- `JANITOR_VM_IP` (default `192.168.56.10`) - the VM's private-network IP.
  Reach the site at `http://{JANITOR_VM_IP}/` (see the comment above the
  `forwarded_port` line in the Vagrantfile for why the forwarded port
  doesn't work under libvirt).
- `JANITOR_REPO` / `JANITOR_BRANCH` (default upstream `main`) - point the
  VM at a fork/branch carrying unmerged fixes, without editing
  `group_vars/all.yml`.
