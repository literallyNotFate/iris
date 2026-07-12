use crate::models::Palette;

/// Semantic color role a TextMate rule maps to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorKey {
    Fg,
    Comment,
    String,
    Number,
    Keyword,
    Operator,
    Func,
    TypeName,
    Attribute,
    Variable,
    Constant,
    Tag,
    Added,
    Deleted,
    Changed,
}

impl ColorKey {
    /// Resolve this color role to the actual hex string for a given palette
    pub fn resolve<'a>(&self, colors: &'a Palette) -> &'a str {
        match self {
            ColorKey::Fg => &colors.fg,
            ColorKey::Comment => &colors.comment,
            ColorKey::String => &colors.string,
            ColorKey::Number => &colors.number,
            ColorKey::Keyword => &colors.keyword,
            ColorKey::Operator => &colors.operator,
            ColorKey::Func => &colors.func,
            ColorKey::TypeName => &colors.type_name,
            ColorKey::Attribute => &colors.attribute,
            ColorKey::Variable => &colors.variable,
            ColorKey::Constant => &colors.constant,
            ColorKey::Tag => &colors.tag,
            ColorKey::Added => &colors.added,
            ColorKey::Deleted => &colors.deleted,
            ColorKey::Changed => &colors.changed,
        }
    }
}
