//! Each pod's log stream, held as a read-only text the editor shows, capped from its start.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use groove_objects::{Logged, Stop};
use groove_text::{Buffer, Document};
use groove_types::{Edit, LogKey, LogLine};

/// How many bytes of text a stream holds; past it, its oldest lines go until it holds nine tenths.
pub const CAP: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Streaming {
    #[default]
    Opening,
    Open,
    /// Every container's stream ended; a followed one opens again.
    Ended,
    Failed(String),
}

#[derive(Debug, Default)]
pub struct Log {
    lines: VecDeque<LogLine>,
    bytes: usize,
    /// Every line it was ever given, those shed included.
    pub total: u64,
    pub state: Streaming,
    buffer: Buffer,
    readers: BTreeSet<String>,
    stop: Stop,
}

impl Log {
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn lines(&self) -> usize {
        self.lines.len()
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    fn landed(&mut self, lines: Vec<LogLine>) {
        let text: String = lines.iter().map(shown).collect();
        self.buffer.append(&text);
        self.total += lines.len() as u64;
        self.bytes += lines.iter().map(|one| one.text.len()).sum::<usize>();
        self.lines.extend(lines);
        if self.bytes <= CAP {
            return;
        }
        let mut shed = 0;
        while self.bytes > CAP / 10 * 9 {
            let Some(one) = self.lines.pop_front() else {
                break;
            };
            self.bytes -= one.text.len();
            shed += 1;
        }
        self.buffer.shed(shed);
    }
}

/// One line as the editor shows it: its clock, then its container where it has one.
fn shown(line: &LogLine) -> String {
    let mut out = line.clock();
    out.push(' ');
    if let Some(container) = &line.container {
        out.push_str(container);
        out.push(' ');
    }
    out.push_str(&line.text);
    out.push('\n');
    out
}

#[derive(Debug, Default)]
pub struct Logs {
    streams: BTreeMap<LogKey, Log>,
}

impl Logs {
    pub fn get(&self, key: &LogKey) -> Option<&Log> {
        self.streams.get(key)
    }

    pub fn read_by<'a>(&'a self, reader: &'a str) -> impl Iterator<Item = &'a LogKey> {
        let reads =
            move |(key, one): (&'a LogKey, &'a Log)| one.readers.contains(reader).then_some(key);
        self.streams.iter().filter_map(reads)
    }

    /// `reader` reads `key`; the stop of a stream to open, when none ran for it.
    pub fn lease(&mut self, key: &LogKey, reader: &str) -> Option<&Stop> {
        let started = !self.streams.contains_key(key);
        let one = self.streams.entry(key.clone()).or_insert_with(|| Log {
            buffer: Buffer::new(Document::plain("stream.log", "")),
            ..Log::default()
        });
        one.readers.insert(reader.to_string());
        started.then_some(&one.stop)
    }

    /// `reader` reads no stream any more; the keys it leaves without a reader.
    pub fn release(&mut self, reader: &str) -> Vec<LogKey> {
        let mut unread = Vec::new();
        for (key, one) in &mut self.streams {
            if one.readers.remove(reader) && one.readers.is_empty() {
                unread.push(key.clone());
            }
        }
        unread
    }

    /// The stream closed and its lines dropped, if nothing reads it still.
    pub fn drop_unread(&mut self, key: &LogKey) {
        if self
            .streams
            .get(key)
            .is_some_and(|one| one.readers.is_empty())
            && let Some(one) = self.streams.remove(key)
        {
            one.stop.stop();
        }
    }

    /// A batch of a stream still held; one dropped meanwhile is ignored.
    pub fn apply(&mut self, key: &LogKey, batch: Logged) {
        let Some(one) = self.streams.get_mut(key) else {
            return;
        };
        match batch {
            Logged::Lines(lines) => one.landed(lines),
            Logged::Streaming => one.state = Streaming::Open,
            Logged::Ended => one.state = Streaming::Ended,
            Logged::Failed(why) => one.state = Streaming::Failed(why),
        }
    }

    /// A move of the caret or of what it holds; anything that would change the text is refused.
    pub fn edit(&mut self, key: &LogKey, edit: &Edit) -> bool {
        let moves = matches!(
            edit,
            Edit::Move(_) | Edit::Extend(_) | Edit::SelectAll | Edit::SelectWord | Edit::SelectLine
        );
        match self.streams.get_mut(key).filter(|_| moves) {
            Some(one) => {
                one.buffer.edit(edit);
                true
            }
            None => false,
        }
    }
}
