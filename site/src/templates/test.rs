#[cfg(test)]
mod tests {
    use crate::templates::setup_templates;
    use tera::{Context, Tera};
    use chrono::Utc;
    use crate::config::SiteConfig;

    fn create_test_tera() -> Tera {
        let mut tera = Tera::default();
        
        // Add test templates
        tera.add_raw_template("layout.html", r#"
<!DOCTYPE html>
<html>
<head>
    <title>{% block page_title %}{% endblock %}</title>
</head>
<body>
    {% block body %}{% endblock %}
</body>
</html>
        "#).unwrap();
        
        tera.add_raw_template("test.html", r#"
{% extends "layout.html" %}
{% block page_title %}Test Page{% endblock %}
{% block body %}
    <h1>{{ title }}</h1>
    <p>{{ content }}</p>
{% endblock %}
        "#).unwrap();
        
        tera
    }

    #[test]
    fn test_template_inheritance() {
        let tera = create_test_tera();
        let mut context = Context::new();
        context.insert("title", "Hello");
        context.insert("content", "World");
        
        let result = tera.render("test.html", &context).unwrap();
        assert!(result.contains("<title>Test Page</title>"));
        assert!(result.contains("<h1>Hello</h1>"));
        assert!(result.contains("<p>World</p>"));
    }

    #[test]
    fn test_macros() {
        let mut tera = Tera::default();
        
        tera.add_raw_template("macros.html", r#"
{% macro greeting(name) %}
    Hello, {{ name }}!
{% endmacro %}
        "#).unwrap();
        
        tera.add_raw_template("test_macro.html", r#"
{% import "macros.html" as m %}
{{ m::greeting(name="World") }}
        "#).unwrap();
        
        let context = Context::new();
        let result = tera.render("test_macro.html", &context).unwrap();
        assert!(result.contains("Hello, World!"));
    }

    #[test]
    fn test_filters() {
        let config = SiteConfig::default();
        let tera = setup_templates(&config).unwrap();
        
        let mut context = Context::new();
        context.insert("path", "/some/path/file.txt");
        context.insert("timestamp", &Utc::now());
        
        // Test basename filter
        let template = "{{ path | basename }}";
        let mut test_tera = tera.clone();
        test_tera.add_raw_template("test_filter", template).unwrap();
        let result = test_tera.render("test_filter", &context).unwrap();
        assert_eq!(result.trim(), "file.txt");
    }

    #[test]
    fn test_template_compilation() {
        // Verify all templates compile without errors
        let config = SiteConfig::default();
        let result = setup_templates(&config);
        
        // This will fail if templates have syntax errors
        assert!(result.is_ok(), "Templates should compile without errors");
    }
}