use pest::Parser;
use pest_derive::Parser;
use crate::ast::*;

#[derive(Parser)]
#[grammar = "asciidoc.pest"]
pub struct AsciiDocParser;

impl AsciiDocParser {
    pub fn parse_document(input: &str) -> Result<Document, Box<dyn std::error::Error>> {
        let pairs = AsciiDocParser::parse(Rule::document, input)?;
        
        for pair in pairs {
            match pair.as_rule() {
                Rule::document => return Ok(parse_document_pair(pair)),
                _ => unreachable!(),
            }
        }
        
        unreachable!()
    }
}

fn parse_document_pair(pair: pest::iterators::Pair<Rule>) -> Document {
    let mut header = None;
    let mut body = Vec::new();
    
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::header => {
                header = Some(parse_header(inner_pair));
            }
            Rule::body => {
                body = parse_body(inner_pair);
            }
            Rule::EOI => break,
            _ => {}
        }
    }
    
    Document { header, body }
}

fn parse_header(pair: pest::iterators::Pair<Rule>) -> Header {
    let mut title = String::new();
    let mut attributes = Vec::new();
    
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::title => {
                title = parse_title(inner_pair);
            }
            Rule::header_attribute => {
                attributes.push(parse_header_attribute(inner_pair));
            }
            _ => {}
        }
    }
    
    Header { title, attributes }
}

fn parse_title(pair: pest::iterators::Pair<Rule>) -> String {
    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == Rule::title_text {
            return inner_pair.as_str().to_string();
        }
    }
    String::new()
}

fn parse_header_attribute(pair: pest::iterators::Pair<Rule>) -> Attribute {
    let mut name = String::new();
    let mut value = None;
    
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::attribute_name => {
                name = inner_pair.as_str().to_string();
            }
            Rule::attribute_value => {
                let val = inner_pair.as_str().trim();
                if !val.is_empty() {
                    value = Some(val.to_string());
                }
            }
            _ => {}
        }
    }
    
    Attribute { name, value }
}

fn parse_body(pair: pest::iterators::Pair<Rule>) -> Vec<Block> {
    let mut blocks = Vec::new();
    
    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == Rule::block {
            if let Some(block) = parse_block(inner_pair) {
                blocks.push(block);
            }
        }
    }
    
    // Post-process to handle block attributes
    process_block_attributes(&mut blocks);
    
    blocks
}

fn process_block_attributes(blocks: &mut Vec<Block>) {
    let mut i = 0;
    while i < blocks.len() {
        // Check if current block is a paragraph that looks like a block attribute
        if let Block::Paragraph { content } = &blocks[i] {
            if content.len() == 1 {
                if let InlineElement::Text(text) = &content[0] {
                    // Check if it matches block attribute pattern [,language] or [options]
                    if text.starts_with('[') && text.ends_with(']') {
                        let attr_content = &text[1..text.len()-1];
                        let attributes: Vec<String> = attr_content.split(',').map(|s| s.trim().to_string()).collect();
                        
                        // Check if next block is a delimited block
                        if i + 1 < blocks.len() {
                            if let Block::DelimitedBlock { kind, content, language: _ } = &blocks[i + 1] {
                                // Extract language from attributes
                                let new_language = extract_language_from_attributes(&Some(attributes));
                                
                                // Replace the next block with updated language
                                blocks[i + 1] = Block::DelimitedBlock {
                                    kind: kind.clone(),
                                    content: content.clone(),
                                    language: new_language,
                                };
                                
                                // Remove the attribute paragraph
                                blocks.remove(i);
                                continue;
                            }
                        }
                    }
                }
            }
        }
        i += 1;
    }
}

fn parse_block(pair: pest::iterators::Pair<Rule>) -> Option<Block> {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::section => return Some(parse_section(inner_pair)),
            // Rule::attributed_block => return Some(parse_attributed_block(inner_pair)),
            Rule::delimited_block => return Some(parse_delimited_block(inner_pair)),
            Rule::list => return Some(parse_list(inner_pair)),
            Rule::paragraph => return Some(parse_paragraph(inner_pair)),
            Rule::block_metadata => return Some(parse_block_metadata(inner_pair)),
            _ => {}
        }
    }
    None
}

