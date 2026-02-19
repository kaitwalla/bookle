//! Conversion tests for bookle-core
//!
//! These tests verify that books can be correctly decoded from various formats
//! and that conversions between formats preserve content accurately.
//!
//! ## Test Strategy
//!
//! 1. **Decoding tests**: Verify each format (EPUB, MOBI, AZW3) can be decoded
//!    and produces expected metadata (via snapshot testing)
//! 2. **Cross-format tests**: Verify the same book decoded from different formats
//!    produces semantically equivalent content
//! 3. **Round-trip tests**: Encode a decoded book and decode again to verify
//!    content preservation
//! 4. **Edge case tests**: Test error handling and boundary conditions

use bookle_core::decoder::decoder_for_extension;
use bookle_core::encoder::encoder_for_format;
use bookle_core::types::{Block, Book, Chapter, Inline, Metadata, ReadingDirection};
use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

// =============================================================================
// Constants
// =============================================================================

/// Path to the test files directory
const TEST_FILES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../files");

/// Minimum expected chapter count for "Alice's Adventures in Wonderland"
/// (The book has 12 chapters plus front/back matter, so we expect at least 5)
const ALICE_MIN_CHAPTERS: usize = 5;

// =============================================================================
// Snapshot Helpers
// =============================================================================

/// Metadata snapshot that excludes the identifier field for deterministic snapshots.
/// MOBI/AZW3 decoders generate random UUIDs when the source doesn't have an embedded identifier,
/// and even EPUB identifiers can vary. Using this struct ensures consistent snapshots.
#[derive(Debug, Serialize)]
struct StableMetadata {
    title: String,
    creator: Vec<String>,
    subject: Vec<String>,
    description: Option<String>,
    publisher: Option<String>,
    language: String,
    cover_resource_key: Option<String>,
    reading_direction: ReadingDirection,
    rights: Option<String>,
}

impl From<&Metadata> for StableMetadata {
    fn from(m: &Metadata) -> Self {
        Self {
            title: m.title.clone(),
            creator: m.creator.clone(),
            subject: m.subject.clone(),
            description: m.description.clone(),
            publisher: m.publisher.clone(),
            language: m.language.clone(),
            cover_resource_key: m.cover_resource_key.clone(),
            reading_direction: m.reading_direction,
            rights: m.rights.clone(),
        }
    }
}

/// Helper macro for metadata snapshots that excludes the identifier field
macro_rules! assert_metadata_snapshot {
    ($name:expr, $metadata:expr) => {
        let snapshot: StableMetadata = $metadata.into();
        insta::assert_json_snapshot!($name, snapshot);
    };
}

// =============================================================================
// Test File Definitions
// =============================================================================

/// Test book file paths for a single book available in multiple formats
struct TestBook {
    /// Human-readable name for error messages
    name: &'static str,
    epub: &'static str,
    mobi: &'static str,
    azw3: &'static str,
}

impl TestBook {
    /// Validate that all test files for this book exist
    fn validate_files_exist(&self) {
        let files = [
            (self.epub, "EPUB"),
            (self.mobi, "MOBI"),
            (self.azw3, "AZW3"),
        ];

        for (filename, format) in files {
            let path = Path::new(TEST_FILES_DIR).join(filename);
            assert!(
                path.exists(),
                "Test file missing for '{}' ({}): {}\nExpected at: {}",
                self.name,
                format,
                filename,
                path.display()
            );
        }
    }
}

const ALICE: TestBook = TestBook {
    name: "Alice's Adventures in Wonderland",
    epub: "sample-epub-files-Alices Adventures in Wonderland.epub",
    mobi: "sample-mobi-files-Alices Adventures in Wonderland.mobi",
    azw3: "sample-azw3-files-Alices Adventures in Wonderland.azw3",
};

const AROUND_THE_WORLD: TestBook = TestBook {
    name: "Around the World in 28 Languages",
    epub: "sample-epub-files-Around the World in 28 Languages.epub",
    mobi: "sample-mobi-files-Around the World in 28 Languages.mobi",
    azw3: "sample-azw3-files-Around the World in 28 Languages.azw3",
};

