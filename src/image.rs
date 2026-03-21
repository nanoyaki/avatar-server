pub fn fallback(sha256: &str, is_tiny: bool) -> String {
    let color = format!(
        "#{}",
        &sha256[..sha256.char_indices().nth(6).expect("test").0]
    );
    let size = if is_tiny { "32" } else { "128" };
    let svg = format!(
        "<svg width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"{}\" height=\"{}\" fill=\"{}\"/></svg>",
        size, size, size, size, size, size, color
    );

    return svg;
}