fn parse_attributed_block(pair: pest::iterators::Pair<Rule>) -> Block {
    let mut attributes: Option<Vec<String>> = None;
    
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::block_attribute => {
                attributes = Some(parse_block_attribute(inner_pair));
            },
            Rule::delimited_block => {
                return parse_delimited_block_with_attributes(inner_pair, attributes);
            },
            _ => {}
        }
    }
    
    // Fallback if no delimited block found
    Block::DelimitedBlock {
        kind: DelimitedBlockKind::Literal,
        content: String::new(),
        language: None,
    }
}

fn parse_section(pair: pest::iterators::Pair<Rule>) -> Block {
    let content = pair.as_str();
    let level = content.chars().take_while(|&c| c == '=').count();
    let title = content.trim_start_matches('=').trim().to_string();
    
    Block::Section { level, title, blocks: Vec::new() }
}


fn parse_delimited_block(pair: pest::iterators::Pair<Rule>) -> Block {
    parse_delimited_block_with_attributes(pair, None)
}

fn parse_delimited_block_with_attributes(pair: pest::iterators::Pair<Rule>, attributes: Option<Vec<String>>) -> Block {
    let language = extract_language_from_attributes(&attributes);
    
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::listing_block => {
                return Block::DelimitedBlock {
                    kind: DelimitedBlockKind::Listing,
                    content: extract_delimited_content(inner_pair, Rule::listing_content),
                    language: language.clone(),
                };
            }
            Rule::example_block => {
                return Block::DelimitedBlock {
                    kind: DelimitedBlockKind::Example,
                    content: extract_delimited_content(inner_pair, Rule::example_content),
                    language: language.clone(),
                };
            }
            Rule::literal_block => {
                return Block::DelimitedBlock {
                    kind: DelimitedBlockKind::Literal,
                    content: extract_delimited_content(inner_pair, Rule::literal_content),
                    language: language.clone(),
                };
            }
            Rule::sidebar_block => {
                return Block::DelimitedBlock {
                    kind: DelimitedBlockKind::Sidebar,
                    content: extract_delimited_content(inner_pair, Rule::sidebar_content),
                    language: language.clone(),
                };
            }
            Rule::quote_block => {
                return Block::DelimitedBlock {
                    kind: DelimitedBlockKind::Quote,
                    content: extract_delimited_content(inner_pair, Rule::quote_content),
                    language: language.clone(),
                };
            }
            _ => {}
        }
    }
    
    Block::DelimitedBlock {
        kind: DelimitedBlockKind::Literal,
        content: String::new(),
        language,
    }
}

fn extract_delimited_content(pair: pest::iterators::Pair<Rule>, content_rule: Rule) -> String {
    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == content_rule {
            return inner_pair.as_str().to_string();
        }
    }
    String::new()
}

fn extract_language_from_attributes(attributes: &Option<Vec<String>>) -> Option<String> {
    if let Some(attrs) = attributes {
        for attr in attrs {
            let trimmed = attr.trim();
            // Handle [,language] syntax - second attribute is language
            if trimmed.starts_with(',') {
                return Some(trimmed[1..].to_string());
            }
            // Handle [language] syntax - if it's a known language or starts with a letter
            if !trimmed.is_empty() && !trimmed.contains('=') && !trimmed.contains(':') {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn parse_list(pair: pest::iterators::Pair<Rule>) -> Block {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::unordered_list => {
                return Block::List {
                    kind: ListKind::Unordered,
                    items: parse_unordered_list(inner_pair),
                };
            }
            Rule::ordered_list => {
                return Block::List {
                    kind: ListKind::Ordered,
                    items: parse_ordered_list(inner_pair),
                };
            }
            Rule::description_list => {
                return Block::List {
                    kind: ListKind::Description,
                    items: parse_description_list(inner_pair),
                };
            }
            _ => {}
        }
    }
    
    Block::List {
        kind: ListKind::Unordered,
        items: Vec::new(),
    }
}

fn parse_unordered_list(pair: pest::iterators::Pair<Rule>) -> Vec<ListItem> {
    let mut items = Vec::new();
    
    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == Rule::unordered_item {
            let (level, content) = parse_unordered_item(inner_pair);
            items.push(ListItem::Unordered { level, content });
        }
    }
    
    items
}

fn parse_unordered_item(pair: pest::iterators::Pair<Rule>) -> (usize, Vec<InlineElement>) {
    let text = pair.as_str();
    let level = text.chars().take_while(|&c| c == '*').count();
    let content_start = text.find(' ').unwrap_or(level) + 1;
    let content = if content_start < text.len() {
        parse_paragraph_content(&text[content_start..])
    } else {
        Vec::new()
    };
    
    (level, content)
}

fn parse_ordered_list(pair: pest::iterators::Pair<Rule>) -> Vec<ListItem> {
    let mut items = Vec::new();
    
    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == Rule::ordered_item {
            let (level, content) = parse_ordered_item(inner_pair);
            items.push(ListItem::Ordered { level, content });
        }
    }
    
    items
}

