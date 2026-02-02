//! OpenSearch descriptor for OPDS search discovery

/// Generate OpenSearch descriptor XML
///
/// This allows e-readers to discover search capabilities.
pub fn render_opensearch(base_url: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/">
  <ShortName>Bookle</ShortName>
  <Description>Search the Bookle library</Description>
  <InputEncoding>UTF-8</InputEncoding>
  <OutputEncoding>UTF-8</OutputEncoding>
  <Url type="application/atom+xml;profile=opds-catalog" template="{base_url}/opds/search?q={{searchTerms}}"/>
  <Url type="application/opds+json" template="{base_url}/opds/v2/search?q={{searchTerms}}"/>
</OpenSearchDescription>"#,
        base_url = base_url
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opensearch_descriptor() {
        let xml = render_opensearch("http://localhost:3000");
        assert!(xml.contains("<ShortName>Bookle</ShortName>"));
        assert!(xml.contains("http://localhost:3000/opds/search?q={searchTerms}"));
        assert!(xml.contains("application/atom+xml;profile=opds-catalog"));
        assert!(xml.contains("application/opds+json"));
    }
}
