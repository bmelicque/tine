// Minimal highlight.js grammar for Tine.
hljs.registerLanguage("tine", function (hljs) {
    const KEYWORDS = {
        keyword:
            "let mut const pub fn struct enum trait impl use if else for in break continue return match true false",
        built_in: "bool int float str Option",
    };

    return {
        name: "Tine",
        keywords: KEYWORDS,
        contains: [
            hljs.COMMENT("//"),
            hljs.QUOTE_STRING_MODE,
            {
                className: "number",
                variants: [
                    { begin: "\\b\\d+\\.\\d*\\b" }, // 9.99, 9.
                    { begin: "\\b\\d+\\b" }, // 42
                ],
                relevance: 0,
            },
            {
                className: "title.function",
                begin: /[A-Za-z_]\w*(?=\()/,
            },
            {
                className: "type",
                begin: /\b[A-Z][A-Za-z0-9_]*\b/,
            },
        ],
    };
});

document.querySelectorAll("code.language-tine").forEach((block) => {
    hljs.highlightBlock(block);
});