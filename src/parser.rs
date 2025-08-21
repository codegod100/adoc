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
    
    blocks
}

fn parse_block(pair: pest::iterators::Pair<Rule>) -> Option<Block> {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::section => return Some(parse_section(inner_pair)),
            Rule::delimited_block => return Some(parse_delimited_block(inner_pair)),
            Rule::list => return Some(parse_list(inner_pair)),
            Rule::paragraph => return Some(parse_paragraph(inner_pair)),
            Rule::block_metadata => return Some(parse_block_metadata(inner_pair)),
            _ => {}
        }
    }
    None
}

fn parse_section(pair: pest::iterators::Pair<Rule>) -> Block {
    let content = pair.as_str();
    let level = content.chars().take_while(|&c| c == '=').count();
    let title = content.trim_start_matches('=').trim().to_string();
    
    Block::Section { level, title, blocks: Vec::new() }
}


fn parse_delimited_block(pair: pest::iterators::Pair<Rule>) -> Block {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::listing_block => {
                return Block::DelimitedBlock {
                    kind: DelimitedBlockKind::Listing,
                    content: extract_delimited_content(inner_pair, Rule::listing_content),
                };
            }
            Rule::example_block => {
                return Block::DelimitedBlock {
                    kind: DelimitedBlockKind::Example,
                    content: extract_delimited_content(inner_pair, Rule::example_content),
                };
            }
            Rule::literal_block => {
                return Block::DelimitedBlock {
                    kind: DelimitedBlockKind::Literal,
                    content: extract_delimited_content(inner_pair, Rule::literal_content),
                };
            }
            Rule::sidebar_block => {
                return Block::DelimitedBlock {
                    kind: DelimitedBlockKind::Sidebar,
                    content: extract_delimited_content(inner_pair, Rule::sidebar_content),
                };
            }
            Rule::quote_block => {
                return Block::DelimitedBlock {
                    kind: DelimitedBlockKind::Quote,
                    content: extract_delimited_content(inner_pair, Rule::quote_content),
                };
            }
            _ => {}
        }
    }
    
    Block::DelimitedBlock {
        kind: DelimitedBlockKind::Literal,
        content: String::new(),
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
    let mut level = 1;
    let mut content = Vec::new();
    
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::unordered_marker => {
                level = inner_pair.as_str().chars().count();
            }
            Rule::list_text => {
                content = parse_inline_elements(inner_pair);
            }
            _ => {}
        }
    }
    
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
    let mut level = 1;
    let mut content = Vec::new();
    
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::ordered_marker => {
                level = inner_pair.as_str().chars().count();
            }
            Rule::list_text => {
                content = parse_inline_elements(inner_pair);
            }
            _ => {}
        }
    }
    
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
    
    for inner_pair in pair.into_inner() {
        if inner_pair.as_rule() == Rule::paragraph_line {
            for line_inner in inner_pair.into_inner() {
                if line_inner.as_rule() == Rule::paragraph_text {
                    content.extend(parse_inline_elements(line_inner));
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