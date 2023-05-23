use select::document::Document;
use select::predicate::{And, Attr, Name};
use serde_json::Value;

pub fn parse_plain_text_body(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line == &"Reply to this email directly or view it on GitHub:" {
            return Some(lines[i + 1].split('#').next().unwrap().to_string());
        }
        if line == &"For more details, see:"
            && lines[i + 1].starts_with("https://code.launchpad.net/")
        {
            return Some(lines[i + 1].to_string());
        }
        if let Some((field, value)) = line.split_once(':') {
            if field.to_lowercase() == "merge request url" {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn parse_json_ld(ld: &Value) -> Option<String> {
    match ld {
        Value::Array(ld_array) => ld_array.iter().find_map(parse_json_ld),
        Value::Object(ld_object) => {
            let context = ld_object.get("@context")?;
            if context == &Value::String("https://schema.org".to_string())
                || context == &Value::String("http://schema.org".to_string())
            {
                let type_ = ld_object.get("@type")?;
                if type_ == &Value::String("EmailMessage".to_string()) {
                    let action = ld_object
                        .get("action")
                        .or_else(|| ld_object.get("potentialAction"))?;
                    let action_type = action.get("@type")?;
                    if action_type == &Value::String("ViewAction".to_string()) {
                        let url = action.get("url")?;
                        return Some(url.as_str().unwrap().split('#').next().unwrap().to_string());
                    }
                }
            }
            None
        }
        _ => None,
    }
}

pub fn parse_html_body(contents: &str) -> Option<String> {
    let document = Document::from(contents);
    let ld = document
        .find(And(Name("script"), Attr("type", "application/ld+json")))
        .next()?;
    if let Ok(ld_json) = serde_json::from_str(ld.text().as_str()) {
        parse_json_ld(&ld_json)
    } else {
        None
    }
}
