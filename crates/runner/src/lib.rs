use std::collections::HashMap;

pub mod queue;

pub fn committer_env(committer: Option<&str>) -> HashMap<String, String> {
    let mut env = HashMap::new();
    if let Some(committer) = committer {
        let (user, email) = breezyshim::config::parse_username(committer);
        if !user.is_empty() {
            env.insert("DEBFULLNAME".to_string(), user.to_string());
            env.insert("GIT_COMMITTER_NAME".to_string(), user.to_string());
            env.insert("GIT_AUTHOR_NAME".to_string(), user.to_string());
        }
        if !email.is_empty() {
            env.insert("DEBEMAIL".to_string(), email.to_string());
            env.insert("GIT_COMMITTER_EMAIL".to_string(), email.to_string());
            env.insert("GIT_AUTHOR_EMAIL".to_string(), email.to_string());
            env.insert("EMAIL".to_string(), email.to_string());
        }
        env.insert("COMMITTER".to_string(), committer.to_string());
        env.insert("BRZ_EMAIL".to_string(), committer.to_string());
    }
    env
}

pub fn is_log_filename(name: &str) -> bool {
    let parts = name.split('.').collect::<Vec<_>>();
    if parts.last() == Some(&"log") {
        true
    } else if parts.len() == 3 {
        let mut rev = parts.iter().rev();
        rev.next().unwrap().chars().all(char::is_numeric) && rev.next() == Some(&"log")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_committer_env() {
        let committer = Some("John Doe <john@example.com>");

        let expected = maplit::hashmap! {
            "DEBFULLNAME".to_string() => "John Doe".to_string(),
            "GIT_COMMITTER_NAME".to_string() => "John Doe".to_string(),
            "GIT_AUTHOR_NAME".to_string() => "John Doe".to_string(),
            "DEBEMAIL".to_string() => "john@example.com".to_string(),
            "GIT_COMMITTER_EMAIL".to_string() => "john@example.com".to_string(),
            "GIT_AUTHOR_EMAIL".to_string() => "john@example.com".to_string(),
            "EMAIL".to_string() => "john@example.com".to_string(),
            "COMMITTER".to_string() => "John Doe <john@example.com>".to_string(),
            "BRZ_EMAIL".to_string() => "John Doe <john@example.com>".to_string(),
        };

        assert_eq!(committer_env(committer), expected);
    }

    #[test]
    fn test_committer_env_no_committer() {
        let committer = None;

        let expected = maplit::hashmap! {};

        assert_eq!(committer_env(committer), expected);
    }

    #[test]
    fn is_log_filename_test() {
        assert!(is_log_filename("foo.log"));
        assert!(is_log_filename("foo.log.1"));
        assert!(is_log_filename("foo.1.log"));
        assert!(!is_log_filename("foo.1"));
        assert!(!is_log_filename("foo.1.log.1"));
        assert!(!is_log_filename("foo.1.notlog"));
        assert!(!is_log_filename("foo.log.notlog"));
    }
}
