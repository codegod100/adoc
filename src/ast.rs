#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub header: Option<Header>,
    pub body: Vec<Block>,
}

impl Document {
    pub fn to_html(&self) -> String {
        let mut html = String::new();
        
        if let Some(header) = &self.header {
            html.push_str(&format!("<h1>{}</h1>\n", escape_html(&header.title)));
        }
        
        for block in &self.body {
            html.push_str(&block.to_html());
        }
        
        html
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub title: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Section {
        level: usize,
        title: String,
        blocks: Vec<Block>,
    },
    Paragraph {
        content: Vec<InlineElement>,
    },
    DelimitedBlock {
        kind: DelimitedBlockKind,
        content: String,
        language: Option<String>,
    },
    List {
        kind: ListKind,
        items: Vec<ListItem>,
    },
    BlockMetadata {
        kind: BlockMetadataKind,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DelimitedBlockKind {
    Listing,
    Example,
    Literal,
    Sidebar,
    Quote,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListKind {
    Unordered,
    Ordered,
    Description,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListItem {
    Unordered {
        level: usize,
        content: Vec<InlineElement>,
    },
    Ordered {
        level: usize,
        content: Vec<InlineElement>,
    },
    Description {
        term: String,
        description: Option<Vec<InlineElement>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockMetadataKind {
    Title(String),
    Attribute(Vec<String>),
    Anchor(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineElement {
    Text(String),
    Formatted {
        kind: FormattedTextKind,
        content: Vec<InlineElement>,
    },
    Macro {
        kind: MacroKind,
    },
    LineBreak,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormattedTextKind {
    Strong,
    Emphasis,
    Monospace,
    Superscript,
    Subscript,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MacroKind {
    Link {
        url: String,
        text: Option<String>,
    },
    Image {
        path: String,
        attributes: Option<String>,
    },
    CrossReference {
        target: String,
        text: Option<String>,
    },
}

impl Block {
    pub fn to_html(&self) -> String {
        match self {
            Block::Section { level, title, blocks } => {
                let heading_level = (*level).min(6);
                let mut html = format!("<h{}>{}</h{}>\n", heading_level, escape_html(title), heading_level);
                for block in blocks {
                    html.push_str(&block.to_html());
                }
                html
            }
            Block::Paragraph { content } => {
                format!("<p>{}</p>\n", inline_elements_to_html(content))
            }
            Block::DelimitedBlock { kind, content, language } => {
                match kind {
                    DelimitedBlockKind::Listing => {
                        if let Some(lang) = language {
                            format!("<pre><code class=\"language-{}\">{}</code></pre>\n", escape_html(lang), escape_html(content))
                        } else {
                            format!("<pre><code>{}</code></pre>\n", escape_html(content))
                        }
                    },
                    DelimitedBlockKind::Example => format!("<div class=\"example\">{}</div>\n", escape_html(content)),
                    DelimitedBlockKind::Literal => format!("<pre>{}</pre>\n", escape_html(content)),
                    DelimitedBlockKind::Sidebar => format!("<aside>{}</aside>\n", escape_html(content)),
                    DelimitedBlockKind::Quote => format!("<blockquote>{}</blockquote>\n", escape_html(content)),
                }
            }
            Block::List { kind, items } => {
                match kind {
                    ListKind::Unordered => {
                        let mut html = String::from("<ul>\n");
                        for item in items {
                            html.push_str(&item.to_html());
                        }
                        html.push_str("</ul>\n");
                        html
                    }
                    ListKind::Ordered => {
                        let mut html = String::from("<ol>\n");
                        for item in items {
                            html.push_str(&item.to_html());
                        }
                        html.push_str("</ol>\n");
                        html
                    }
                    ListKind::Description => {
                        let mut html = String::from("<dl>\n");
                        for item in items {
                            html.push_str(&item.to_html());
                        }
                        html.push_str("</dl>\n");
                        html
                    }
                }
            }
            Block::BlockMetadata { .. } => String::new(),
        }
    }
}

impl ListItem {
    pub fn to_html(&self) -> String {
        match self {
            ListItem::Unordered { content, .. } | ListItem::Ordered { content, .. } => {
                format!("<li>{}</li>\n", inline_elements_to_html(content))
            }
            ListItem::Description { term, description } => {
                let mut html = format!("<dt>{}</dt>\n", escape_html(term));
                if let Some(desc) = description {
                    html.push_str(&format!("<dd>{}</dd>\n", inline_elements_to_html(desc)));
                }
                html
            }
        }
    }
}

impl InlineElement {
    pub fn to_html(&self) -> String {
        match self {
            InlineElement::Text(text) => escape_html(text),
            InlineElement::Formatted { kind, content } => {
                let inner = inline_elements_to_html(content);
                match kind {
                    FormattedTextKind::Strong => format!("<strong>{}</strong>", inner),
                    FormattedTextKind::Emphasis => format!("<em>{}</em>", inner),
                    FormattedTextKind::Monospace => format!("<code>{}</code>", inner),
                    FormattedTextKind::Superscript => format!("<sup>{}</sup>", inner),
                    FormattedTextKind::Subscript => format!("<sub>{}</sub>", inner),
                }
            }
            InlineElement::Macro { kind } => {
                match kind {
                    MacroKind::Link { url, text } => {
                        let link_text = text.as_ref().map(|t| escape_html(t)).unwrap_or_else(|| escape_html(url));
                        format!("<a href=\"{}\">{}</a>", escape_html(url), link_text)
                    }
                    MacroKind::Image { path, attributes } => {
                        let alt = attributes.as_ref().map(|a| escape_html(a)).unwrap_or_else(|| "Image".to_string());
                        format!("<img src=\"{}\" alt=\"{}\">", escape_html(path), alt)
                    }
                    MacroKind::CrossReference { target, text } => {
                        let link_text = text.as_ref().map(|t| escape_html(t)).unwrap_or_else(|| escape_html(target));
                        format!("<a href=\"#{}\">{}</a>", escape_html(target), link_text)
                    }
                }
            }
            InlineElement::LineBreak => "<br>\n".to_string(),
        }
    }
}

fn inline_elements_to_html(elements: &[InlineElement]) -> String {
    elements.iter().map(|e| e.to_html()).collect::<String>()
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}