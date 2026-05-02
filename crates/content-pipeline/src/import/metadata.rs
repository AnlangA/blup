/// Detect language from text content
pub fn detect_language(text: &str) -> Option<String> {
    // Simple heuristic based on character ranges
    let total_chars = text.chars().count() as f64;
    if total_chars == 0.0 {
        return None;
    }

    // Count characters by Unicode range
    let mut cjk_count = 0;
    let mut korean_count = 0;
    let mut cyrillic_count = 0;
    let mut arabic_count = 0;
    let mut devanagari_count = 0;

    for c in text.chars() {
        let code = c as u32;
        if code < 128 {
            // ASCII — not separately tracked
        } else if (0x4E00..=0x9FFF).contains(&code)
            || (0x3400..=0x4DBF).contains(&code)
            || (0x3000..=0x303F).contains(&code)
            || (0xFF00..=0xFFEF).contains(&code)
        {
            cjk_count += 1;
        } else if (0xAC00..=0xD7AF).contains(&code) || (0x1100..=0x11FF).contains(&code) {
            korean_count += 1;
        } else if (0x0400..=0x04FF).contains(&code) {
            cyrillic_count += 1;
        } else if (0x0600..=0x06FF).contains(&code) || (0xFB50..=0xFDFF).contains(&code) {
            arabic_count += 1;
        } else if (0x0900..=0x097F).contains(&code) {
            devanagari_count += 1;
        }
    }

    let threshold = total_chars * 0.3;

    // Check Korean first (before CJK)
    if korean_count as f64 > threshold {
        return Some("ko".to_string());
    }

    if cjk_count as f64 > threshold {
        // Distinguish between Chinese and Japanese
        let hiragana_katakana = text
            .chars()
            .filter(|c| {
                let code = *c as u32;
                (0x3040..=0x309F).contains(&code) || (0x30A0..=0x30FF).contains(&code)
            })
            .count();

        if hiragana_katakana > 0 {
            return Some("ja".to_string());
        }

        return Some("zh".to_string());
    }

    if cyrillic_count as f64 > threshold {
        return Some("ru".to_string());
    }

    if arabic_count as f64 > threshold {
        return Some("ar".to_string());
    }

    if devanagari_count as f64 > threshold {
        return Some("hi".to_string());
    }

    // Default to English for Latin script
    Some("en".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english() {
        let text = "Hello, this is a test document in English.";
        assert_eq!(detect_language(text), Some("en".to_string()));
    }

    #[test]
    fn test_chinese() {
        let text = "这是一份中文测试文档。";
        assert_eq!(detect_language(text), Some("zh".to_string()));
    }

    #[test]
    fn test_japanese() {
        let text = "これは日本語のテスト文書です。";
        assert_eq!(detect_language(text), Some("ja".to_string()));
    }

    #[test]
    fn test_korean() {
        let text = "이것은 한국어 테스트 문서입니다.";
        assert_eq!(detect_language(text), Some("ko".to_string()));
    }

    #[test]
    fn test_russian() {
        let text = "Это тестовый документ на русском языке.";
        assert_eq!(detect_language(text), Some("ru".to_string()));
    }

    #[test]
    fn test_empty() {
        assert_eq!(detect_language(""), None);
    }
}