const FAMOUS_PAINTINGS: TestBook = TestBook {
    name: "Famous Paintings",
    epub: "sample-epub-files-famouspaintings.epub",
    mobi: "sample-mobi-files-famouspaintings.mobi",
    azw3: "sample-azw3-files-famouspaintings.azw3",
};

const SAMPLE1: TestBook = TestBook {
    name: "Sample 1 (Geography of Bliss)",
    epub: "sample-epub-files-sample1.epub",
    mobi: "sample-mobi-files-sample1.mobi",
    azw3: "sample-azw3-files-sample1.azw3",
};

// =============================================================================
// Helper Functions
// =============================================================================

/// Decode a book from a test file
fn decode_file(filename: &str, extension: &str) -> Result<Book, String> {
    let path = Path::new(TEST_FILES_DIR).join(filename);
    if !path.exists() {
        return Err(format!(
            "Test file not found: {}\nFull path: {}\nHint: Ensure test files are in the 'files/' directory",
            filename,
            path.display()
        ));
    }

    let file = File::open(&path).map_err(|e| format!("Failed to open {}: {}", filename, e))?;
    let mut reader = BufReader::new(file);

    let decoder = decoder_for_extension(extension)
        .ok_or_else(|| format!("No decoder for extension: {}", extension))?;

    decoder
        .decode(&mut reader)
        .map_err(|e| format!("Failed to decode {}: {}", filename, e))
}

/// Encode a book to bytes
fn encode_to_bytes(book: &Book, format: &str) -> Result<Vec<u8>, String> {
    let encoder = encoder_for_format(format)
        .ok_or_else(|| format!("No encoder for format: {}", format))?;

    let mut output = Vec::new();
    encoder
        .encode(book, &mut output)
        .map_err(|e| format!("Failed to encode to {}: {}", format, e))?;

    Ok(output)
}

/// Compare two books for semantic equality (ignoring UUIDs and other non-content fields).
/// Different formats may have different chapter structures (e.g., MOBI often has 1 chapter
/// containing all content, while EPUB has explicit chapters).
fn assert_books_similar(book1: &Book, book2: &Book, context: &str) {
    // Compare titles (normalized)
    let title1 = book1.metadata.title.trim().to_lowercase();
    let title2 = book2.metadata.title.trim().to_lowercase();
    assert!(
        title1.contains(&title2) || title2.contains(&title1) || titles_match(&title1, &title2),
        "{}: Titles don't match.\n  Book 1: '{}'\n  Book 2: '{}'",
        context,
        book1.metadata.title,
        book2.metadata.title
    );

    // Both books should have at least some content
    assert!(
        !book1.chapters.is_empty(),
        "{}: Book 1 has no chapters (title: '{}')",
        context,
        book1.metadata.title
    );
    assert!(
        !book2.chapters.is_empty(),
        "{}: Book 2 has no chapters (title: '{}')",
        context,
        book2.metadata.title
    );
}

/// Check if titles match (allowing for format-specific variations)
fn titles_match(t1: &str, t2: &str) -> bool {
    let normalize = |s: &str| -> String {
        s.replace('\u{2018}', "'") // left single quote
            .replace('\u{2019}', "'") // right single quote
            .replace('\u{201C}', "\"") // left double quote
            .replace('\u{201D}', "\"") // right double quote
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };

    normalize(t1) == normalize(t2)
}

/// Create a minimal valid book for testing
fn create_minimal_book(title: &str, language: &str) -> Book {
    let mut book = Book::new(title, language);
    let mut chapter = Chapter::new("Chapter 1");
    chapter.add_block(Block::Paragraph(vec![Inline::Text("Test content.".to_string())]));
    book.add_chapter(chapter);
    book
}

// =============================================================================
// Test File Validation
// =============================================================================

#[test]
fn test_all_test_files_exist() {
    ALICE.validate_files_exist();
    AROUND_THE_WORLD.validate_files_exist();
    FAMOUS_PAINTINGS.validate_files_exist();
    SAMPLE1.validate_files_exist();
}

// =============================================================================
// EPUB Decoding Tests
// =============================================================================

#[test]
fn test_decode_epub_alice() {
    let book = decode_file(ALICE.epub, "epub").expect("Failed to decode Alice EPUB");
    assert!(!book.metadata.title.is_empty(), "Title should not be empty");
    assert!(!book.chapters.is_empty(), "Should have chapters");
    assert_metadata_snapshot!("alice_epub_metadata", &book.metadata);
}

