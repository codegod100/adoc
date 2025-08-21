pub mod ast;
pub mod parser;

pub use ast::*;
pub use parser::AsciiDocParser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_document() {
        let input = "= Test Document\n\nHello world!";
        let result = AsciiDocParser::parse_document(input);
        match &result {
            Ok(_) => {},
            Err(e) => println!("Parse error: {}", e),
        }
        assert!(result.is_ok());
        
        let doc = result.unwrap();
        assert!(doc.header.is_some());
        assert_eq!(doc.header.unwrap().title, "Test Document");
        assert_eq!(doc.body.len(), 1);
    }

    #[test]
    fn test_header_with_attributes() {
        let input = r#"= My Document
:author: John Doe
:version: 1.0

Content here."#;
        
        let result = AsciiDocParser::parse_document(input);
        assert!(result.is_ok());
        
        let doc = result.unwrap();
        let header = doc.header.unwrap();
        assert_eq!(header.title, "My Document");
        assert_eq!(header.attributes.len(), 2);
        assert_eq!(header.attributes[0].name, "author");
        assert_eq!(header.attributes[0].value, Some("John Doe".to_string()));
    }

    #[test]
    fn test_sections() {
        let input = r#"= Document

== Section 1

=== Subsection

Content"#;
        
        let result = AsciiDocParser::parse_document(input);
        match &result {
            Ok(_) => {},
            Err(e) => println!("Parse error: {}", e),
        }
        assert!(result.is_ok());
        
        let doc = result.unwrap();
        println!("Body has {} blocks: {:#?}", doc.body.len(), doc.body);
        assert_eq!(doc.body.len(), 1);
        
        if let Block::Section { level, title, .. } = &doc.body[0] {
            assert_eq!(*level, 2);
            assert_eq!(title, "Section 1");
        } else {
            panic!("Expected section block");
        }
    }

    #[test]
    fn test_delimited_block() {
        let input = r#"= Document

----
Code block content
line 2
----"#;
        
        let result = AsciiDocParser::parse_document(input);
        assert!(result.is_ok());
        
        let doc = result.unwrap();
        assert_eq!(doc.body.len(), 1);
        
        if let Block::DelimitedBlock { kind, content } = &doc.body[0] {
            assert!(matches!(kind, DelimitedBlockKind::Listing));
            assert!(content.contains("Code block content"));
        } else {
            panic!("Expected delimited block");
        }
    }

    #[test]
    fn test_unordered_list() {
        let input = r#"= Document

* Item 1
* Item 2
** Nested item"#;
        
        let result = AsciiDocParser::parse_document(input);
        assert!(result.is_ok());
        
        let doc = result.unwrap();
        assert_eq!(doc.body.len(), 1);
        
        if let Block::List { kind, items } = &doc.body[0] {
            assert!(matches!(kind, ListKind::Unordered));
            assert_eq!(items.len(), 3);
        } else {
            panic!("Expected list block");
        }
    }

    #[test]
    fn test_formatted_text() {
        let input = r#"= Document

This is *bold* and _italic_ text."#;
        
        let result = AsciiDocParser::parse_document(input);
        assert!(result.is_ok());
        
        let doc = result.unwrap();
        assert_eq!(doc.body.len(), 1);
        
        if let Block::Paragraph { content } = &doc.body[0] {
            assert!(!content.is_empty());
        } else {
            panic!("Expected paragraph block");
        }
    }

    #[test]
    fn test_link_macro() {
        let input = r#"= Document

Visit link:https://example.com[Example Site] for more info."#;
        
        let result = AsciiDocParser::parse_document(input);
        assert!(result.is_ok());
        
        let doc = result.unwrap();
        assert_eq!(doc.body.len(), 1);
    }

    #[test]
    fn test_description_list() {
        let input = r#"= Document

Term 1:: Definition 1
Term 2:: Definition 2"#;
        
        let result = AsciiDocParser::parse_document(input);
        assert!(result.is_ok());
        
        let doc = result.unwrap();
        assert_eq!(doc.body.len(), 1);
        
        if let Block::List { kind, items } = &doc.body[0] {
            assert!(matches!(kind, ListKind::Description));
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected description list");
        }
    }
}