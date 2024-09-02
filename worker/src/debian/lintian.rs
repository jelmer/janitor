use log::debug;
use serde_json;
use std::path::{PathBuf,Path};
use std::process::{Command, Output};
use std::str;

#[derive(Debug)]
pub enum Error {
    LintianFailed(std::io::Error),
    LintianOutputInvalid(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::LintianFailed(ref e) => write!(f, "Lintian failed: {}", e),
            Error::LintianOutputInvalid(ref e) => write!(f, "Lintian output invalid: {}", e),
        }
    }
}

#[derive(serde::Deserialize, PartialEq, Eq, serde::Serialize)]
pub struct LintianInputFile {
    pub hints: Vec<String>,
    pub path: PathBuf,
}

#[derive(serde::Deserialize, PartialEq, Eq, serde::Serialize)]
pub struct LintianGroup {
    pub group_id: String,
    pub input_files: Vec<LintianInputFile>,
    pub source_name: String,
    pub source_version: debversion::Version
}

#[derive(serde::Deserialize, PartialEq, Eq, Default, serde::Serialize)]
pub struct LintianResult {
    pub groups: Vec<LintianGroup>,
}

impl std::error::Error for Error {}

fn parse_lintian_output(text: &str) -> Result<LintianResult, serde_json::Error> {
    let lines: Vec<&str> = text.trim().split('\n').collect();
    let mut joined_lines: Vec<&str> = Vec::new();
    for line in lines {
        joined_lines.push(line);
        if line == "}" {
            break;
        }
    }

    let joined_str = joined_lines.join("\n");
    let mut result: LintianResult = serde_json::from_str(&joined_str)?;

    // Strip irrelevant directory information
    for group in &mut result.groups {
        for input_file in &mut group.input_files {
            input_file.path = Path::new(input_file.path.file_name().unwrap()).to_path_buf();
        }
    }

    Ok(result)
}

pub fn run_lintian(
    output_directory: &Path,
    changes_names: Vec<&Path>,
    profile: Option<&str>,
    suppress_tags: Option<Vec<&str>>,
) -> Result<LintianResult, Error> {
    let mut args: Vec<String> = vec![
        "--exp-output=format=json".to_owned(),
        "--allow-root".to_owned(),
    ];
    if let Some(tags) = suppress_tags {
        args.push(format!("--suppress-tags={}", tags.join(",")));
    }
    if let Some(profile_str) = profile {
        args.push(format!("--profile={}", profile_str));
    }
    let mut cmd = Command::new("lintian");
    cmd.args(args);
    cmd.args(changes_names);
    cmd.current_dir(output_directory);
    debug!("Running lintian: {:?}", cmd);

    let lintian_output: Output = match cmd.output() {
        Ok(output) => output,
        Err(e) => {
            return Err(Error::LintianFailed(e));
        }
    };

    let output_str = match str::from_utf8(&lintian_output.stdout) {
        Ok(s) => s,
        Err(e) => {
            return Err(Error::LintianOutputInvalid(e.to_string()));
        }
    };

    parse_lintian_output(output_str).map_err(|e| Error::LintianOutputInvalid(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lintian_output() {
        let output_str = r#"{
   "groups" : [
      {
         "group_id" : "lintian-brush_0.148",
         "input_files" : [
            {
               "hints" : [],
               "path" : "/tmp/lintian-brush_0.148.dsc"
            },
            {
               "hints" : [],
               "path" : "/tmp/lintian-brush_0.148_source.buildinfo"
            },
            {
               "hints" : [],
               "path" : "/tmp/lintian-brush_0.148_source.changes"
            }
         ],
         "source_name" : "lintian-brush",
         "source_version" : "0.148"
      }
   ],
   "lintian_version" : "2.116.3"
}
OTHER BOGUS DATA
"#;
        let result = parse_lintian_output(output_str).unwrap();
        assert_eq!(
            result,
            serde_json::json!({
                "groups": [
                    {
                        "group_id": "lintian-brush_0.148",
                        "input_files": [
                            {
                                "hints": [],
                                "path": "lintian-brush_0.148.dsc"
                            },
                            {
                                "hints": [],
                                "path": "lintian-brush_0.148_source.buildinfo"
                            },
                            {
                                "hints": [],
                                "path": "lintian-brush_0.148_source.changes"
                            }
                        ],
                        "source_name": "lintian-brush",
                        "source_version": "0.148"
                    }
                ],
                "lintian_version": "2.116.3"
            })
        );
    }
}
