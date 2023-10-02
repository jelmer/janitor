use super::parse_email;


#[test]
fn test_parse_github_merged_email() {
    let email = include_bytes!("../tests/data/github-merged-email.txt");

    assert_eq!(Some("https://github.com/UbuntuBudgie/budgie-desktop/pull/78"), parse_email(std::io::Cursor::new(email)).as_deref());
}

#[test]
fn test_parse_gitlab_merged_email() {
    let email = include_bytes!("../tests/data/gitlab-merged-email.txt");

    assert_eq!(Some("https://salsa.debian.org/debian/pkg-lojban-common/-/merge_requests/2"), parse_email(std::io::Cursor::new(email)).as_deref());
}