fn parse_ordered_item(pair: pest::iterators::Pair<Rule>) -> (usize, Vec<InlineElement>) {
    let text = pair.as_str();
    let level = text.chars().take_while(|&c| c == '.').count();
    let content_start = text.find(' ').unwrap_or(level) + 1;
    let content = if content_start < text.len() {
        parse_paragraph_content(&text[content_start..])
    } else {
        Vec::new()
    };
    
    (level, content)
}

fn parse_description_list(pair: pest::iterators::Pair<Rule>) -> Vec<ListItem> {
    let mut items = Vec::new();
    
    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == Rule::description_item {
            items.push(parse_description_item(inner_pair));
        }
    }
    
    items
}

fn parse_description_item(pair: pest::iterators::Pair<Rule>) -> ListItem {
    let mut term = String::new();
    let mut description = None;
    
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::description_term => {
                term = inner_pair.as_str().to_string();
            }
            Rule::description_text => {
                let text = inner_pair.as_str().trim();
                if !text.is_empty() {
                    description = Some(vec![InlineElement::Text(text.to_string())]);
                }
            }
            _ => {}
        }
    }
    
    ListItem::Description { term, description }
}

fn parse_paragraph(pair: pest::iterators::Pair<Rule>) -> Block {
    let mut content = Vec::new();
    let mut first_line = true;
    
    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == Rule::paragraph_line {
            for line_inner in inner_pair.into_inner() {
                if line_inner.as_rule() == Rule::paragraph_text {
                    // Add space between lines (except for the first line)
                    if !first_line && !content.is_empty() {
                        content.push(InlineElement::Text(" ".to_string()));
                    }
                    content.extend(parse_paragraph_content(line_inner.as_str()));
                    first_line = false;
                }
            }
        }
    }
    
    Block::Paragraph { content }
}

fn parse_block_metadata(pair: pest::iterators::Pair<Rule>) -> Block {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::block_title => {
                let title = inner_pair.as_str().trim_start_matches('.').to_string();
                return Block::BlockMetadata {
                    kind: BlockMetadataKind::Title(title),
                };
            }
            Rule::block_attribute => {
                let attributes = parse_block_attribute(inner_pair);
                return Block::BlockMetadata {
                    kind: BlockMetadataKind::Attribute(attributes),
                };
            }
            Rule::block_anchor => {
                let anchor = parse_block_anchor(inner_pair);
                return Block::BlockMetadata {
                    kind: BlockMetadataKind::Anchor(anchor),
                };
            }
            _ => {}
        }
    }
    
    Block::BlockMetadata {
        kind: BlockMetadataKind::Title(String::new()),
    }
}

fn parse_block_attribute(pair: pest::iterators::Pair<Rule>) -> Vec<String> {
    let mut attributes = Vec::new();
    
    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == Rule::attribute_list {
            for attr_pair in inner_pair.into_inner() {
                if attr_pair.as_rule() == Rule::attribute_entry {
                    attributes.push(attr_pair.as_str().trim().to_string());
                }
            }
        }
    }
    
    attributes
}

fn parse_block_anchor(pair: pest::iterators::Pair<Rule>) -> String {
    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == Rule::anchor_id {
            return inner_pair.as_str().to_string();
        }
    }
    String::new()
}

fn parse_inline_elements(pair: pest::iterators::Pair<Rule>) -> Vec<InlineElement> {
    let mut elements = Vec::new();
    
    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == Rule::inline_element {
            if let Some(element) = parse_inline_element(inner_pair) {
                elements.push(element);
            }
        }
    }
    
    elements
}

fn parse_inline_element(pair: pest::iterators::Pair<Rule>) -> Option<InlineElement> {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::formatted_text => return Some(parse_formatted_text(inner_pair)),
            Rule::inline_macro => return Some(parse_inline_macro(inner_pair)),
            Rule::line_break => return Some(InlineElement::LineBreak),
            Rule::whitespace => {
                return Some(InlineElement::Text(inner_pair.as_str().to_string()));
            }
            Rule::regular_text => {
                return Some(InlineElement::Text(inner_pair.as_str().to_string()));
            }
            _ => {}
        }
    }
    None
}

