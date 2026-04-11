use super::*;

#[test]
fn test_parse_github_merged_email() {
    let email = include_bytes!("../tests/data/github-merged-email.txt");

    assert_eq!(
        Some("https://github.com/UbuntuBudgie/budgie-desktop/pull/78"),
        parse_email(std::io::Cursor::new(email)).as_deref()
    );
}

#[test]
fn test_parse_gitlab_merged_email() {
    let email = include_bytes!("../tests/data/gitlab-merged-email.txt");

    assert_eq!(
        Some("https://salsa.debian.org/debian/pkg-lojban-common/-/merge_requests/2"),
        parse_email(std::io::Cursor::new(email)).as_deref()
    );
}

#[test]
fn test_parse_plain_text_github_reply() {
    let text = "Some review comments\n\
                Reply to this email directly or view it on GitHub:\n\
                https://github.com/owner/repo/pull/42#issuecomment-12345\n";
    assert_eq!(
        parse_plain_text_body(text),
        Some("https://github.com/owner/repo/pull/42".to_string())
    );
}

#[test]
fn test_parse_plain_text_launchpad() {
    let text = "Someone proposed a merge.\n\
                For more details, see:\n\
                https://code.launchpad.net/~user/project/+branch/trunk/+merge/12345\n";
    assert_eq!(
        parse_plain_text_body(text),
        Some("https://code.launchpad.net/~user/project/+branch/trunk/+merge/12345".to_string())
    );
}

#[test]
fn test_parse_plain_text_merge_request_url_field() {
    let text = "Subject: Review request\n\
                Merge Request URL: https://code.example.com/mr/99\n\
                Some other text\n";
    assert_eq!(
        parse_plain_text_body(text),
        Some("https://code.example.com/mr/99".to_string())
    );
}

#[test]
fn test_parse_plain_text_merge_request_url_field_case_insensitive() {
    let text = "merge request url: https://code.example.com/mr/100\n";
    assert_eq!(
        parse_plain_text_body(text),
        Some("https://code.example.com/mr/100".to_string())
    );
}

#[test]
fn test_parse_plain_text_no_match() {
    let text = "Just a regular email with no merge proposal URLs.\n\
                Nothing to see here.\n";
    assert_eq!(parse_plain_text_body(text), None);
}

#[test]
fn test_parse_html_json_ld_view_action() {
    let html = r#"<html>
    <head>
        <script type="application/ld+json">
        {
            "@context": "https://schema.org",
            "@type": "EmailMessage",
            "potentialAction": {
                "@type": "ViewAction",
                "url": "https://github.com/owner/repo/pull/5#event-999"
            }
        }
        </script>
    </head>
    <body>Email body</body>
    </html>"#;
    assert_eq!(
        parse_html_body(html),
        Some("https://github.com/owner/repo/pull/5".to_string())
    );
}

#[test]
fn test_parse_html_json_ld_http_schema() {
    let html = r#"<html>
    <head>
        <script type="application/ld+json">
        {
            "@context": "http://schema.org",
            "@type": "EmailMessage",
            "action": {
                "@type": "ViewAction",
                "url": "https://github.com/owner/repo/pull/10"
            }
        }
        </script>
    </head>
    <body>Email body</body>
    </html>"#;
    assert_eq!(
        parse_html_body(html),
        Some("https://github.com/owner/repo/pull/10".to_string())
    );
}

#[test]
fn test_parse_html_json_ld_array() {
    let html = r#"<html>
    <head>
        <script type="application/ld+json">
        [{
            "@context": "https://schema.org",
            "@type": "EmailMessage",
            "potentialAction": {
                "@type": "ViewAction",
                "url": "https://github.com/owner/repo/pull/7"
            }
        }]
        </script>
    </head>
    <body>Email body</body>
    </html>"#;
    assert_eq!(
        parse_html_body(html),
        Some("https://github.com/owner/repo/pull/7".to_string())
    );
}

#[test]
fn test_parse_html_no_json_ld() {
    let html = r#"<html><body>Just a regular email</body></html>"#;
    assert_eq!(parse_html_body(html), None);
}

#[test]
fn test_parse_html_wrong_type() {
    let html = r#"<html>
    <head>
        <script type="application/ld+json">
        {
            "@context": "https://schema.org",
            "@type": "BlogPosting",
            "url": "https://example.com/blog/1"
        }
        </script>
    </head>
    <body>Email body</body>
    </html>"#;
    assert_eq!(parse_html_body(html), None);
}

#[test]
fn test_parse_html_wrong_action_type() {
    let html = r#"<html>
    <head>
        <script type="application/ld+json">
        {
            "@context": "https://schema.org",
            "@type": "EmailMessage",
            "potentialAction": {
                "@type": "ConfirmAction",
                "url": "https://example.com/confirm"
            }
        }
        </script>
    </head>
    <body>Email body</body>
    </html>"#;
    assert_eq!(parse_html_body(html), None);
}
