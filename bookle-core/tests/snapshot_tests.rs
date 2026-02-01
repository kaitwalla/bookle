//! Snapshot tests for bookle-core using insta
//!
//! These tests capture the output of various conversions to detect
//! unintended changes in the output format.

use bookle_core::encoder::encoder_for_format;
use bookle_core::types::{Block, Book, Chapter, Inline, Metadata, TableCell, TableData};

/// Helper to create a sample book for testing
fn sample_book() -> Book {
    let mut metadata = Metadata::new("The Art of Testing", "en");
    metadata.creator = vec!["Jane Doe".to_string(), "John Smith".to_string()];
    metadata.description = Some("A comprehensive guide to software testing.".to_string());
    metadata.publisher = Some("Test Press".to_string());
    metadata.subject = vec!["Testing".to_string(), "Software".to_string()];
    // Use fixed identifier for reproducible snapshots
    metadata.identifier = "test-book-identifier-12345".to_string();

    let mut book = Book::with_metadata(metadata);
    // Use a fixed UUID for reproducible snapshots
    book.id = uuid::Uuid::parse_str("12345678-1234-1234-1234-123456789abc").unwrap();

    // Chapter 1: Introduction
    let mut chapter1 = Chapter::new("Introduction");
    chapter1.content = vec![
        Block::Header {
            level: 2,
            content: vec![Inline::Text("Welcome".to_string())],
            anchor: Some("welcome".to_string()),
        },
        Block::Paragraph(vec![
            Inline::Text("This is a ".to_string()),
            Inline::Bold(vec![Inline::Text("comprehensive".to_string())]),
            Inline::Text(" guide to testing with ".to_string()),
            Inline::Italic(vec![Inline::Text("real examples".to_string())]),
            Inline::Text(".".to_string()),
        ]),
        Block::List {
            items: vec![
                vec![Block::Paragraph(vec![Inline::Text("First item".to_string())])],
                vec![Block::Paragraph(vec![Inline::Text("Second item".to_string())])],
                vec![Block::Paragraph(vec![
                    Inline::Text("Third item with ".to_string()),
                    Inline::Code("inline code".to_string()),
                ])],
            ],
            ordered: false,
        },
    ];
    book.add_chapter(chapter1);

    // Chapter 2: Advanced Topics
    let mut chapter2 = Chapter::new("Advanced Topics");
    chapter2.content = vec![
        Block::Paragraph(vec![
            Inline::Text("Let's explore some advanced concepts.".to_string()),
        ]),
        Block::CodeBlock {
            lang: Some("rust".to_string()),
            code: "fn main() {\n    println!(\"Hello, world!\");\n}".to_string(),
        },
        Block::Blockquote(vec![Block::Paragraph(vec![
            Inline::Text("Testing is the key to quality software.".to_string()),
        ])]),
        Block::ThematicBreak,
        Block::Paragraph(vec![
            Inline::Text("Visit our ".to_string()),
            Inline::Link {
                children: vec![Inline::Text("website".to_string())],
                url: "https://example.com".to_string(),
            },
            Inline::Text(" for more.".to_string()),
        ]),
    ];
    book.add_chapter(chapter2);

    // Chapter 3: Tables and Formatting
    let mut chapter3 = Chapter::new("Tables and Special Formatting");
    chapter3.content = vec![
        Block::Table(TableData {
            headers: vec![
                TableCell::new(vec![Inline::Text("Feature".to_string())]),
                TableCell::new(vec![Inline::Text("Status".to_string())]),
            ],
            rows: vec![
                vec![
                    TableCell::new(vec![Inline::Text("Unit Tests".to_string())]),
                    TableCell::new(vec![Inline::Bold(vec![Inline::Text("Complete".to_string())])]),
                ],
                vec![
                    TableCell::new(vec![Inline::Text("Integration Tests".to_string())]),
                    TableCell::new(vec![Inline::Italic(vec![Inline::Text("In Progress".to_string())])]),
                ],
            ],
        }),
        Block::Paragraph(vec![
            Inline::Text("Water formula: H".to_string()),
            Inline::Subscript(vec![Inline::Text("2".to_string())]),
            Inline::Text("O".to_string()),
        ]),
        Block::Paragraph(vec![
            Inline::Text("E = mc".to_string()),
            Inline::Superscript(vec![Inline::Text("2".to_string())]),
        ]),
        Block::Paragraph(vec![
            Inline::Strikethrough(vec![Inline::Text("Deprecated feature".to_string())]),
        ]),
    ];
    book.add_chapter(chapter3);

    book
}

