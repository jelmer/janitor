// Core utility functions migrated from py/janitor/__init__.py

use std::collections::HashMap;

/// The version of the janitor package
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Regex pattern for valid campaign names
pub const CAMPAIGN_REGEX: &str = r"[a-z0-9-]+";

/// Split environment variables from a command string.
/// 
/// This function parses a command string that may start with environment variable
/// assignments (in the form KEY=VALUE) and separates them from the actual command.
/// 
/// # Arguments
/// * `command` - The command string to parse
/// 
/// # Returns
/// A tuple containing (environment_variables, remaining_command)
/// 
/// # Examples
/// ```
/// use janitor::utils::splitout_env;
/// use std::collections::HashMap;
/// 
/// let (env, cmd) = splitout_env("FOO=bar BAZ=qux echo hello");
/// assert_eq!(env.get("FOO"), Some(&"bar".to_string()));
/// assert_eq!(env.get("BAZ"), Some(&"qux".to_string()));
/// assert_eq!(cmd, "echo hello");
/// ```
pub fn splitout_env(command: &str) -> (HashMap<String, String>, String) {
    // Use shlex to properly split the command while respecting quotes
    let args = match shlex::split(command) {
        Some(args) => args,
        None => {
            // If shlex parsing fails, return empty env and original command
            return (HashMap::new(), command.to_string());
        }
    };
    
    let mut env = HashMap::new();
    let mut remaining_args = args;
    
    // Process arguments from the beginning, collecting environment variables
    while !remaining_args.is_empty() && remaining_args[0].contains('=') {
        let arg = remaining_args.remove(0);
        if let Some((key, value)) = arg.split_once('=') {
            env.insert(key.to_string(), value.to_string());
        } else {
            // If split failed, this isn't a valid env var, put it back and break
            remaining_args.insert(0, arg);
            break;
        }
    }
    
    // Reconstruct the remaining command
    let remaining_command = if remaining_args.is_empty() {
        String::new()
    } else {
        shlex::try_join(remaining_args.iter().map(|s| s.as_str()))
            .unwrap_or_else(|_| remaining_args.join(" "))
    };
    
    (env, remaining_command)
}

/// Get the default user agent string for HTTP requests
pub fn default_user_agent() -> String {
    format!("janitor/{}", VERSION)
}

/// Get a service-specific user agent string
pub fn service_user_agent(service_name: &str) -> String {
    format!("janitor-{}/{}", service_name, VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splitout_env_no_env() {
        let (env, cmd) = splitout_env("echo hello world");
        assert!(env.is_empty());
        assert_eq!(cmd, "echo hello world");
    }

    #[test]
    fn test_splitout_env_single_env() {
        let (env, cmd) = splitout_env("FOO=bar echo hello");
        assert_eq!(env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(cmd, "echo hello");
    }

    #[test]
    fn test_splitout_env_multiple_env() {
        let (env, cmd) = splitout_env("FOO=bar BAZ=qux echo hello world");
        assert_eq!(env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(env.get("BAZ"), Some(&"qux".to_string()));
        assert_eq!(cmd, "echo hello world");
    }

    #[test]
    fn test_splitout_env_quoted_values() {
        let (env, cmd) = splitout_env(r#"FOO="bar baz" echo hello"#);
        assert_eq!(env.get("FOO"), Some(&"bar baz".to_string()));
        assert_eq!(cmd, "echo hello");
    }

    #[test]
    fn test_splitout_env_equals_in_value() {
        let (env, cmd) = splitout_env("URL=http://example.com/path?a=b echo test");
        assert_eq!(env.get("URL"), Some(&"http://example.com/path?a=b".to_string()));
        assert_eq!(cmd, "echo test");
    }

    #[test]
    fn test_splitout_env_only_env() {
        let (env, cmd) = splitout_env("FOO=bar BAZ=qux");
        assert_eq!(env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(env.get("BAZ"), Some(&"qux".to_string()));
        assert_eq!(cmd, "");
    }

    #[test]
    fn test_splitout_env_empty_command() {
        let (env, cmd) = splitout_env("");
        assert!(env.is_empty());
        assert_eq!(cmd, "");
    }

    #[test]
    fn test_splitout_env_equals_in_command() {
        let (env, cmd) = splitout_env("FOO=bar echo a=b");
        assert_eq!(env.get("FOO"), Some(&"bar".to_string()));
        // The shlex library quotes arguments containing = which is correct behavior
        assert!(cmd == "echo a=b" || cmd == "echo 'a=b'");
    }

    #[test]
    fn test_campaign_regex_pattern() {
        // Test that the regex pattern is valid
        let regex = regex::Regex::new(&format!("^{}$", CAMPAIGN_REGEX)).unwrap();
        
        // Valid campaign names
        assert!(regex.is_match("lintian-fixes"));
        assert!(regex.is_match("debian-janitor"));
        assert!(regex.is_match("fresh-releases"));
        assert!(regex.is_match("a"));
        assert!(regex.is_match("test123"));
        
        // Invalid campaign names
        assert!(!regex.is_match("INVALID"));
        assert!(!regex.is_match("invalid_name"));
        assert!(!regex.is_match("invalid name"));
        assert!(!regex.is_match(""));
    }

    #[test]
    fn test_user_agent_functions() {
        let default_ua = default_user_agent();
        assert!(default_ua.starts_with("janitor/"));
        assert!(default_ua.contains(VERSION));
        
        let service_ua = service_user_agent("runner");
        assert!(service_ua.starts_with("janitor-runner/"));
        assert!(service_ua.contains(VERSION));
    }
}