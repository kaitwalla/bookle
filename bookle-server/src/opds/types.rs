//! OPDS data structures
//!
//! Types for OPDS 1.2 (Atom/XML) and OPDS 2.0 (JSON) catalog feeds.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// OPDS Link Relations
pub mod rel {
    /// Self link
    pub const SELF: &str = "self";
    /// Start/root of catalog
    pub const START: &str = "start";
    /// Navigation feed
    pub const SUBSECTION: &str = "subsection";
    /// Search link
    pub const SEARCH: &str = "search";
    /// Next page
    pub const NEXT: &str = "next";
    /// Previous page
    pub const PREVIOUS: &str = "previous";
    /// Acquisition link (download)
    pub const ACQUISITION: &str = "http://opds-spec.org/acquisition";
    /// Open access acquisition
    pub const ACQUISITION_OPEN_ACCESS: &str = "http://opds-spec.org/acquisition/open-access";
    /// Cover image
    pub const IMAGE: &str = "http://opds-spec.org/image";
    /// Thumbnail image
    pub const THUMBNAIL: &str = "http://opds-spec.org/image/thumbnail";
}

// MIME Types
pub mod mime {
    /// OPDS 1.2 Navigation feed
    pub const OPDS_CATALOG: &str = "application/atom+xml;profile=opds-catalog";
    /// OPDS 1.2 Navigation feed
    pub const OPDS_CATALOG_KIND_NAVIGATION: &str =
        "application/atom+xml;profile=opds-catalog;kind=navigation";
    /// OPDS 1.2 Acquisition feed
    pub const OPDS_CATALOG_KIND_ACQUISITION: &str =
        "application/atom+xml;profile=opds-catalog;kind=acquisition";
    /// OPDS 2.0 JSON
    pub const OPDS_JSON: &str = "application/opds+json";
    /// OpenSearch descriptor
    pub const OPENSEARCH: &str = "application/opensearchdescription+xml";
    /// EPUB
    pub const EPUB: &str = "application/epub+zip";
    /// JPEG
    pub const JPEG: &str = "image/jpeg";
    /// PNG
    pub const PNG: &str = "image/png";
    /// Generic image (for links where actual type is determined at runtime)
    pub const IMAGE: &str = "image/*";
}

/// OPDS 1.2 Atom Feed
#[derive(Debug, Clone)]
pub struct OpdsFeed {
    /// Unique feed identifier (URN)
    pub id: String,
    /// Feed title
    pub title: String,
    /// Last update timestamp
    pub updated: DateTime<Utc>,
    /// Feed author/owner
    pub author: Option<FeedAuthor>,
    /// Links (self, start, search, pagination)
    pub links: Vec<OpdsLink>,
    /// Feed entries (books or navigation items)
    pub entries: Vec<OpdsEntry>,
    /// Icon URL
    pub icon: Option<String>,
}

impl OpdsFeed {
    /// Create a new OPDS feed
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            updated: Utc::now(),
            author: None,
            links: Vec::new(),
            entries: Vec::new(),
            icon: None,
        }
    }

    /// Add a link to the feed
    pub fn add_link(&mut self, link: OpdsLink) {
        self.links.push(link);
    }

    /// Add an entry to the feed
    pub fn add_entry(&mut self, entry: OpdsEntry) {
        self.entries.push(entry);
    }

    /// Set the feed author
    pub fn with_author(mut self, name: impl Into<String>) -> Self {
        self.author = Some(FeedAuthor {
            name: name.into(),
            uri: None,
        });
        self
    }
}

/// Feed author information
#[derive(Debug, Clone)]
pub struct FeedAuthor {
    pub name: String,
    pub uri: Option<String>,
}

/// OPDS 1.2 Entry (book or navigation item)
#[derive(Debug, Clone)]
pub struct OpdsEntry {
    /// Unique entry identifier (URN)
    pub id: String,
    /// Entry title
    pub title: String,
    /// Last update timestamp
    pub updated: DateTime<Utc>,
    /// Entry authors
    pub authors: Vec<EntryAuthor>,
    /// Entry summary/description
    pub summary: Option<String>,
    /// Content (for navigation entries)
    pub content: Option<String>,
    /// Links (acquisition, images, etc.)
    pub links: Vec<OpdsLink>,
    /// Categories/subjects
    pub categories: Vec<String>,
    /// Language code
    pub language: Option<String>,
    /// Publication date
    pub published: Option<DateTime<Utc>>,
    /// Publisher
    pub publisher: Option<String>,
}

