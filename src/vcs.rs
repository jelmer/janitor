pub fn is_authenticated_url(url: &url::Url) -> bool {
    ["git+ssh", "bzr+ssh"].contains(&url.scheme())
}

#[cfg(test)]
mod is_authenticated_url_tests {
    #[test]
    fn test_simple() {
        assert!(super::is_authenticated_url(
            &url::Url::parse("git+ssh://example.com").unwrap()
        ));
        assert!(super::is_authenticated_url(
            &url::Url::parse("bzr+ssh://example.com").unwrap()
        ));
        assert!(!super::is_authenticated_url(
            &url::Url::parse("http://example.com").unwrap()
        ));
    }
}
