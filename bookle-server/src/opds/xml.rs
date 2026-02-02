//! XML generation for OPDS 1.2 Atom feeds
//!
//! Uses quick-xml to generate well-formed Atom XML with OPDS extensions.

use super::{OpdsEntry, OpdsFeed, OpdsLink};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Cursor;

// XML Namespaces
const NS_ATOM: &str = "http://www.w3.org/2005/Atom";
const NS_DC: &str = "http://purl.org/dc/terms/";
const NS_OPDS: &str = "http://opds-spec.org/2010/catalog";

/// Render an OPDS feed to Atom XML
pub fn render_feed(feed: &OpdsFeed) -> Result<String, quick_xml::Error> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    // XML declaration
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    // Root feed element with namespaces
    let mut feed_elem = BytesStart::new("feed");
    feed_elem.push_attribute(("xmlns", NS_ATOM));
    feed_elem.push_attribute(("xmlns:dc", NS_DC));
    feed_elem.push_attribute(("xmlns:opds", NS_OPDS));
    writer.write_event(Event::Start(feed_elem))?;

    // Feed ID
    write_text_element(&mut writer, "id", &feed.id)?;

    // Feed title
    write_text_element(&mut writer, "title", &feed.title)?;

    // Feed updated
    write_text_element(&mut writer, "updated", &feed.updated.to_rfc3339())?;

    // Feed author
    if let Some(ref author) = feed.author {
        writer.write_event(Event::Start(BytesStart::new("author")))?;
        write_text_element(&mut writer, "name", &author.name)?;
        if let Some(ref uri) = author.uri {
            write_text_element(&mut writer, "uri", uri)?;
        }
        writer.write_event(Event::End(BytesEnd::new("author")))?;
    }

    // Feed icon
    if let Some(ref icon) = feed.icon {
        write_text_element(&mut writer, "icon", icon)?;
    }

    // Feed links
    for link in &feed.links {
        write_link(&mut writer, link)?;
    }

    // Feed entries
    for entry in &feed.entries {
        write_entry(&mut writer, entry)?;
    }

    // Close feed
    writer.write_event(Event::End(BytesEnd::new("feed")))?;

    let result = writer.into_inner().into_inner();
    Ok(String::from_utf8(result).unwrap_or_default())
}

/// Write a simple text element
fn write_text_element<W: std::io::Write>(
    writer: &mut Writer<W>,
    name: &str,
    content: &str,
) -> Result<(), quick_xml::Error> {
    writer.write_event(Event::Start(BytesStart::new(name)))?;
    writer.write_event(Event::Text(BytesText::new(content)))?;
    writer.write_event(Event::End(BytesEnd::new(name)))?;
    Ok(())
}

/// Write an OPDS link element
fn write_link<W: std::io::Write>(
    writer: &mut Writer<W>,
    link: &OpdsLink,
) -> Result<(), quick_xml::Error> {
    let mut elem = BytesStart::new("link");
    elem.push_attribute(("rel", link.rel.as_str()));
    elem.push_attribute(("href", link.href.as_str()));
    elem.push_attribute(("type", link.media_type.as_str()));
    if let Some(ref title) = link.title {
        elem.push_attribute(("title", title.as_str()));
    }
    writer.write_event(Event::Empty(elem))?;
    Ok(())
}

/// Write an OPDS entry element
fn write_entry<W: std::io::Write>(
    writer: &mut Writer<W>,
    entry: &OpdsEntry,
) -> Result<(), quick_xml::Error> {
    writer.write_event(Event::Start(BytesStart::new("entry")))?;

    // Entry ID
    write_text_element(writer, "id", &entry.id)?;

    // Entry title
    write_text_element(writer, "title", &entry.title)?;

    // Entry updated
    write_text_element(writer, "updated", &entry.updated.to_rfc3339())?;

    // Entry published
    if let Some(ref published) = entry.published {
        write_text_element(writer, "published", &published.to_rfc3339())?;
    }

    // Entry authors
    for author in &entry.authors {
        writer.write_event(Event::Start(BytesStart::new("author")))?;
        write_text_element(writer, "name", &author.name)?;
        writer.write_event(Event::End(BytesEnd::new("author")))?;
    }

    // Entry summary
    if let Some(ref summary) = entry.summary {
        let mut elem = BytesStart::new("summary");
        elem.push_attribute(("type", "text"));
        writer.write_event(Event::Start(elem))?;
        writer.write_event(Event::Text(BytesText::new(summary)))?;
        writer.write_event(Event::End(BytesEnd::new("summary")))?;
    }

    // Entry content (for navigation entries)
    if let Some(ref content) = entry.content {
        let mut elem = BytesStart::new("content");
        elem.push_attribute(("type", "text"));
        writer.write_event(Event::Start(elem))?;
        writer.write_event(Event::Text(BytesText::new(content)))?;
        writer.write_event(Event::End(BytesEnd::new("content")))?;
    }

    // Dublin Core elements
    if let Some(ref language) = entry.language {
        write_text_element(writer, "dc:language", language)?;
    }
    if let Some(ref publisher) = entry.publisher {
        write_text_element(writer, "dc:publisher", publisher)?;
    }

    // Categories
    for category in &entry.categories {
        let mut elem = BytesStart::new("category");
        elem.push_attribute(("term", category.as_str()));
        elem.push_attribute(("label", category.as_str()));
        writer.write_event(Event::Empty(elem))?;
    }

    // Entry links
    for link in &entry.links {
        write_link(writer, link)?;
    }

    writer.write_event(Event::End(BytesEnd::new("entry")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opds::{mime, rel};

    #[test]
    fn test_render_empty_feed() {
        let feed = OpdsFeed::new("urn:uuid:test", "Test Feed");
        let xml = render_feed(&feed).unwrap();
        assert!(xml.contains("<feed"));
        assert!(xml.contains("xmlns=\"http://www.w3.org/2005/Atom\""));
        assert!(xml.contains("<title>Test Feed</title>"));
    }

    #[test]
    fn test_render_feed_with_entry() {
        let mut feed = OpdsFeed::new("urn:uuid:test", "Test Feed");
        let mut entry = OpdsEntry::new("urn:uuid:book1", "Test Book");
        entry.add_author("Test Author");
        entry.add_link(OpdsLink::new(rel::ACQUISITION, "/download", mime::EPUB));
        feed.add_entry(entry);

        let xml = render_feed(&feed).unwrap();
        assert!(xml.contains("<entry>"));
        assert!(xml.contains("<title>Test Book</title>"));
        assert!(xml.contains("<name>Test Author</name>"));
        assert!(xml.contains("rel=\"http://opds-spec.org/acquisition\""));
    }
}