impl OpdsEntry {
    /// Create a new entry
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            updated: Utc::now(),
            authors: Vec::new(),
            summary: None,
            content: None,
            links: Vec::new(),
            categories: Vec::new(),
            language: None,
            published: None,
            publisher: None,
        }
    }

    /// Add an author
    pub fn add_author(&mut self, name: impl Into<String>) {
        self.authors.push(EntryAuthor { name: name.into() });
    }

    /// Add a link
    pub fn add_link(&mut self, link: OpdsLink) {
        self.links.push(link);
    }
}

/// Entry author
#[derive(Debug, Clone)]
pub struct EntryAuthor {
    pub name: String,
}

/// OPDS Link
#[derive(Debug, Clone)]
pub struct OpdsLink {
    /// Link relation
    pub rel: String,
    /// Target URL
    pub href: String,
    /// MIME type
    pub media_type: String,
    /// Title (optional)
    pub title: Option<String>,
}

impl OpdsLink {
    /// Create a new link
    pub fn new(
        rel: impl Into<String>,
        href: impl Into<String>,
        media_type: impl Into<String>,
    ) -> Self {
        Self {
            rel: rel.into(),
            href: href.into(),
            media_type: media_type.into(),
            title: None,
        }
    }

    /// Set the link title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

// ============================================================================
// OPDS 2.0 Types (JSON)
// ============================================================================

/// OPDS 2.0 Feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opds2Feed {
    /// Feed metadata
    pub metadata: Opds2FeedMetadata,
    /// Navigation/feed links
    pub links: Vec<Opds2Link>,
    /// Publications (books)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub publications: Vec<Opds2Publication>,
    /// Navigation entries (for navigation feeds)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub navigation: Vec<Opds2Navigation>,
    /// Groups (for grouped feeds)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<Opds2Group>,
}

/// OPDS 2.0 Feed Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opds2FeedMetadata {
    /// Feed title
    pub title: String,
    /// Feed subtitle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// Last modified timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<DateTime<Utc>>,
    /// Number of items (for paginated feeds)
    #[serde(rename = "numberOfItems", skip_serializing_if = "Option::is_none")]
    pub number_of_items: Option<u32>,
    /// Items per page
    #[serde(rename = "itemsPerPage", skip_serializing_if = "Option::is_none")]
    pub items_per_page: Option<u32>,
    /// Current page
    #[serde(rename = "currentPage", skip_serializing_if = "Option::is_none")]
    pub current_page: Option<u32>,
}

/// OPDS 2.0 Link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opds2Link {
    /// Link relation
    pub rel: String,
    /// Target URL
    pub href: String,
    /// MIME type
    #[serde(rename = "type")]
    pub media_type: String,
    /// Title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl Opds2Link {
    /// Create a new link
    pub fn new(
        rel: impl Into<String>,
        href: impl Into<String>,
        media_type: impl Into<String>,
    ) -> Self {
        Self {
            rel: rel.into(),
            href: href.into(),
            media_type: media_type.into(),
            title: None,
        }
    }

    /// Set the link title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// OPDS 2.0 Navigation Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opds2Navigation {
    /// Entry title
    pub title: String,
    /// Entry href
    pub href: String,
    /// MIME type
    #[serde(rename = "type")]
    pub media_type: String,
    /// Link relation
    pub rel: String,
}

/// OPDS 2.0 Group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opds2Group {
    /// Group metadata
    pub metadata: Opds2GroupMetadata,
    /// Group links
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Opds2Link>,
    /// Publications in this group
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub publications: Vec<Opds2Publication>,
}

/// OPDS 2.0 Group Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opds2GroupMetadata {
    /// Group title
    pub title: String,
}

/// OPDS 2.0 Publication (book)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opds2Publication {
    /// Publication metadata
    pub metadata: Opds2PublicationMetadata,
    /// Acquisition links
    pub links: Vec<Opds2Link>,
    /// Cover images
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<Opds2Image>,
}

/// OPDS 2.0 Publication Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opds2PublicationMetadata {
    /// Book title
    pub title: String,
    /// Authors
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub author: Vec<Opds2Contributor>,
    /// Language code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Publisher
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    /// Publication date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<DateTime<Utc>>,
    /// Last modified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<DateTime<Utc>>,
    /// Subjects/categories
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subject: Vec<Opds2Subject>,
    /// Unique identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
}

/// OPDS 2.0 Contributor (author, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opds2Contributor {
    /// Contributor name
    pub name: String,
}

/// OPDS 2.0 Subject
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opds2Subject {
    /// Subject name
    pub name: String,
}

/// OPDS 2.0 Image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opds2Image {
    /// Image URL
    pub href: String,
    /// MIME type
    #[serde(rename = "type")]
    pub media_type: String,
    /// Width in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Height in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}
