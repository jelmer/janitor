pub fn iter_sections(text: &str) -> impl Iterator<Item = (Option<&str>, Vec<&str>)> {
    let lines = text.split_terminator('\n').collect::<Vec<_>>();
    let mut title = None;
    let mut paragraph = Vec::new();
    let mut i = 0;
    let mut ret = vec![];
    while i < lines.len() {
        let line = lines[i];
        if i + 1 < lines.len() && lines[i + 1] == ("-".repeat(line.len())) {
            if title.is_some() || !paragraph.is_empty() {
                ret.push((title, paragraph));
            }
            title = Some(line);
            paragraph = Vec::new();
            i += 1;
        } else if line.trim_end().is_empty() {
            if title.is_some() || !paragraph.is_empty() {
                ret.push((title, paragraph));
            }
            title = None;
            paragraph = Vec::new();
        } else {
            paragraph.push(line);
        }
        i += 1;
    }
    if title.is_some() || !paragraph.is_empty() {
        ret.push((title, paragraph));
    }
    ret.into_iter()
}

pub fn filter_boring_wdiff<'a>(
    lines: Vec<&'a str>, old_version: &'a str, new_version: &'a str
) -> Vec<String> {
    if lines.is_empty() {
        return Vec::new();
    }
    let (field, _changes) = match lines[0].split_once(':') {
        Some((field, changes)) => (field, changes),
        None => return lines.iter().map(|line| line.to_string()).collect::<Vec<_>>(),
    };
    if field == "Installed-Size" {
        return Vec::new();
    }
    if field == "Version" {
        return Vec::new();
    }
    let regex = regex::Regex::new(&format!(r"\[-{}(.*?)-\] \{{\+{}\1\+\}}",
                regex::escape(old_version),
                regex::escape(new_version),
            )).unwrap();

    let lines = lines.iter().map(|line| {
        regex.replace_all(line, "").to_string()
    }).collect::<Vec<_>>();
    let block = lines.join("\n");

    if lazy_regex::regex_find!(r"\[-.*?-\]", &block).is_none() && !lazy_regex::regex_find!(r"\{\+.*?\+\}", &block).is_none() {
        return Vec::new();
    }
    lines
}

fn iter_fields<'a> (lines: impl Iterator<Item = &'a str> + 'a) -> impl Iterator<Item = Vec<&'a str>> + 'a {
    let mut cl = Vec::new();
    let mut ret = Vec::new();
    for line in lines {
        if !cl.is_empty() && line.starts_with(" ") {
            cl.push(line);
        } else {
            ret.push(cl);
            cl = vec![line];
        }
    }
    if !cl.is_empty() {
        ret.push(cl);
    }
    ret.into_iter()
}

pub fn filter_boring(debdiff: &str, old_version: &str, new_version: &str) -> String {
    let mut ret = Vec::new();
    for (title, paragraph) in iter_sections(debdiff) {
        if title.is_none() {
            ret.push((title, paragraph.iter().map(|line| line.to_string()).collect::<Vec<_>>()));
            continue;
        }
        let title = title.unwrap();
        let (package, wdiff) = if let Some((_, package)) = lazy_regex::regex_captures!(r"Control files of package (.*): lines which differ \(wdiff format\)", title) {
            (Some(package), true)
        } else if title == "Control files: lines which differ (wdiff format)" {
            (None, true)
        } else {
            (None, false)
        };
        if wdiff {
            let mut paragraph_unfiltered = Vec::new();
            for lines in iter_fields(paragraph.into_iter()) {
                let newlines = filter_boring_wdiff(lines, old_version, new_version);
                paragraph_unfiltered.extend(newlines);
            }
            let paragraph = paragraph_unfiltered;
            if paragraph.iter().any(|line| line.trim().is_empty()) {
                if let Some(package) = package {
                    ret.push((None, vec![format!("No differences were encountered between the control files of package {}", package)]));
                } else {
                    ret.push((None, vec!["No differences were encountered in the control files".to_string()]));
                }
            } else {
                ret.push((Some(title), paragraph));
            }
        } else {
            ret.push((Some(title), paragraph.iter().map(|line| line.to_string()).collect::<Vec<_>>()));
        }
    }

    let mut lines = vec![];
    for (title, paragraph) in ret {
        if let Some(title) = title {
            lines.push(title.to_string());
            lines.push("-".repeat(title.len()));
        }
        lines.extend(paragraph);
        lines.push("".to_string());
    }
    lines.join("\n")
}

#[derive(Debug)]
pub struct DebdiffError {
    message: String,
}

