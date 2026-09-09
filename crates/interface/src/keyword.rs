#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    Select,
    From,
    Where,
    Insert,
    Into,
    Values,
    Create,
    Table,
    As,
    Primary,
    Key,
}
