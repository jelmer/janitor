use log::debug;
use serde_json;
use std::path::Path;
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

impl std::error::Error for Error {}

fn parse_lintian_output(text: &str) -> Result<serde_json::Value, serde_json::Error> {
    let lines: Vec<&str> = text.trim().split('\n').collect();
    let mut joined_lines: Vec<&str> = Vec::new();
    for line in lines {
        joined_lines.push(line);
        if line == "}" {
            break;
        }
    }

    let joined_str = joined_lines.join("\n");
    let mut result: serde_json::Value = serde_json::from_str(&joined_str)?;

    // Strip irrelevant directory information
    if let Some(groups) = result.get_mut("groups") {
        if let Some(groups_array) = groups.as_array_mut() {
            for group in groups_array {
                if let Some(input_files) = group.get_mut("input_files") {
                    if let Some(input_files_array) = input_files.as_array_mut() {
                        for input_file in input_files_array {
                            if let Some(path) = input_file.get_mut("path") {
                                if let Some(path_str) = path.as_str() {
                                    let basename =
                                        Path::new(path_str).file_name().unwrap().to_str().unwrap();
                                    *path = serde_json::Value::String(basename.to_owned());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}

pub fn run_lintian(
    output_directory: &str,
    changes_names: Vec<&str>,
    profile: Option<&str>,
    suppress_tags: Option<Vec<&str>>,
) -> Result<serde_json::Value, Error> {
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