impl From<tokio::io::Error> for DebdiffError {
    fn from(e: tokio::io::Error) -> Self {
        DebdiffError { message: e.to_string() }
    }
}

impl std::fmt::Display for DebdiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "DebdiffError: {}", self.message)
    }
}

impl std::error::Error for DebdiffError {}

pub async fn run_debdiff(old_binaries: Vec<&str>, new_binaries: Vec<&str>) -> Result<Vec<u8>, DebdiffError> {
    let args = ["debdiff", "--from"].iter().chain(old_binaries.iter()).chain(["--to"].iter()).chain(new_binaries.iter()).collect::<Vec<_>>();
    let mut p = tokio::process::Command::new(args[0]);
    for arg in args.iter().skip(1) {
        p.arg(arg);
    }
    let output = p.output().await?;
    if !output.status.success() {
        return Err(DebdiffError { message: String::from_utf8_lossy(&output.stderr).to_string() });
    }
    Ok(output.stdout)
}

pub fn debdiff_is_empty(debdiff: &str) -> bool {
    !iter_sections(debdiff).any(|(title, _paragraph)| title.is_some())
}

pub fn section_is_wdiff(title: &str) -> (bool, Option<&str>) {
    if let Some((_, package)) = lazy_regex::regex_captures!(r"Control files of package (.*): lines which differ \(wdiff format\)", title) {
        return (true, Some(package));
    }
    if title == "Control files: lines which differ (wdiff format)" {
        return (true, None);
    }
    (false, None)
}

pub fn markdownify_debdiff(debdiff: &str) -> String {
    let fix_wdiff_md = |line: &str| {
        // GitLab markdown will render links but then not show the
        // delta highlighting. This fools it into not autolinking:
        line.replace("://", "&#8203;://")
    };

    let mut ret = vec![];
    for (title, lines) in iter_sections(debdiff) {
        if let Some(title) = title {
            ret.push(format!("### {}", title));
            let (wdiff, _package) = section_is_wdiff(title);
            if wdiff {
                ret.extend(lines.iter().filter_map(|line| if line.trim().is_empty() { None } else { Some(format!("* {}", fix_wdiff_md(line))) }));
            } else {
                for line in lines {
                    ret.push(format!("    {}", line));
                }
            }
        } else {
            ret.push("".to_owned());
            for line in lines {
                if !line.trim().is_empty() {
                    let line = lazy_regex::regex_replace!(
                        "^(No differences were encountered between the control files of package) (.*)$",
                        r"\1 \*\*\2\*\*",
                        line,
                    );
                    ret.push(line.to_string());
                } else {
                    ret.push("".to_owned());
                }
            }
            if ret.last() == Some(&String::new()) {
                ret.pop();
            }
        }
    }
    ret.join("\n")
}