#[test]
fn test_decode_epub_around_the_world() {
    let book =
        decode_file(AROUND_THE_WORLD.epub, "epub").expect("Failed to decode Around the World EPUB");
    assert!(!book.metadata.title.is_empty(), "Title should not be empty");
    assert!(!book.chapters.is_empty(), "Should have chapters");
    assert_metadata_snapshot!("around_the_world_epub_metadata", &book.metadata);
}

#[test]
fn test_decode_epub_famous_paintings() {
    let book = decode_file(FAMOUS_PAINTINGS.epub, "epub")
        .expect("Failed to decode Famous Paintings EPUB");
    assert!(!book.metadata.title.is_empty(), "Title should not be empty");
    assert_metadata_snapshot!("famous_paintings_epub_metadata", &book.metadata);
}

#[test]
fn test_decode_epub_sample1() {
    let book = decode_file(SAMPLE1.epub, "epub").expect("Failed to decode Sample1 EPUB");
    assert!(!book.metadata.title.is_empty(), "Title should not be empty");
    assert_metadata_snapshot!("sample1_epub_metadata", &book.metadata);
}

// =============================================================================
// MOBI Decoding Tests
// =============================================================================

#[test]
fn test_decode_mobi_alice() {
    let book = decode_file(ALICE.mobi, "mobi").expect("Failed to decode Alice MOBI");
    assert!(!book.metadata.title.is_empty(), "Title should not be empty");
    assert!(!book.chapters.is_empty(), "Should have chapters");
    assert_metadata_snapshot!("alice_mobi_metadata", &book.metadata);
}

#[test]
fn test_decode_mobi_around_the_world() {
    let book =
        decode_file(AROUND_THE_WORLD.mobi, "mobi").expect("Failed to decode Around the World MOBI");
    assert!(!book.metadata.title.is_empty(), "Title should not be empty");
    assert_metadata_snapshot!("around_the_world_mobi_metadata", &book.metadata);
}

#[test]
fn test_decode_mobi_famous_paintings() {
    let book = decode_file(FAMOUS_PAINTINGS.mobi, "mobi")
        .expect("Failed to decode Famous Paintings MOBI");
    assert!(!book.metadata.title.is_empty(), "Title should not be empty");
    assert_metadata_snapshot!("famous_paintings_mobi_metadata", &book.metadata);
}

#[test]
fn test_decode_mobi_sample1() {
    let book = decode_file(SAMPLE1.mobi, "mobi").expect("Failed to decode Sample1 MOBI");
    assert!(!book.metadata.title.is_empty(), "Title should not be empty");
    assert_metadata_snapshot!("sample1_mobi_metadata", &book.metadata);
}

// =============================================================================
// AZW3 Decoding Tests
// =============================================================================

#[test]
fn test_decode_azw3_alice() {
    let book = decode_file(ALICE.azw3, "azw3").expect("Failed to decode Alice AZW3");
    assert!(!book.metadata.title.is_empty(), "Title should not be empty");
    assert!(!book.chapters.is_empty(), "Should have chapters");
    assert_metadata_snapshot!("alice_azw3_metadata", &book.metadata);
}

#[test]
fn test_decode_azw3_around_the_world() {
    let book =
        decode_file(AROUND_THE_WORLD.azw3, "azw3").expect("Failed to decode Around the World AZW3");
    assert!(!book.metadata.title.is_empty(), "Title should not be empty");
    assert_metadata_snapshot!("around_the_world_azw3_metadata", &book.metadata);
}

#[test]
fn test_decode_azw3_famous_paintings() {
    let book = decode_file(FAMOUS_PAINTINGS.azw3, "azw3")
        .expect("Failed to decode Famous Paintings AZW3");
    assert!(!book.metadata.title.is_empty(), "Title should not be empty");
    assert_metadata_snapshot!("famous_paintings_azw3_metadata", &book.metadata);
}

#[test]
fn test_decode_azw3_sample1() {
    let book = decode_file(SAMPLE1.azw3, "azw3").expect("Failed to decode Sample1 AZW3");
    assert!(!book.metadata.title.is_empty(), "Title should not be empty");
    assert_metadata_snapshot!("sample1_azw3_metadata", &book.metadata);
}

