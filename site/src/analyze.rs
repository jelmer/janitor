use buildlog_consultant::common::find_build_failure_description;

/// The result of analyzing a build log.
pub struct AnalyzeResult {
    /// The total number of lines in the build log.
    pub total_lines: usize,

    /// The range of lines to include in the output.
    pub include_lines: (usize, usize),

    /// The line numbers to highlight in the output.
    pub highlight_lines: Option<Vec<usize>>,
}

/// Find the failure in a dist.log file.
pub fn find_dist_log_failure(logf: &str, length: usize) -> AnalyzeResult {
    let lines = logf.lines().collect::<Vec<&str>>();
    let (r#match, _unused_err) = find_build_failure_description(lines.clone());
    let highlight_lines = r#match.map(|r#match| r#match.linenos());

    let include_lines = (std::cmp::max(1, lines.len() - length), lines.len());

    AnalyzeResult {
        total_lines: lines.len(),
        include_lines,
        highlight_lines,
    }
}

/// Find the build failure in the given build log.
///
/// Returns:
/// - The total number of lines in the build log.
/// - The range of lines to include in the output.
/// - The line numbers to highlight in the output.
pub fn find_build_log_failure<R: std::io::Read>(logf: R, length: usize) -> AnalyzeResult {
    let bufread = std::io::BufReader::new(logf);
    let sbuildlog = buildlog_consultant::sbuild::SbuildLog::try_from(bufread).unwrap();
    let linecount = sbuildlog.sections().last().map(|s| s.offsets.1).unwrap();
    let failure = buildlog_consultant::sbuild::worker_failure_from_sbuild_log(&sbuildlog);

    if failure.r#match.is_some() && failure.section.is_some() {
        let section = failure.section.as_ref().unwrap();
        let r#match = failure.r#match.as_ref().unwrap();
        let abs_offset = section.offsets.0 + r#match.lineno();
        let include_lines = (
            std::cmp::max(1, abs_offset - length / 2),
            abs_offset + std::cmp::min(length / 2, section.lines.len()),
        );
        let highlight_lines = vec![abs_offset];
        return AnalyzeResult {
            total_lines: linecount,
            include_lines,
            highlight_lines: Some(highlight_lines),
        };
    }

    if let Some(r#match) = failure.r#match.as_ref() {
        let include_lines = (
            std::cmp::max(1, r#match.lineno() - length / 2),
            r#match.lineno() + std::cmp::min(length / 2, linecount),
        );
        let highlight_lines = vec![r#match.lineno()];
        return AnalyzeResult {
            total_lines: linecount,
            include_lines,
            highlight_lines: Some(highlight_lines),
        };
    }

    let include_lines = if let Some(section) = failure.section.as_ref() {
        (
            std::cmp::max(1, section.offsets.1 - length),
            Some(section.offsets.1),
        )
    } else if length < linecount {
        (linecount - length, None)
    } else {
        (1, Some(linecount))
    };

    AnalyzeResult {
        total_lines: linecount,
        include_lines: (include_lines.0, include_lines.1.unwrap_or(linecount)),
        highlight_lines: None,
    }
}