fn parse_formatted_text(pair: pest::iterators::Pair<Rule>) -> InlineElement {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::strong_text => {
                return InlineElement::Formatted {
                    kind: FormattedTextKind::Strong,
                    content: parse_formatted_content(inner_pair, Rule::strong_content),
                };
            }
            Rule::emphasis_text => {
                return InlineElement::Formatted {
                    kind: FormattedTextKind::Emphasis,
                    content: parse_formatted_content(inner_pair, Rule::emphasis_content),
                };
            }
            Rule::monospace_text => {
                return InlineElement::Formatted {
                    kind: FormattedTextKind::Monospace,
                    content: parse_formatted_content(inner_pair, Rule::monospace_content),
                };
            }
            Rule::superscript_text => {
                return InlineElement::Formatted {
                    kind: FormattedTextKind::Superscript,
                    content: parse_formatted_content(inner_pair, Rule::superscript_content),
                };
            }
            Rule::subscript_text => {
                return InlineElement::Formatted {
                    kind: FormattedTextKind::Subscript,
                    content: parse_formatted_content(inner_pair, Rule::subscript_content),
                };
            }
            _ => {}
        }
    }
    
    InlineElement::Text(String::new())
}

fn parse_formatted_content(pair: pest::iterators::Pair<Rule>, content_rule: Rule) -> Vec<InlineElement> {
    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == content_rule {
            return vec![InlineElement::Text(inner_pair.as_str().to_string())];
        }
    }
    Vec::new()
}

