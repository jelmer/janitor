

async fn scan_packages<T>(td: &Path, arch: Option<&str>) -> Result<impl Stream<Item = Result<T, std::io::Error> {
    let mut args = vec![];
    if let Some(arch) = arch {
        args.extend(&["-a", arch]);
    }
    let mut proc = Command::new("dpkg-scanpackages")
        .args(&[td])
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        ?;

    if !proc.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("dpkg-scanpackages failed: {}", proc.status),
        ));
    }

    let stdout = proc.stdout.take().unwrap();
    let stderr = proc.stderr.take().unwrap();

    let stdout = BufReader::new(stdout);
    let stderr = BufReader::new(stderr);


}

async def scan_packages(td, arch: Optional[str] = None):
    args = []
    if arch:
        args.extend(["-a", arch])
    proc = await asyncio.create_subprocess_exec(
        "dpkg-scanpackages",
        td,
        *args,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    stdout, stderr = await proc.communicate()
    for para in Packages.iter_paragraphs(stdout, use_apt_pkg=False):
        yield para
    for line in stderr.splitlines(keepends=False):
        if line.startswith(b"dpkg-scanpackages: "):
            line = line[len(b"dpkg-scanpackages: ") :]
        if line.startswith(b"info: "):
            logging.debug("%s", line.rstrip(b"\n").decode())
        elif line.startswith(b"warning: "):
            logging.warning("%s", line.rstrip(b"\n").decode())
        elif line.startswith(b"error: "):
            logging.error("%s", line.rstrip(b"\n").decode())
        else:
            logging.info("dpkg-scanpackages error: %s", line.rstrip(b"\n").decode())


async def scan_sources(td):
    proc = await asyncio.create_subprocess_exec(
        "dpkg-scansources",
        td,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    stdout, stderr = await proc.communicate()
    for para in Sources.iter_paragraphs(stdout, use_apt_pkg=False):
        yield para
    for line in stderr.splitlines(keepends=False):
        if line.startswith(b"dpkg-scansources: "):
            line = line[len(b"dpkg-scansources: ") :]
        if line.startswith(b"info: "):
            logging.debug("%s", line.rstrip(b"\n").decode())
        elif line.startswith(b"warning: "):
            logging.warning("%s", line.rstrip(b"\n").decode())
        elif line.startswith(b"error: "):
            logging.error("%s", line.rstrip(b"\n").decode())
        else:
            logging.info("dpkg-scansources error: %s", line.rstrip(b"\n").decode())


class GeneratingPackageInfoProvider(PackageInfoProvider):
    def __init__(self, artifact_manager) -> None:
        self.artifact_manager = artifact_manager

    async def __aenter__(self):
        await self.artifact_manager.__aenter__()

    async def __aexit__(self, exc_tp, exc_val, exc_tb):
        await self.artifact_manager.__aexit__(exc_tp, exc_val, exc_tb)
        return False

    async def packages_for_run(self, run_id, suite_name, package, arch):
        with tempfile.TemporaryDirectory(prefix=TMP_PREFIX) as td:
            await self.artifact_manager.retrieve_artifacts(
                run_id, td, timeout=DEFAULT_GCS_TIMEOUT
            )
            async for para in scan_packages(td):
                para["Filename"] = os.path.join(
                    suite_name,
                    "pkg",
                    package,
                    run_id,
                    os.path.basename(para["Filename"]),
                )
                yield bytes(para)
                yield b"\n"

        await asyncio.sleep(0)

    async def sources_for_run(self, run_id, suite_name, package):
        with tempfile.TemporaryDirectory(prefix=TMP_PREFIX) as td:
            await self.artifact_manager.retrieve_artifacts(
                run_id, td, timeout=DEFAULT_GCS_TIMEOUT
            )
            async for para in scan_sources(td):
                para["Directory"] = os.path.join(
                    suite_name,
                    "pkg",
                    package,
                    run_id,
                )
                yield bytes(para)
                yield b"\n"

        await asyncio.sleep(0)


class DiskCachingPackageInfoProvider(PackageInfoProvider):
    def __init__(self, primary_info_provider, cache_directory) -> None:
        self.primary_info_provider = primary_info_provider
        self.cache_directory = cache_directory

    async def __aenter__(self):
        await self.primary_info_provider.__aenter__()

    async def __aexit__(self, exc_tp, exc_val, exc_tb):
        await self.primary_info_provider.__aexit__(exc_tp, exc_val, exc_tb)
        return False

    async def packages_for_run(self, run_id, suite_name, package, arch):
        cache_path = os.path.join(self.cache_directory, f"binary-{arch}", run_id)
        os.makedirs(os.path.dirname(cache_path), exist_ok=True)
        try:
            with open(cache_path, "rb") as f:
                for chunk in f:
                    yield chunk
        except FileNotFoundError:
            logger.debug(
                "Retrieving artifacts for %s/%s (%s)", suite_name, package, run_id
            )
            with open(cache_path, "wb") as f:
                async for chunk in self.primary_info_provider.packages_for_run(
                    run_id, suite_name, package, arch=arch
                ):
                    f.write(chunk)
                    yield chunk

    async def sources_for_run(self, run_id, suite_name, package):
        cache_path = os.path.join(self.cache_directory, "source", run_id)
        os.makedirs(os.path.dirname(cache_path), exist_ok=True)
        try:
            with open(cache_path, "rb") as f:
                for chunk in f:
                    yield chunk
        except FileNotFoundError:
            logger.debug(
                "Retrieving artifacts for %s/%s (%s)", suite_name, package, run_id
            )
            with open(cache_path, "wb") as f:
                async for chunk in self.primary_info_provider.sources_for_run(
                    run_id, suite_name, package
                ):
                    f.write(chunk)
                    yield chunk

    async def cache_run(self, run_id, suite_name, package, arches):
        async for _ in self.sources_for_run(run_id, suite_name, package):
            pass
        for arch in arches:
            async for _ in self.packages_for_run(run_id, suite_name, package, arch):
                pass


async def retrieve_packages(info_provider, rows, suite_name, component, arch):
    logger.debug(
        "Need to process %d rows for %s/%s/%s", len(rows), suite_name, component, arch
    )
    for package, run_id, build_distribution, _build_version in rows:
        try:
            async for chunk in info_provider.packages_for_run(
                run_id, build_distribution, package, arch=arch
            ):
                yield chunk
                await asyncio.sleep(0)
        except ArtifactsMissing:
            logger.warning("Artifacts missing for %s (%s), skipping", package, run_id)
            continue


async def retrieve_sources(info_provider, rows, suite_name, component):
    logger.debug("Need to process %d rows for %s/%s", len(rows), suite_name, component)
    for package, run_id, build_distribution, _build_version in rows:
        try:
            async for chunk in info_provider.sources_for_run(
                run_id, build_distribution, package
            ):
                yield chunk
                await asyncio.sleep(0)
        except ArtifactsMissing:
            logger.warning("Artifacts missing for %s (%s), skipping", package, run_id)
            continue


async def get_builds_for_suite(db, build_distribution):
    async with db.acquire() as conn:
        return await conn.fetch(
            "SELECT DISTINCT ON (source) "
            "debian_build.source, debian_build.run_id, debian_build.distribution, "
            "debian_build.version FROM debian_build "
            "INNER JOIN run ON run.id = debian_build.run_id "
            "WHERE debian_build.distribution = $1 AND run.publish_status != 'rejected' "
            "ORDER BY debian_build.source, debian_build.version DESC",
            build_distribution,
        )


async def get_builds_for_changeset(db, cs_id):
    async with db.acquire() as conn:
        return await conn.fetch(
            "SELECT DISTINCT ON (source) "
            "debian_build.source, debian_build.run_id, debian_build.distribution, "
            "debian_build.version FROM debian_build "
            "INNER JOIN run ON run.id = debian_build.run_id "
            "WHERE run.change_set = $1 AND run.publish_status != 'rejected' "
            "ORDER BY debian_build.source, debian_build.version DESC",
            cs_id,
        )


async def get_builds_for_run(db, run_id):
    async with db.acquire() as conn:
        return await conn.fetch(
            "SELECT DISTINCT ON (source) "
            "debian_build.source, debian_build.run_id, debian_build.distribution, "
            "debian_build.version FROM debian_build "
            "INNER JOIN run ON run.id = debian_build.run_id "
            "WHERE run.id = $1 AND run.publish_status != 'rejected' "
            "ORDER BY debian_build.source, debian_build.version DESC",
            run_id,
        )


HASHES = {
    "MD5Sum": hashlib.md5,
    "SHA1": hashlib.sha1,
    "SHA256": hashlib.sha256,
    "SHA512": hashlib.sha512,
}


def cleanup_by_hash_files(base, number_to_keep):
    for h in HASHES:
        ages = []
        for entry in os.scandir(os.path.join(base, "by-hash", h)):
            ages.append((entry, time.time() - entry.stat().st_mtime))

        ages.sort(key=lambda k: k[1])

        for entry, _age in ages[number_to_keep:]:
            os.unlink(entry.path)


async fn write_suite_files(
    base_path: &Path,
    get_packages,
    get_sources,
    suite_name,
    archive_description,
    components,
    arches,
    origin,
    gpg_context,
    timestamp: Optional[datetime] = None,
)
    SUFFIXES: dict[str, Any] = {
        "": open,
        ".gz": gzip.GzipFile,
        ".bz2": bz2.BZ2File,
    }

    let timestamp: chrono::DateTime<chrono::Utc> = timestamp.unwrap_or_else(chrono::Utc::now());

    stamp = mktime(timestamp.timetuple())

    let r = Release {
        origin,
        label: archive_description,
        codename: suite_name,
        suite: suite_name,
        date: formatdate(timeval=stamp, localtime=False, usegmt=True),
        not_automatic: true,
        but_automatic_upgrades: true,
        architectures: arches,
        components,
        description: "Generated by the Janitor",
        acquire_by_hash: true,
    };

    for component in components {
        debug!("Publishing component {}/{}", suite_name, component);
        let component_dir = component;
        let base_path = base_path.join(&component_dir);
        std::fs::create_dir_all(&base_path).await?;
        for arch in arches {
            let arch_dir = component_dir.join(format!("binary-{}", arch));
            std::fs::create_dir_all(base_path.join(&arch_dir)).await?;

            let packages_path = arch_dir.join("Packages");

            let mut fs = vec![];
            for (suffix, f) in SUFFIXES.iter() {


                fs.append(
                    es.enter_context(
                        HashedFileWriter(r, base_path, packages_path + suffix, fn)
                    )
                )
            async for chunk in get_packages(suite_name, component, arch):
                for f in fs:
                    f.write(chunk)
            for f in fs:
                f.done()
            cleanup_by_hash_files(
                os.path.join(base_path, arch_dir), 4 * len(SUFFIXES)
            )
            await asyncio.sleep(0)
        }
        source_dir = os.path.join(component_dir, "source")
        os.makedirs(os.path.join(base_path, source_dir), exist_ok=True)

        sources_path = os.path.join(source_dir, "Sources")
        fs = []
        for suffix, fn in SUFFIXES.items():
            fs.append(
                es.enter_context(
                    HashedFileWriter(r, base_path, sources_path + suffix, fn)
                )
            )
        async for chunk in get_sources(suite_name, component):
            for f in fs:
                f.write(chunk)
        for f in fs:
            f.done()
        cleanup_by_hash_files(
            os.path.join(base_path, source_dir), 4 * len(SUFFIXES)
        )

        await asyncio.sleep(0)

    debug!("Writing Release file for {}", suite_name);
    with open(os.path.join(base_path, "Release"), "wb") as f:
        r.dump(f)

    logger.debug("Writing Release.gpg file for %s", suite_name)
    data = gpg.Data(r.dump())
    with open(os.path.join(base_path, "Release.gpg"), "wb") as f:
        signature, result = gpg_context.sign(data, mode=gpg_mode.DETACH)
        f.write(signature)

    logger.debug("Writing InRelease file for %s", suite_name)
    data = gpg.Data(r.dump())
    with open(os.path.join(base_path, "InRelease"), "wb") as f:
        signature, result = gpg_context.sign(data, mode=gpg_mode.CLEAR)
        f.write(signature)


/// Get supported architectures from configuration
pub fn get_supported_architectures(config: &crate::config::ArchiveConfig) -> Vec<String> {
    // Get architectures from the first configured repository, or use defaults
    if let Some((_, repo_config)) = config.repositories.iter().next() {
        repo_config.architectures.clone()
    } else {
        // Default architectures if no repositories configured
        vec!["amd64".to_string(), "all".to_string()]
    }
}


@routes.post("/publish", name="publish")
async def handle_publish(request):
    post = await request.post()
    campaign = post.get("campaign")
    for campaign_config in request.app["config"].campaign:
        if not campaign_config.HasField("debian_build"):
            continue
        if campaign is not None and campaign != campaign_config.name:
            continue
        await request.app["generator_manager"].trigger_campaign(campaign_config.name)

    return web.json_response({})


@routes.get("/last-publish", name="last-publish")
async def handle_last_publish(request):
    return web.json_response(
        {suite: dt.isoformat() for (suite, dt) in last_publish_time.items()}
    )


@routes.get("/health", name="health")
async def handle_health(request):
    return web.Response(text="ok")


@routes.get("/ready", name="ready")
async def handle_ready(request):
    missing = []
    for apt_repo_config in request.app["config"].apt_repository:
        if apt_repo_config.name not in last_publish_time:
            missing.append(apt_repo_config.name)
    status = "".join(
        [f"{name}: {dt.isoformat()}\n" for (name, dt) in last_publish_time.items()]
    )
    if missing:
        return web.Response(
            text=(
                "missing: {}".format(", ".join(missing)) + "\n\n" "present:\n" + status
            ),
            status=500,
        )
    return web.Response(text=status)


@routes.get("/", name="index")
async def handle_index(request):
    return web.Response(text="")


@routes.get("/pgp_keys", name="pgp-keys")
async def handle_pgp_keys(request):
    pgp_keys = []
    for entry in list(request.app["gpg"].keylist(secret=True)):
        pgp_keys.append(request.app["gpg"].key_export_minimal(entry.fpr).decode())
    return web.json_response(pgp_keys)


async def serve_dists_release_file(request):
    path = os.path.join(
        request.app["dists_dir"],
        request.match_info["release"],
        request.match_info["file"],
    )
    if not os.path.exists(path):
        raise web.HTTPNotFound(text="suite release files not generated yet")
    return web.FileResponse(path)


async def serve_dists_component_file(request):
    path = os.path.join(
        request.app["dists_dir"],
        request.match_info["release"],
        request.match_info["component"],
        request.match_info["arch"],
        request.match_info["file"],
    )
    if not os.path.exists(path):
        raise web.HTTPNotFound(text="suite component files not generated yet")
    return web.FileResponse(path)


async def serve_dists_component_hash_file(request):
    path = os.path.join(
        request.app["dists_dir"],
        request.match_info["release"],
        request.match_info["component"],
        request.match_info["arch"],
        "by-hash",
        request.match_info["hash_type"],
        request.match_info["hash"],
    )
    if not os.path.exists(path):
        raise web.HTTPNotFound(text="suite by-hash file not present")
    return web.FileResponse(path)


async def refresh_on_demand_dists(
    dists_dir, db, config, package_info_provider, gpg_context, kind, id
):
    os.makedirs(os.path.join(dists_dir, kind, id), exist_ok=True)
    release_path = os.path.join(dists_dir, kind, id, "Release")
    try:
        with open(release_path) as f:
            release = Release(f)
    except FileNotFoundError:
        stamp = None
    else:
        stamp = parsedate_to_datetime(release["Date"])
    async with db.acquire() as conn:
        if kind == "run":
            # /run/{run_id}
            row = await conn.fetchrow(
                "SELECT suite, max(finish_time) FROM run WHERE id = $1", id
            )
            if row is None:
                raise web.HTTPNotFound(text=f"no such run: {id}")
            campaign, max_finish_time = row
            builds = await get_builds_for_run(db, id)
            campaign_config = get_campaign_config(config, campaign)
        elif kind == "cs":
            # /cs/{change_set_id}
            campaign = await conn.fetchval(
                "SELECT campaign FROM change_set WHERE id = $1", id
            )
            if campaign is None:
                raise web.HTTPNotFound(text=f"no such changeset: {id}")
            max_finish_time = await conn.fetchval(
                "SELECT max(finish_time) FROM run WHERE change_set = $1", id
            )
            builds = await get_builds_for_changeset(db, id)
            description = f"Change set {id}"
            campaign_config = get_campaign_config(config, campaign)
        else:
            # /{suite}/{codebase}
            try:
                campaign_config = get_campaign_config(config, kind)
            except KeyError as e:
                raise web.HTTPNotFound(text=f"No such campaign: {kind}") from e
            cs_id = await conn.fetchval(
                "SELECT run.change_set FROM run "
                "INNER JOIN change_set ON change_set.id = run.change_set "
                "WHERE run.suite = $1 AND run.codebase = $2 "
                "AND change_set.state in ('working', 'ready', 'publishing', 'done') AND "
                "run.result_code = 'success' "
                "ORDER BY run.finish_time DESC",
                kind,
                id,
            )
            if cs_id is None:
                if not (
                    await conn.fetchrow(
                        "SELECT 1 FROM debian_build WHERE source = $1", id
                    )
                ):
                    raise web.HTTPNotFound(text=f"No such source package: {id}")
            max_finish_time = await conn.fetchval(
                "SELECT max(finish_time) FROM run WHERE change_set = $1", cs_id
            )
            builds = await get_builds_for_changeset(db, cs_id)
            description = f"Campaign {kind} for {id}"
    if stamp is not None and max_finish_time and max_finish_time.astimezone() < stamp:
        return
    logging.info("Generating metadata for %s/%s", kind, id, extra={"run_id": id})
    distribution = get_distribution(
        config, campaign_config.debian_build.base_distribution
    )
    await write_suite_files(
        os.path.join(dists_dir, kind, id),
        get_packages=partial(retrieve_packages, package_info_provider, builds),
        get_sources=partial(retrieve_sources, package_info_provider, builds),
        suite_name=f"{kind}/{id}",
        archive_description=description,
        components=distribution.component,
        arches=ARCHES,
        origin=config.origin,
        gpg_context=gpg_context,
    )


async def serve_on_demand_dists_release_file(request):
    await refresh_on_demand_dists(
        request.app["dists_dir"],
        request.app["db"],
        request.app["config"],
        request.app["generator_manager"].package_info_provider,
        request.app["gpg"],
        request.match_info["kind"],
        request.match_info["id"],
    )

    path = os.path.join(
        request.app["dists_dir"],
        request.match_info["kind"],
        request.match_info["id"],
        request.match_info["file"],
    )
    if not os.path.exists(path):
        raise web.HTTPNotFound()
    return web.FileResponse(path)


async def serve_on_demand_dists_component_file(request):
    await refresh_on_demand_dists(
        request.app["dists_dir"],
        request.app["db"],
        request.app["config"],
        request.app["generator_manager"].package_info_provider,
        request.app["gpg"],
        request.match_info["kind"],
        request.match_info["id"],
    )

    path = os.path.join(
        request.app["dists_dir"],
        request.match_info["kind"],
        request.match_info["id"],
        request.match_info["component"],
        request.match_info["arch"],
        request.match_info["file"],
    )
    if not os.path.exists(path):
        raise web.HTTPNotFound()
    return web.FileResponse(path)


async def serve_on_demand_dists_component_hash_file(request):
    await refresh_on_demand_dists(
        request.app["dists_dir"],
        request.app["db"],
        request.app["config"],
        request.app["generator_manager"].package_info_provider,
        request.app["gpg"],
        request.match_info["kind"],
        request.match_info["id"],
    )

    path = os.path.join(
        request.app["dists_dir"],
        request.match_info["kind"],
        request.match_info["id"],
        request.match_info["component"],
        request.match_info["arch"],
        "by-hash",
        request.match_info["hash_type"],
        request.match_info["hash"],
    )
    if not os.path.exists(path):
        raise web.HTTPNotFound()
    return web.FileResponse(path)


async def create_app(generator_manager, config, dists_dir, db):
    trailing_slash_redirect = normalize_path_middleware(append_slash=True)
    app = web.Application(
        middlewares=[trailing_slash_redirect, state.asyncpg_error_middleware]
    )
    app["gpg"] = gpg.Context(armor=True)
    app["dists_dir"] = dists_dir
    app["config"] = config
    app["generator_manager"] = generator_manager
    app["db"] = db
    setup_metrics(app)
    app.router.add_routes(routes)
    app.router.add_get(
        "/dists/{release}/{file:InRelease|Release.gpg|Release}",
        serve_dists_release_file,
    )
    app.router.add_get(
        "/dists/{release}/{component}/{arch}/" r"{file:(Packages|Sources)(|\..*)}",
        serve_dists_component_file,
    )
    app.router.add_get(
        "/dists/{release}/{component}/{arch}/" r"by-hash/{hash_type}/{hash}",
        serve_dists_component_hash_file,
    )

    CAMPAIGNS_REGEX = "|".join(re.escape(c.name) for c in config.campaign)
    app.router.add_get(
        "/dists/{kind:cs|run|"
        + CAMPAIGNS_REGEX
        + "}/{id}/{file:InRelease|Release.gpg|Release}",
        serve_on_demand_dists_release_file,
    )
    app.router.add_get(
        "/dists/{kind:cs|run|" + CAMPAIGNS_REGEX + "}/{id}/{component}/{arch}/"
        r"{file:(Packages|Sources)(|\..*)}",
        serve_on_demand_dists_component_file,
    )
    app.router.add_get(
        "/dists/{kind:cs|run|" + CAMPAIGNS_REGEX + "}/{id}/{component}/{arch}/"
        r"by-hash/{hash_type}/{hash}",
        serve_on_demand_dists_component_hash_file,
    )
    return app


async def run_web_server(
    listen_addr, port, dists_dir, config, db, generator_manager, tracer
):
    app = await create_app(generator_manager, config, dists_dir, db)
    aiozipkin.setup(app, tracer)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, listen_addr, port)
    await site.start()


async def listen_to_runner(redis, generator_manager):
    async def handle_result_message(msg):
        result = json.loads(msg["data"])
        if result["code"] != "success":
            return
        if result["target"]["name"] != "debian":
            return
        await generator_manager.trigger_campaign(result["campaign"])

    try:
        async with redis.pubsub(ignore_subscribe_messages=True) as ch:
            await ch.subscribe("result", result=handle_result_message)
            await ch.run()
    finally:
        await redis.close()


async def publish_repository(
    dists_directory,
    db,
    package_info_provider,
    config,
    apt_repository_config,
    gpg_context,
):
    start_time = datetime.utcnow()
    logger.info("Publishing %s", apt_repository_config.name)
    distribution = get_distribution(config, apt_repository_config.base)
    assert distribution
    suite_path = os.path.join(dists_directory, apt_repository_config.name)
    builds = []
    for select in apt_repository_config.select:
        campaign_config = get_campaign_config(config, select.campaign)
        assert campaign_config
        assert campaign_config.debian_build
        builds.extend(
            await get_builds_for_suite(
                db, campaign_config.debian_build.build_distribution
            )
        )
    await write_suite_files(
        suite_path,
        get_packages=partial(retrieve_packages, package_info_provider, builds),
        get_sources=partial(retrieve_sources, package_info_provider, builds),
        suite_name=apt_repository_config.name,
        archive_description=apt_repository_config.description,
        components=distribution.component,
        arches=ARCHES,
        origin=config.origin,
        gpg_context=gpg_context,
    )

    logger.info(
        "Done publishing %s (took %s)",
        apt_repository_config.name,
        datetime.utcnow() - start_time,
    )
    last_publish_success.labels(suite=apt_repository_config.name).set_to_current_time()
    last_publish_time[apt_repository_config.name] = datetime.utcnow()


class GeneratorManager:
    def __init__(
        self, dists_dir, db, config, package_info_provider, gpg_context
    ) -> None:
        self.dists_dir = dists_dir
        self.db = db
        self.config = config
        self.package_info_provider = package_info_provider
        self.gpg_context = gpg_context
        self.scheduler = Scheduler()
        self.jobs: dict[str, Job] = {}
        self._campaign_to_repository: dict[str, list[AptRepositoryConfig]] = {}
        for apt_repo in self.config.apt_repository:
            for select in apt_repo.select:
                self._campaign_to_repository.setdefault(select.campaign, []).append(
                    apt_repo
                )

    async def trigger_campaign(self, campaign_name):
        for apt_repo in self._campaign_to_repository.get(campaign_name, []):
            await self.trigger(apt_repo)

    async def trigger(self, apt_repository_config: AptRepositoryConfig):
        try:
            job = self.jobs[apt_repository_config.name]
        except KeyError:
            pass
        else:
            if not job.closed:
                return
        self.jobs[apt_repository_config.name] = await self.scheduler.spawn(
            publish_repository(
                self.dists_dir,
                self.db,
                self.package_info_provider,
                self.config,
                apt_repository_config,
                self.gpg_context,
            )
        )


async def loop_publish(config, generator_manager: GeneratorManager) -> None:
    while True:
        for apt_repo in config.apt_repository:
            await generator_manager.trigger(apt_repo)
        # every 12 hours
        await asyncio.sleep(60 * 60 * 12)


async def main(argv=None):
    import argparse

    from redis.asyncio import Redis

    parser = argparse.ArgumentParser(prog="janitor.debian.archive")
    parser.add_argument(
        "--listen-address", type=str, help="Listen address", default="localhost"
    )
    parser.add_argument("--port", type=int, help="Listen port", default=9914)
    parser.add_argument(
        "--config", type=str, default="janitor.conf", help="Path to configuration."
    )
    parser.add_argument("--dists-directory", type=str, help="Dists directory")
    parser.add_argument("--cache-directory", type=str, help="Cache directory")
    parser.add_argument("--verbose", action="store_true")
    parser.add_argument(
        "--gcp-logging", action="store_true", help="Use Google cloud logging."
    )

    args = parser.parse_args()
    if not args.dists_directory:
        parser.print_usage()
        sys.exit(1)

    if args.gcp_logging:
        import google.cloud.logging

        client = google.cloud.logging.Client()
        client.get_default_handler()
        client.setup_logging()
    elif args.verbose:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    with open(args.config) as f:
        config = read_config(f)

    os.makedirs(args.dists_directory, exist_ok=True)

    db = await state.create_pool(config.database_location)

    endpoint = aiozipkin.create_endpoint(
        "janitor.debian.archive", ipv4=args.listen_address, port=args.port
    )
    if config.zipkin_address:
        tracer = await aiozipkin.create(
            config.zipkin_address, endpoint, sample_rate=0.1
        )
    else:
        tracer = await aiozipkin.create_custom(endpoint)
    trace_configs = [aiozipkin.make_trace_config(tracer)]

    artifact_manager = get_artifact_manager(
        config.artifact_location, trace_configs=trace_configs
    )

    gpg_context = gpg.Context()

    package_info_provider: PackageInfoProvider
    package_info_provider = GeneratingPackageInfoProvider(artifact_manager)
    if args.cache_directory:
        os.makedirs(args.cache_directory, exist_ok=True)
        package_info_provider = DiskCachingPackageInfoProvider(
            package_info_provider, args.cache_directory
        )

    generator_manager = GeneratorManager(
        args.dists_directory,
        db,
        config,
        package_info_provider,
        gpg_context,
    )

    loop = asyncio.get_event_loop()
    tasks = [
        loop.create_task(
            run_web_server(
                args.listen_address,
                args.port,
                args.dists_directory,
                config,
                db,
                generator_manager,
                tracer,
            )
        ),
        loop.create_task(loop_publish(config, generator_manager)),
    ]

    redis = Redis.from_url(config.redis_location)

    tasks.append(loop.create_task(listen_to_runner(redis, generator_manager)))

    async with package_info_provider:
        await asyncio.gather(*tasks)


if __name__ == "__main__":
    asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())
    sys.exit(asyncio.run(main(sys.argv)))
