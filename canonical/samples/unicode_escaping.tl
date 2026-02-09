# Canonical test: Unicode and escape sequences
# Tests: all escape sequences, various Unicode ranges, edge cases

# =============================================================================
# Escape Sequences
# =============================================================================

# Basic escape sequences
escape_newline: "line1\nline2"
escape_tab: "col1\tcol2"
escape_carriage: "old\rstyle"
escape_quote: "say \"hello\""
escape_backslash: "path\\to\\file"

# Multiple escapes combined
escape_all: "start\n\t\"middle\"\r\\end"

# Escapes at boundaries
escape_at_start: "\nstarting with newline"
escape_at_end: "ending with newline\n"
escape_only: "\n\t\r\"\\"

# =============================================================================
# Unicode: CJK (Chinese, Japanese, Korean)
# =============================================================================

unicode_japanese: "日本語テスト"
unicode_chinese_simplified: "中文简体测试"
unicode_chinese_traditional: "中文繁體測試"
unicode_korean: "한국어 테스트"

# =============================================================================
# Unicode: European scripts
# =============================================================================

unicode_greek: "Ελληνικά τεστ"
unicode_cyrillic: "Русский тест"
unicode_arabic: "اختبار عربي"
unicode_hebrew: "בדיקה עברית"

# =============================================================================
# Unicode: Indic scripts
# =============================================================================

unicode_hindi: "हिंदी परीक्षण"
unicode_tamil: "தமிழ் சோதனை"
unicode_thai: "การทดสอบภาษาไทย"

# =============================================================================
# Unicode: Emoji and symbols
# =============================================================================

emoji_faces: "😀😃😄😁😆"
emoji_flags: "🇺🇸🇬🇧🇯🇵🇩🇪🇫🇷"
emoji_compound: "👨‍👩‍👧‍👦"
emoji_skin_tone: "👋🏻👋🏼👋🏽👋🏾👋🏿"
symbols_math: "∑∏∫∂∇"
symbols_currency: "€£¥₹₽"
symbols_arrows: "←→↑↓↔↕"

# =============================================================================
# Unicode: Special characters
# =============================================================================

# Combining characters (diacritics)
combining_acute: "café résumé naïve"
combining_umlaut: "über größe"
combining_tilde: "São Paulo español"

# Zero-width characters (useful to test they're preserved)
zero_width_joiner: "👨‍💻"
# Note: This emoji uses ZWJ to combine 👨 + 💻

# =============================================================================
# Unicode in different contexts
# =============================================================================

# Unicode in object keys
unicode_keys: {
    日本語: "Japanese key",
    ключ: "Russian key",
    مفتاح: "Arabic key",
}

# Unicode in arrays
unicode_array: ["один", "два", "три", "四", "五", "六"]

# Mixed content
mixed_content: "Hello 世界! Привет мир! مرحبا بالعالم!"

# =============================================================================
# Edge cases
# =============================================================================

# Empty and whitespace
empty_string: ""
only_spaces: "   "
only_tabs: "\t\t\t"
only_newlines: "\n\n\n"

# Repeated characters
repeated_emoji: "🔥🔥🔥🔥🔥"
repeated_escape: "\n\n\n\n\n"

# Long strings
long_unicode: "这是一个很长的中文字符串用于测试长字符串的处理能力以确保系统能够正确处理超长的Unicode文本内容"

# Quotes inside quotes (double escaping)
nested_quotes: "He said \"she said \\\"hello\\\"\""

# Path-like strings
windows_path: "C:\\Users\\名前\\Documents\\文件"
url_encoded: "https://example.com/path?q=%E4%B8%AD%E6%96%87"

# JSON-like content with escapes
json_string: "{\"name\": \"テスト\", \"value\": \"значение\"}"
