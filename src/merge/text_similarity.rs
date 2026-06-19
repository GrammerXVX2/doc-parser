fn normalize_text(input: &str) -> String {
    let lowered = input.to_ascii_lowercase();
    let mut normalized = String::with_capacity(lowered.len());
    let mut prev_space = false;

    for c in lowered.chars() {
        if c.is_ascii_alphanumeric() {
            normalized.push(c);
            prev_space = false;
        } else if !prev_space && !normalized.is_empty() {
                normalized.push(' ');
                prev_space = true;
        }
    }

    normalized.trim().to_string()
}

fn compact_text(input: &str) -> String {
    input.chars().filter(|c| !c.is_whitespace()).collect()
}

fn token_overlap_ratio(a: &str, b: &str) -> f32 {
    let a_tokens = a.split_whitespace().collect::<Vec<_>>();
    let b_tokens = b.split_whitespace().collect::<Vec<_>>();
    if a_tokens.is_empty() || b_tokens.is_empty() {
        return 0.0;
    }

    let mut overlap = 0_usize;
    for token in &a_tokens {
        if b_tokens.contains(token) {
            overlap += 1;
        }
    }

    let denom = a_tokens.len().max(b_tokens.len()) as f32;
    overlap as f32 / denom
}

pub fn text_similarity(a: &str, b: &str) -> f32 {
    let an = normalize_text(a);
    let bn = normalize_text(b);

    if an.is_empty() || bn.is_empty() {
        return 0.0;
    }

    if an == bn {
        return 1.0;
    }

    let ac = compact_text(&an);
    let bc = compact_text(&bn);
    if !ac.is_empty() && ac == bc {
        return 0.95;
    }

    if an.contains(&bn) || bn.contains(&an) {
        return 0.85;
    }

    if ac.contains(&bc) || bc.contains(&ac) {
        return 0.85;
    }

    token_overlap_ratio(&an, &bn).clamp(0.0, 1.0)
}
