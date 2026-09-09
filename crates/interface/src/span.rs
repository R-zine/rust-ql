/// A region within the source text.
///
/// The span is half-open:
/// [start, end)
///
/// Example:
/// "SELECT"
///  012345
///
/// Span { start: 0, end: 6 }
/// covers the entire word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Creates a new span.
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns the length of the span.
    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns true if the span is empty.
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Creates a span covering both spans.
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Returns true if the position is within the span.
    pub const fn contains(&self, position: usize) -> bool {
        position >= self.start && position < self.end
    }

    /// Extracts the text covered by this span.
    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}