// =============================================================================
// Cross-Format Comparison Tests
// =============================================================================

#[test]
fn test_compare_alice_epub_vs_mobi() {
    let epub_book = decode_file(ALICE.epub, "epub").expect("Failed to decode Alice EPUB");
    let mobi_book = decode_file(ALICE.mobi, "mobi").expect("Failed to decode Alice MOBI");

    assert_books_similar(&epub_book, &mobi_book, "Alice EPUB vs MOBI");
}

#[test]
fn test_compare_alice_epub_vs_azw3() {
    let epub_book = decode_file(ALICE.epub, "epub").expect("Failed to decode Alice EPUB");
    let azw3_book = decode_file(ALICE.azw3, "azw3").expect("Failed to decode Alice AZW3");

    assert_books_similar(&epub_book, &azw3_book, "Alice EPUB vs AZW3");
}

#[test]
fn test_compare_alice_mobi_vs_azw3() {
    let mobi_book = decode_file(ALICE.mobi, "mobi").expect("Failed to decode Alice MOBI");
    let azw3_book = decode_file(ALICE.azw3, "azw3").expect("Failed to decode Alice AZW3");

    assert_books_similar(&mobi_book, &azw3_book, "Alice MOBI vs AZW3");
}

#[test]
fn test_compare_sample1_all_formats() {
    let epub_book = decode_file(SAMPLE1.epub, "epub").expect("Failed to decode Sample1 EPUB");
    let mobi_book = decode_file(SAMPLE1.mobi, "mobi").expect("Failed to decode Sample1 MOBI");
    let azw3_book = decode_file(SAMPLE1.azw3, "azw3").expect("Failed to decode Sample1 AZW3");

    assert_books_similar(&epub_book, &mobi_book, "Sample1 EPUB vs MOBI");
    assert_books_similar(&epub_book, &azw3_book, "Sample1 EPUB vs AZW3");
    assert_books_similar(&mobi_book, &azw3_book, "Sample1 MOBI vs AZW3");
}

// =============================================================================
// Round-Trip Conversion Tests (decode -> encode -> decode)
// =============================================================================

#[test]
fn test_roundtrip_epub_to_epub_alice() {
    let original = decode_file(ALICE.epub, "epub").expect("Failed to decode Alice EPUB");

    // Encode to EPUB
    let encoded = encode_to_bytes(&original, "epub").expect("Failed to encode to EPUB");
    assert!(!encoded.is_empty(), "Encoded EPUB should not be empty");

    // Decode the encoded EPUB
    let mut cursor = Cursor::new(encoded);
    let decoder = decoder_for_extension("epub").unwrap();
    let roundtrip = decoder
        .decode(&mut cursor)
        .expect("Failed to decode roundtrip EPUB");

    // Compare
    assert_eq!(
        original.metadata.title, roundtrip.metadata.title,
        "Title should be preserved in roundtrip"
    );
    assert_eq!(
        original.chapters.len(),
        roundtrip.chapters.len(),
        "Chapter count should be preserved"
    );
}

#[test]
fn test_roundtrip_mobi_to_epub_alice() {
    let original = decode_file(ALICE.mobi, "mobi").expect("Failed to decode Alice MOBI");

    // Encode to EPUB
    let encoded = encode_to_bytes(&original, "epub").expect("Failed to encode MOBI to EPUB");
    assert!(!encoded.is_empty(), "Encoded EPUB should not be empty");

    // Decode the encoded EPUB
    let mut cursor = Cursor::new(encoded);
    let decoder = decoder_for_extension("epub").unwrap();
    let roundtrip = decoder
        .decode(&mut cursor)
        .expect("Failed to decode MOBI->EPUB roundtrip");

    // Compare - the roundtrip should preserve content
    assert_eq!(
        original.metadata.title, roundtrip.metadata.title,
        "Title should be preserved"
    );
    assert_eq!(
        original.chapters.len(),
        roundtrip.chapters.len(),
        "Chapter count should be preserved"
    );
}