#[test]
fn test_book_ir_json_snapshot() {
    let book = sample_book();
    let json = serde_json::to_string_pretty(&book).unwrap();
    insta::assert_snapshot!("book_ir_json", json);
}

#[test]
fn test_typst_output_snapshot() {
    let book = sample_book();
    let encoder = encoder_for_format("typ").unwrap();

    let mut output = Vec::new();
    encoder.encode(&book, &mut output).unwrap();
    let typst_source = String::from_utf8(output).unwrap();

    insta::assert_snapshot!("typst_output", typst_source);
}

#[test]
fn test_simple_paragraph_typst() {
    let mut book = Book::new("Simple Test", "en");
    book.id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

    let mut chapter = Chapter::new("Test Chapter");
    chapter.content = vec![Block::Paragraph(vec![
        Inline::Text("Hello ".to_string()),
        Inline::Bold(vec![Inline::Text("bold".to_string())]),
        Inline::Text(" and ".to_string()),
        Inline::Italic(vec![Inline::Text("italic".to_string())]),
        Inline::Text(".".to_string()),
    ])];
    book.add_chapter(chapter);

    let encoder = encoder_for_format("typ").unwrap();
    let mut output = Vec::new();
    encoder.encode(&book, &mut output).unwrap();
    let typst_source = String::from_utf8(output).unwrap();

    insta::assert_snapshot!("simple_paragraph_typst", typst_source);
}

#[test]
fn test_special_characters_escape() {
    let mut book = Book::new("Special Chars", "en");
    book.id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

    let mut chapter = Chapter::new("Escape Test");
    chapter.content = vec![Block::Paragraph(vec![Inline::Text(
        "Special chars: # * _ @ $ [ ]".to_string(),
    )])];
    book.add_chapter(chapter);

    let encoder = encoder_for_format("typ").unwrap();
    let mut output = Vec::new();
    encoder.encode(&book, &mut output).unwrap();
    let typst_source = String::from_utf8(output).unwrap();

    insta::assert_snapshot!("special_chars_typst", typst_source);
}

#[test]
fn test_nested_formatting() {
    let mut book = Book::new("Nested Formatting", "en");
    book.id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();

    let mut chapter = Chapter::new("Nested");
    chapter.content = vec![Block::Paragraph(vec![Inline::Bold(vec![
        Inline::Text("Bold with ".to_string()),
        Inline::Italic(vec![Inline::Text("nested italic".to_string())]),
        Inline::Text(" inside".to_string()),
    ])])];
    book.add_chapter(chapter);

    let encoder = encoder_for_format("typ").unwrap();
    let mut output = Vec::new();
    encoder.encode(&book, &mut output).unwrap();
    let typst_source = String::from_utf8(output).unwrap();

    insta::assert_snapshot!("nested_formatting_typst", typst_source);
}

#[test]
fn test_code_block_snapshot() {
    let mut book = Book::new("Code Block Test", "en");
    book.id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap();

    let mut chapter = Chapter::new("Code Examples");
    chapter.content = vec![
        Block::CodeBlock {
            lang: Some("python".to_string()),
            code: "def hello():\n    print('Hello, World!')".to_string(),
        },
        Block::CodeBlock {
            lang: None,
            code: "plain code block".to_string(),
        },
    ];
    book.add_chapter(chapter);

    let encoder = encoder_for_format("typ").unwrap();
    let mut output = Vec::new();
    encoder.encode(&book, &mut output).unwrap();
    let typst_source = String::from_utf8(output).unwrap();

    insta::assert_snapshot!("code_block_typst", typst_source);
}

#[test]
fn test_list_variations() {
    let mut book = Book::new("List Test", "en");
    book.id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000005").unwrap();

    let mut chapter = Chapter::new("Lists");
    chapter.content = vec![
        Block::List {
            items: vec![
                vec![Block::Paragraph(vec![Inline::Text("Unordered 1".to_string())])],
                vec![Block::Paragraph(vec![Inline::Text("Unordered 2".to_string())])],
            ],
            ordered: false,
        },
        Block::List {
            items: vec![
                vec![Block::Paragraph(vec![Inline::Text("Ordered 1".to_string())])],
                vec![Block::Paragraph(vec![Inline::Text("Ordered 2".to_string())])],
                vec![Block::Paragraph(vec![Inline::Text("Ordered 3".to_string())])],
            ],
            ordered: true,
        },
    ];
    book.add_chapter(chapter);

    let encoder = encoder_for_format("typ").unwrap();
    let mut output = Vec::new();
    encoder.encode(&book, &mut output).unwrap();
    let typst_source = String::from_utf8(output).unwrap();

    insta::assert_snapshot!("list_variations_typst", typst_source);
}
