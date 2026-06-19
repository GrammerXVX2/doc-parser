use regex::Regex;

pub fn decode_xml_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

pub fn strip_xml_tags(input: &str) -> String {
    match Regex::new(r"(?s)<[^>]+>") {
        Ok(re) => decode_xml_entities(&re.replace_all(input, "").to_string()),
        Err(_) => decode_xml_entities(input),
    }
}

pub fn extract_xml_texts(xml: &str, tag_name: &str) -> Vec<String> {
    let escaped = regex::escape(tag_name);
    let pattern = format!(r"(?s)<{escaped}[^>]*>(.*?)</{escaped}>");
    let Ok(re) = Regex::new(&pattern) else {
        return vec![];
    };

    re.captures_iter(xml)
        .filter_map(|cap| cap.get(1).map(|m| decode_xml_entities(m.as_str())))
        .collect()
}