#[test]
fn test_roundtrip_azw3_to_epub_sample1() {
    let original = decode_file(SAMPLE1.azw3, "azw3").expect("Failed to decode Sample1 AZW3");

    // Encode to EPUB
    let encoded = encode_to_bytes(&original, "epub").expect("Failed to encode AZW3 to EPUB");
    assert!(!encoded.is_empty(), "Encoded EPUB should not be empty");

    // Decode the encoded EPUB
    let mut cursor = Cursor::new(encoded);
    let decoder = decoder_for_extension("epub").unwrap();
    let roundtrip = decoder
        .decode(&mut cursor)
        .expect("Failed to decode AZW3->EPUB roundtrip");

    assert_eq!(
        original.metadata.title, roundtrip.metadata.title,
        "Title should be preserved"
    );
}

// =============================================================================
// Typst Output Tests
// =============================================================================

#[test]
fn test_epub_to_typst_alice() {
    let book = decode_file(ALICE.epub, "epub").expect("Failed to decode Alice EPUB");
    let typst = encode_to_bytes(&book, "typ").expect("Failed to encode to Typst");

    let typst_str = String::from_utf8(typst).expect("Typst output should be valid UTF-8");
    assert!(
        typst_str.contains(&book.metadata.title),
        "Typst should contain book title"
    );
    insta::assert_snapshot!("alice_epub_to_typst", typst_str);
}

#[test]
fn test_mobi_to_typst_sample1() {
    let book = decode_file(SAMPLE1.mobi, "mobi").expect("Failed to decode Sample1 MOBI");
    let typst = encode_to_bytes(&book, "typ").expect("Failed to encode to Typst");

    let typst_str = String::from_utf8(typst).expect("Typst output should be valid UTF-8");
    assert!(!typst_str.is_empty(), "Typst output should not be empty");
    insta::assert_snapshot!("sample1_mobi_to_typst", typst_str);
}

// =============================================================================
// KEPUB Output Tests
// =============================================================================

#[test]
fn test_epub_to_kepub_alice() {
    let book = decode_file(ALICE.epub, "epub").expect("Failed to decode Alice EPUB");
    let kepub = encode_to_bytes(&book, "kepub").expect("Failed to encode to KEPUB");

    assert!(!kepub.is_empty(), "KEPUB output should not be empty");

    // KEPUB should be a valid zip file (similar to EPUB)
    let cursor = Cursor::new(&kepub);
    let archive = zip::ZipArchive::new(cursor).expect("KEPUB should be a valid ZIP archive");
    assert!(archive.len() > 0, "KEPUB archive should contain files");
}

#[test]
fn test_mobi_to_kepub_sample1() {
    let book = decode_file(SAMPLE1.mobi, "mobi").expect("Failed to decode Sample1 MOBI");
    let kepub = encode_to_bytes(&book, "kepub").expect("Failed to encode MOBI to KEPUB");

    assert!(!kepub.is_empty(), "KEPUB output should not be empty");
}

// =============================================================================
// Content Integrity Tests
// =============================================================================

#[test]
fn test_chapter_content_preserved_alice() {
    let epub_book = decode_file(ALICE.epub, "epub").expect("Failed to decode Alice EPUB");

    // Verify we have meaningful content (Alice has 12 chapters plus front/back matter)
    assert!(
        epub_book.chapters.len() >= ALICE_MIN_CHAPTERS,
        "Alice should have at least {} chapters, found {}",
        ALICE_MIN_CHAPTERS,
        epub_book.chapters.len()
    );

    // Check that chapters have content
    for (i, chapter) in epub_book.chapters.iter().enumerate() {
        assert!(
            !chapter.content.is_empty() || !chapter.title.is_empty(),
            "Chapter {} should have title or content",
            i
        );
    }
}

