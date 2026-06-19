use serde_json::Value;

pub fn normalize_model_json(model: &Value, ignore_fields: &[String]) -> Value {
    let mut root = model.clone();
    let mut ignore = ignore_fields.to_vec();
    if ignore.is_empty() {
        ignore = default_ignored_fields();
    } else {
        for item in default_ignored_fields() {
            if !ignore.contains(&item) {
                ignore.push(item);
            }
        }
    }

    normalize_value(&mut root, &ignore);
    root
}

fn normalize_value(value: &mut Value, ignore_fields: &[String]) {
    match value {
        Value::Object(map) => {
            for key in ignore_fields {
                if map.contains_key(key) {
                    map.insert(key.clone(), Value::String("<normalized>".to_string()));
                }
            }

            let keys = map.keys().cloned().collect::<Vec<_>>();
            for key in keys {
                if let Some(item) = map.get_mut(&key) {
                    if key.ends_with("_ms") {
                        *item = Value::Number(0_u64.into());
                    } else if key == "path" {
                        *item = Value::String("<normalized_path>".to_string());
                    } else if key == "hostname" {
                        *item = Value::String("<normalized_host>".to_string());
                    } else {
                        normalize_value(item, ignore_fields);
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_value(item, ignore_fields);
            }
        }
        _ => {}
    }
}

fn default_ignored_fields() -> Vec<String> {
    vec![
        "document_id".to_string(),
        "job_id".to_string(),
        "processed_at".to_string(),
        "uploaded_at".to_string(),
        "duration_ms".to_string(),
        "sha256".to_string(),
        "asset_id".to_string(),
        "path".to_string(),
        "hostname".to_string(),
        "model_warmup_ms".to_string(),
    ]
}