fn parse_paragraph_content(text: &str) -> Vec<InlineElement> {
    let mut elements = Vec::new();
    let mut current_pos = 0;
    
    while current_pos < text.len() {
        // Find the earliest formatting marker
        let remaining = &text[current_pos..];
        let mut earliest_pos = remaining.len();
        let mut marker_type = None;
        
        // Check for all formatting types
        if let Some(pos) = remaining.find('*') {
            if pos < earliest_pos {
                earliest_pos = pos;
                marker_type = Some("*");
            }
        }
        if let Some(pos) = remaining.find('_') {
            if pos < earliest_pos {
                earliest_pos = pos;
                marker_type = Some("_");
            }
        }
        if let Some(pos) = remaining.find('`') {
            if pos < earliest_pos {
                earliest_pos = pos;
                marker_type = Some("`");
            }
        }
        if let Some(pos) = remaining.find('^') {
            if pos < earliest_pos {
                earliest_pos = pos;
                marker_type = Some("^");
            }
        }
        if let Some(pos) = remaining.find('~') {
            if pos < earliest_pos {
                earliest_pos = pos;
                marker_type = Some("~");
            }
        }
        if let Some(pos) = remaining.find("link:") {
            if pos < earliest_pos {
                earliest_pos = pos;
                marker_type = Some("link:");
            }
        }
        if let Some(pos) = remaining.find("https://") {
            if pos < earliest_pos {
                earliest_pos = pos;
                marker_type = Some("https://");
            }
        }
        if let Some(pos) = remaining.find("http://") {
            if pos < earliest_pos {
                earliest_pos = pos;
                marker_type = Some("http://");
            }
        }
        if let Some(pos) = remaining.find("<<") {
            if pos < earliest_pos {
                earliest_pos = pos;
                marker_type = Some("<<");
            }
        }
        
        if let Some(marker) = marker_type {
            let actual_start = current_pos + earliest_pos;
            
            // Add text before the marker
            if earliest_pos > 0 {
                elements.push(InlineElement::Text(text[current_pos..actual_start].to_string()));
            }
            
            match marker {
                "*" => {
                    if let Some(end) = text[actual_start + 1..].find('*') {
                        let actual_end = actual_start + 1 + end;
                        let content = &text[actual_start + 1..actual_end];
                        elements.push(InlineElement::Formatted {
                            kind: FormattedTextKind::Strong,
                            content: vec![InlineElement::Text(content.to_string())],
                        });
                        current_pos = actual_end + 1;
                    } else {
                        elements.push(InlineElement::Text(text[actual_start..actual_start + 1].to_string()));
                        current_pos = actual_start + 1;
                    }
                }
                "_" => {
                    if let Some(end) = text[actual_start + 1..].find('_') {
                        let actual_end = actual_start + 1 + end;
                        let content = &text[actual_start + 1..actual_end];
                        elements.push(InlineElement::Formatted {
                            kind: FormattedTextKind::Emphasis,
                            content: vec![InlineElement::Text(content.to_string())],
                        });
                        current_pos = actual_end + 1;
                    } else {
                        elements.push(InlineElement::Text(text[actual_start..actual_start + 1].to_string()));
                        current_pos = actual_start + 1;
                    }
                }
                "`" => {
                    if let Some(end) = text[actual_start + 1..].find('`') {
                        let actual_end = actual_start + 1 + end;
                        let content = &text[actual_start + 1..actual_end];
                        elements.push(InlineElement::Formatted {
                            kind: FormattedTextKind::Monospace,
                            content: vec![InlineElement::Text(content.to_string())],
                        });
                        current_pos = actual_end + 1;
                    } else {
                        elements.push(InlineElement::Text(text[actual_start..actual_start + 1].to_string()));
                        current_pos = actual_start + 1;
                    }
                }
                "^" => {
                    if let Some(end) = text[actual_start + 1..].find('^') {
                        let actual_end = actual_start + 1 + end;
                        let content = &text[actual_start + 1..actual_end];
                        elements.push(InlineElement::Formatted {
                            kind: FormattedTextKind::Superscript,
                            content: vec![InlineElement::Text(content.to_string())],
                        });
                        current_pos = actual_end + 1;
                    } else {
                        elements.push(InlineElement::Text(text[actual_start..actual_start + 1].to_string()));
                        current_pos = actual_start + 1;
                    }
                }
                "~" => {
                    if let Some(end) = text[actual_start + 1..].find('~') {
                        let actual_end = actual_start + 1 + end;
                        let content = &text[actual_start + 1..actual_end];
                        elements.push(InlineElement::Formatted {
                            kind: FormattedTextKind::Subscript,
                            content: vec![InlineElement::Text(content.to_string())],
                        });
                        current_pos = actual_end + 1;
                    } else {
                        elements.push(InlineElement::Text(text[actual_start..actual_start + 1].to_string()));
                        current_pos = actual_start + 1;
                    }
                }
                "link:" => {
                    if let Some(bracket_start) = text[actual_start..].find('[') {
                        if let Some(bracket_end) = text[actual_start + bracket_start..].find(']') {
                            let url_start = actual_start + 5; // after "link:"
                            let url_end = actual_start + bracket_start;
                            let text_start = actual_start + bracket_start + 1;
                            let text_end = actual_start + bracket_start + bracket_end;
                            
                            let url = text[url_start..url_end].to_string();
                            let link_text = text[text_start..text_end].to_string();
                            
                            elements.push(InlineElement::Macro {
                                kind: MacroKind::Link {
                                    url,
                                    text: if link_text.is_empty() { None } else { Some(link_text) },
                                },
                            });
                            current_pos = text_end + 1;
                        } else {
                            elements.push(InlineElement::Text(text[actual_start..actual_start + 5].to_string()));
                            current_pos = actual_start + 5;
                        }
                    } else {
                        elements.push(InlineElement::Text(text[actual_start..actual_start + 5].to_string()));
                        current_pos = actual_start + 5;
                    }
                }
                "https://" | "http://" => {
                    // Find the end of the URL (space, newline, or common delimiters)
                    let url_start = actual_start;
                    let mut url_end = text.len();
                    let remaining_text = &text[actual_start..];
                    let chars: Vec<char> = remaining_text.chars().collect();
                    
                    for (i, &ch) in chars.iter().enumerate() {
                        if ch.is_whitespace() || ch == ',' || ch == ')' || ch == '>' {
                            url_end = actual_start + remaining_text.char_indices().nth(i).unwrap().0;
                            break;
                        }
                        // Stop at period if it's followed by space (end of sentence)
                        if ch == '.' && chars.get(i + 1).map_or(true, |&c| c.is_whitespace()) {
                            url_end = actual_start + remaining_text.char_indices().nth(i).unwrap().0;
                            break;
                        }
                        // Stop at '[' to check for link text
                        if ch == '[' {
                            url_end = actual_start + remaining_text.char_indices().nth(i).unwrap().0;
                            break;
                        }
                    }
                    
                    let url = text[url_start..url_end].to_string();
                    
                    // Check if there's link text after the URL
                    if text.get(url_end..url_end + 1) == Some("[") {
                        if let Some(bracket_end) = text[url_end + 1..].find(']') {
                            let text_start = url_end + 1;
                            let text_end = url_end + 1 + bracket_end;
                            let link_text = text[text_start..text_end].to_string();
                            
                            elements.push(InlineElement::Macro {
                                kind: MacroKind::Link {
                                    url,
                                    text: if link_text.is_empty() { None } else { Some(link_text) },
                                },
                            });
                            current_pos = text_end + 1; // Skip the ']'
                        } else {
                            // No closing bracket, treat as bare URL
                            elements.push(InlineElement::Macro {
                                kind: MacroKind::Link {
                                    url: url.clone(),
                                    text: Some(url),
                                },
                            });
                            current_pos = url_end;
                        }
                    } else {
                        // No link text, use URL as text
                        elements.push(InlineElement::Macro {
                            kind: MacroKind::Link {
                                url: url.clone(),
                                text: Some(url),
                            },
                        });
                        current_pos = url_end;
                    }
                }
                "<<" => {
                    if let Some(end_pos) = text[actual_start + 2..].find(">>") {
                        let actual_end = actual_start + 2 + end_pos;
                        let content = &text[actual_start + 2..actual_end];
                        
                        // Check for comma separator for xref text
                        if let Some(comma_pos) = content.find(',') {
                            let target = content[..comma_pos].trim().to_string();
                            let xref_text = content[comma_pos + 1..].trim().to_string();
                            elements.push(InlineElement::Macro {
                                kind: MacroKind::CrossReference {
                                    target,
                                    text: if xref_text.is_empty() { None } else { Some(xref_text) },
                                },
                            });
                        } else {
                            elements.push(InlineElement::Macro {
                                kind: MacroKind::CrossReference {
                                    target: content.to_string(),
                                    text: None,
                                },
                            });
                        }
                        current_pos = actual_end + 2; // Skip the ">>"
                    } else {
                        elements.push(InlineElement::Text(text[actual_start..actual_start + 2].to_string()));
                        current_pos = actual_start + 2;
                    }
                }
                _ => {
                    elements.push(InlineElement::Text(text[actual_start..actual_start + 1].to_string()));
                    current_pos = actual_start + 1;
                }
            }
        } else {
            // No more special characters, add the rest as text
            elements.push(InlineElement::Text(text[current_pos..].to_string()));
            break;
        }
    }
    
    elements
}