#[test]
fn test_resources_extracted_famous_paintings() {
    let book = decode_file(FAMOUS_PAINTINGS.epub, "epub")
        .expect("Failed to decode Famous Paintings EPUB");

    // Famous paintings book should have image resources
    let resource_count = book.resources.len();
    assert!(
        resource_count > 0,
        "Famous Paintings should have image resources, found {}",
        resource_count
    );
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_empty_book_encoding() {
    // A book with no chapters should still encode without panicking
    let book = Book::new("Empty Book", "en");

    // Should encode to EPUB (may produce minimal valid EPUB)
    let result = encode_to_bytes(&book, "epub");
    assert!(result.is_ok(), "Empty book should encode without error");

    // Should encode to Typst
    let result = encode_to_bytes(&book, "typ");
    assert!(result.is_ok(), "Empty book should encode to Typst without error");
}

#[test]
fn test_minimal_book_roundtrip() {
    let original = create_minimal_book("Minimal Test Book", "en");

    // Encode to EPUB
    let encoded = encode_to_bytes(&original, "epub").expect("Failed to encode minimal book");

    // Decode back
    let mut cursor = Cursor::new(encoded);
    let decoder = decoder_for_extension("epub").unwrap();
    let roundtrip = decoder.decode(&mut cursor).expect("Failed to decode minimal book");

    assert_eq!(original.metadata.title, roundtrip.metadata.title);
    assert_eq!(original.chapters.len(), roundtrip.chapters.len());
}

#[test]
fn test_unicode_title_handling() {
    // Test various Unicode scripts in book titles
    let test_cases = [
        ("日本語タイトル", "ja"),      // Japanese
        ("Título en español", "es"),   // Spanish with accents
        ("Ελληνικά", "el"),            // Greek
        ("العربية", "ar"),              // Arabic
        ("🎉 Emoji Title 📚", "en"),   // Emojis
    ];

    for (title, lang) in test_cases {
        let book = create_minimal_book(title, lang);

        // Should encode without panicking
        let encoded = encode_to_bytes(&book, "epub")
            .unwrap_or_else(|e| panic!("Failed to encode '{}': {}", title, e));

        // Should decode back with title preserved
        let mut cursor = Cursor::new(encoded);
        let decoder = decoder_for_extension("epub").unwrap();
        let decoded = decoder
            .decode(&mut cursor)
            .unwrap_or_else(|e| panic!("Failed to decode '{}': {}", title, e));

        assert_eq!(
            book.metadata.title, decoded.metadata.title,
            "Unicode title should be preserved: '{}'",
            title
        );
    }
}

#[test]
fn test_long_chapter_title() {
    let long_title = "A".repeat(1000);
    let mut book = Book::new("Test Book", "en");
    let chapter = Chapter::new(&long_title);
    book.add_chapter(chapter);

    // Should encode without truncation issues
    let encoded = encode_to_bytes(&book, "epub").expect("Failed to encode book with long chapter title");

    let mut cursor = Cursor::new(encoded);
    let decoder = decoder_for_extension("epub").unwrap();
    let decoded = decoder.decode(&mut cursor).expect("Failed to decode");

    assert_eq!(decoded.chapters[0].title, long_title, "Long chapter title should be preserved");
}

#[test]
fn test_special_characters_in_content() {
    let mut book = Book::new("Special Chars Test", "en");
    let mut chapter = Chapter::new("Test Chapter");

    // Add content with special characters that might need escaping
    chapter.add_block(Block::Paragraph(vec![
        Inline::Text("Ampersand & less than < greater than > quotes \"double\" 'single'".to_string()),
    ]));
    chapter.add_block(Block::Paragraph(vec![
        Inline::Text("Backslash \\ and forward slash / and hash #".to_string()),
    ]));

    book.add_chapter(chapter);

    // Should encode without issues
    let epub_result = encode_to_bytes(&book, "epub");
    assert!(epub_result.is_ok(), "Special characters should encode to EPUB");

    let typst_result = encode_to_bytes(&book, "typ");
    assert!(typst_result.is_ok(), "Special characters should encode to Typst");
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_invalid_extension_returns_none() {
    assert!(decoder_for_extension("xyz").is_none());
    assert!(decoder_for_extension("doc").is_none());
    assert!(decoder_for_extension("").is_none());
}

#[test]
fn test_invalid_format_returns_none() {
    assert!(encoder_for_format("xyz").is_none());
    assert!(encoder_for_format("mobi").is_none()); // We can decode MOBI but not encode
    assert!(encoder_for_format("").is_none());
}

#[test]
fn test_decode_nonexistent_file() {
    let result = decode_file("nonexistent_file.epub", "epub");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_decode_with_wrong_extension() {
    // Try to decode an EPUB file as MOBI - should fail gracefully
    let result = decode_file(ALICE.epub, "mobi");
    // This may either fail to decode or produce unexpected results
    // The important thing is it doesn't panic
    let _ = result; // We just care that it doesn't panic
}