pub fn htmlize_debdiff(debdiff: &str) -> String {
    let highlight_wdiff = |line| {
        let line = lazy_regex::regex_replace!(
            r"\[-(.*?)-\]", r#"<span style="color:red;font-weight:bold">\1</span>"#, line
        );
        let line = lazy_regex::regex_replace!(
            r"\{\+(.*?)\+\}",
            r#"<span style="color:green;font-weight:bold">\1</span>"#,
            line,
        );
        line
    };

    let mut ret = vec![];
    for (title, lines) in iter_sections(debdiff) {
        if let Some(title) = title {
            ret.push(format!("<h4>{}</h4>", title));
            let wdiff = if lazy_regex::regex_is_match!(
                r"Control files of package .*: lines which differ \(wdiff format\)",
                title,
            ) {
                true
            } else if title == "Control files: lines which differ (wdiff format)" {
                true
            } else {
                false
            };
            if wdiff {
                ret.push("<ul>".to_owned());
                for mlines in iter_fields(lines.into_iter()) {
                    if mlines.is_empty() {
                        continue;
                    }
                    ret.push(format!("<li><pre>{}</pre></li>", highlight_wdiff(mlines.join("\n"))));
                }
                ret.push("</ul>".to_owned());
            } else {
                ret.push("<pre>".to_owned());
                ret.extend(lines.iter().map(|line| line.to_string()).collect::<Vec<_>>());
                ret.push("</pre>".to_owned());
            }
        } else {
            ret.push("<p>".to_owned());
            for line in lines {
                if !line.trim().is_empty() {
                    let line = lazy_regex::regex_replace!(
                        "^(No differences were encountered between the control files of package) (.*)$",
                        "\\1 <b>\\2</b>",
                        line,
                    ).to_string();
                    ret.push(line);
                } else {
                    ret.push("</p>".to_owned());
                    ret.push("<p>".to_owned());
                }
            }
            if ret.last().unwrap() == "<p>" {
                ret.pop();
            }
        }
    }
    ret.join("\n")
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_nothing() {
        assert_eq!(iter_sections("foo\n").collect::<Vec<_>>(), vec![(None, vec!["foo"])]);
    }

    #[test]
    fn test_simple() {
    assert_eq!(
        vec![
            (
                None,
                vec![
                    "[The following lists of changes regard files as different if they have different names, permissions or owners.]",
                ],
            ),
            (
                Some("Files in second .changes but not in first"),
                vec!["-rw-r--r--  root/root   /usr/lib/debug/.build-id/e4/3520e0f1e.debug"],
            ),
            (
                Some("Files in first .changes but not in second"),
                vec!["-rw-r--r--  root/root   /usr/lib/debug/.build-id/28/0303571bd.debug"],
            ),
            (
                Some("Control files of package xserver-blah: lines which differ (wdiff format)"),
                vec![
                    "Installed-Size: [-174-] {+170+}",
                    "Version: [-1:1.7.9-2~jan+unchanged1-] {+1:1.7.9-3~jan+lint1+}",
                ],
            ),
            (
                Some("Control files of package xserver-dbgsym: lines which differ (wdiff format)"),
                vec![
                    "Build-Ids: [-280303571bd7f8-] {+e43520e0f1eb+}",
                    "Depends: xserver-blah (= [-1:1.7.9-2~jan+unchanged1)-] {+1:1.7.9-3~jan+lint1)+}",
                    "Installed-Size: [-515-] {+204+}",
                    "Version: [-1:1.7.9-2~jan+unchanged1-] {+1:1.7.9-3~jan+lint1+}",
                ],
            ),
        ], iter_sections(r#"[The following lists of changes regard files as different if they have
different names, permissions or owners.]

Files in second .changes but not in first
-----------------------------------------
-rw-r--r--  root/root   /usr/lib/debug/.build-id/e4/3520e0f1e.debug

Files in first .changes but not in second
-----------------------------------------
-rw-r--r--  root/root   /usr/lib/debug/.build-id/28/0303571bd.debug

Control files of package xserver-blah: lines which differ (wdiff format)
------------------------------------------------------------------------
Installed-Size: [-174-] {+170+}
Version: [-1:1.7.9-2~jan+unchanged1-] {+1:1.7.9-3~jan+lint1+}

Control files of package xserver-dbgsym: lines which differ (wdiff format)
--------------------------------------------------------------------------
Build-Ids: [-280303571bd7f8-] {+e43520e0f1eb+}
Depends: xserver-blah (= [-1:1.7.9-2~jan+unchanged1)-] {+1:1.7.9-3~jan+lint1)+}
Installed-Size: [-515-] {+204+}
Version: [-1:1.7.9-2~jan+unchanged1-] {+1:1.7.9-3~jan+lint1+}
"#).collect::<Vec<_>>());
    }

    #[test]
    fn test_just_versions() {
        let debdiff = r#"File lists identical (after any substitutions)

Control files of package acpi-fakekey: lines which differ (wdiff format)
------------------------------------------------------------------------
Version: [-0.143-4~jan+unchanged1-] {+0.143-5~jan+lint1+}

Control files of package acpi-fakekey-dbgsym: lines which differ (wdiff format)
-------------------------------------------------------------------------------
Depends: acpi-fakekey (= [-0.143-4~jan+unchanged1)-] {+0.143-5~jan+lint1)+}
Version: [-0.143-4~jan+unchanged1-] {+0.143-5~jan+lint1+}

Control files of package acpi-support: lines which differ (wdiff format)
------------------------------------------------------------------------
Version: [-0.143-4~jan+unchanged1-] {+0.143-5~jan+lint1+}

Control files of package acpi-support-base: lines which differ (wdiff format)
-----------------------------------------------------------------------------
Version: [-0.143-4~jan+unchanged1-] {+0.143-5~jan+lint1+}
"#;
    let newdebdiff = super::filter_boring(debdiff, "0.143-4~jan+unchanged1", "0.143-5~jan+lint1");
    assert_eq!(
        newdebdiff,
        r#"File lists identical (after any substitutions)

No differences were encountered between the control files of package \
acpi-fakekey

No differences were encountered between the control files of package \
acpi-fakekey-dbgsym

No differences were encountered between the control files of package \
acpi-support

No differences were encountered between the control files of package \
acpi-support-base
"#);
    }
}