fn parse_inline_macro(pair: pest::iterators::Pair<Rule>) -> InlineElement {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::link_macro => {
                return InlineElement::Macro {
                    kind: parse_link_macro(inner_pair),
                };
            }
            Rule::image_macro => {
                return InlineElement::Macro {
                    kind: parse_image_macro(inner_pair),
                };
            }
            Rule::xref_macro => {
                return InlineElement::Macro {
                    kind: parse_xref_macro(inner_pair),
                };
            }
            _ => {}
        }
    }
    
    InlineElement::Text(String::new())
}

fn parse_link_macro(pair: pest::iterators::Pair<Rule>) -> MacroKind {
    let mut url = String::new();
    let mut text = None;
    
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::url => {
                url = inner_pair.as_str().to_string();
            }
            Rule::link_text => {
                let link_text = inner_pair.as_str().trim();
                if !link_text.is_empty() {
                    text = Some(link_text.to_string());
                }
            }
            _ => {}
        }
    }
    
    MacroKind::Link { url, text }
}

fn parse_image_macro(pair: pest::iterators::Pair<Rule>) -> MacroKind {
    let mut path = String::new();
    let mut attributes = None;
    
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::image_path => {
                path = inner_pair.as_str().to_string();
            }
            Rule::image_attributes => {
                let attrs = inner_pair.as_str().trim();
                if !attrs.is_empty() {
                    attributes = Some(attrs.to_string());
                }
            }
            _ => {}
        }
    }
    
    MacroKind::Image { path, attributes }
}

fn parse_xref_macro(pair: pest::iterators::Pair<Rule>) -> MacroKind {
    let mut target = String::new();
    let mut text = None;
    
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::xref_target => {
                target = inner_pair.as_str().to_string();
            }
            Rule::xref_text => {
                text = Some(inner_pair.as_str().to_string());
            }
            _ => {}
        }
    }
    
    MacroKind::CrossReference { target, text }
}