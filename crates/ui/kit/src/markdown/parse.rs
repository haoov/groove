//! Markdown as blocks of styled spans, which the layout wraps.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub strong: bool,
    pub em: bool,
    pub code: bool,
    pub link: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bullet {
    Dot,
    Number(u64),
    Box(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Paragraph,
    Heading(u8),
    Item(Bullet),
    /// One line of a fenced or indented block.
    Code,
    Rule,
}

/// One block, `depth` lists deep; a code block holds one span per line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub kind: Kind,
    pub depth: usize,
    pub quoted: bool,
    pub spans: Vec<Span>,
}

pub fn blocks(text: &str) -> Vec<Block> {
    let options =
        Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let mut reader = Reader::default();
    for event in Parser::new_ext(text, options) {
        reader.read(event);
    }
    reader.finish();
    reader.out
}

#[derive(Default)]
struct Reader {
    out: Vec<Block>,
    open: Option<Block>,
    lists: Vec<Option<u64>>,
    quoted: usize,
    strong: usize,
    em: usize,
    link: Option<String>,
}

impl Reader {
    fn read(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) if self.in_code() => self.lines(&text),
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                self.said(&text, false)
            }
            Event::Code(text) => self.said(&text, true),
            Event::SoftBreak | Event::HardBreak => self.said("\n", false),
            Event::Rule => {
                self.begin(Kind::Rule);
                self.finish();
            }
            Event::TaskListMarker(done) => self.ticked(done),
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph if self.item_is_empty() => {}
            Tag::Paragraph | Tag::TableRow | Tag::TableHead => self.begin(Kind::Paragraph),
            Tag::Heading { level, .. } => self.begin(Kind::Heading(heading(level))),
            Tag::List(start) => {
                self.finish();
                self.lists.push(start);
            }
            Tag::Item => {
                let bullet = self.bullet();
                self.begin(Kind::Item(bullet));
            }
            Tag::BlockQuote(_) => self.quoted += 1,
            Tag::CodeBlock(_) => self.begin(Kind::Code),
            Tag::Strong => self.strong += 1,
            Tag::Emphasis => self.em += 1,
            Tag::Link { dest_url, .. } => self.link = Some(dest_url.to_string()),
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::Item
            | TagEnd::CodeBlock
            | TagEnd::TableRow
            | TagEnd::TableHead => self.finish(),
            TagEnd::TableCell => self.said("  ·  ", false),
            TagEnd::List(_) => {
                self.finish();
                self.lists.pop();
            }
            TagEnd::BlockQuote(_) => self.quoted = self.quoted.saturating_sub(1),
            TagEnd::Strong => self.strong = self.strong.saturating_sub(1),
            TagEnd::Emphasis => self.em = self.em.saturating_sub(1),
            TagEnd::Link => self.link = None,
            _ => {}
        }
    }

    fn begin(&mut self, kind: Kind) {
        self.finish();
        self.open = Some(Block {
            kind,
            depth: self.lists.len(),
            quoted: self.quoted > 0,
            spans: Vec::new(),
        });
    }

    fn finish(&mut self) {
        let Some(block) = self.open.take() else {
            return;
        };
        let bare = matches!(block.kind, Kind::Item(_) | Kind::Rule);
        if bare || !block.spans.is_empty() {
            self.out.push(block);
        }
    }

    fn said(&mut self, text: &str, code: bool) {
        if self.open.is_none() {
            self.begin(Kind::Paragraph);
        }
        let span = Span {
            text: text.to_string(),
            strong: self.strong > 0,
            em: self.em > 0,
            code,
            link: self.link.clone(),
        };
        if let Some(block) = self.open.as_mut() {
            block.spans.push(span);
        }
    }

    /// A code block's text, a span per line.
    fn lines(&mut self, text: &str) {
        for line in text.lines() {
            self.said(line, true);
        }
    }

    fn in_code(&self) -> bool {
        self.open.as_ref().is_some_and(|one| one.kind == Kind::Code)
    }

    fn item_is_empty(&self) -> bool {
        self.open
            .as_ref()
            .is_some_and(|one| matches!(one.kind, Kind::Item(_)) && one.spans.is_empty())
    }

    fn bullet(&mut self) -> Bullet {
        match self.lists.last_mut() {
            Some(Some(next)) => {
                *next += 1;
                Bullet::Number(*next - 1)
            }
            _ => Bullet::Dot,
        }
    }

    fn ticked(&mut self, done: bool) {
        if let Some(block) = self.open.as_mut() {
            block.kind = Kind::Item(Bullet::Box(done));
        }
    }
}

fn heading(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        _ => 3,
    }
}
