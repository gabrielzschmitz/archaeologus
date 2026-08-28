use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolType {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Type,
    Const,
    Static,
    Module,
    Method,
    Field,
    Variant,
    Interface,
    Class,
    Macro,
    Constructor,
}

impl std::fmt::Display for SymbolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function => write!(f, "function"),
            Self::Struct => write!(f, "struct"),
            Self::Enum => write!(f, "enum"),
            Self::Trait => write!(f, "trait"),
            Self::Impl => write!(f, "impl"),
            Self::Type => write!(f, "type"),
            Self::Const => write!(f, "const"),
            Self::Static => write!(f, "static"),
            Self::Module => write!(f, "module"),
            Self::Method => write!(f, "method"),
            Self::Field => write!(f, "field"),
            Self::Variant => write!(f, "variant"),
            Self::Interface => write!(f, "interface"),
            Self::Class => write!(f, "class"),
            Self::Macro => write!(f, "macro"),
            Self::Constructor => write!(f, "constructor"),
        }
    }
}

impl std::str::FromStr for SymbolType {
    type Err = crate::error::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "function" => Ok(Self::Function),
            "struct" => Ok(Self::Struct),
            "enum" => Ok(Self::Enum),
            "trait" => Ok(Self::Trait),
            "impl" => Ok(Self::Impl),
            "type" => Ok(Self::Type),
            "const" => Ok(Self::Const),
            "static" => Ok(Self::Static),
            "module" => Ok(Self::Module),
            "method" => Ok(Self::Method),
            "field" => Ok(Self::Field),
            "variant" => Ok(Self::Variant),
            "interface" => Ok(Self::Interface),
            "class" => Ok(Self::Class),
            "macro" => Ok(Self::Macro),
            "constructor" => Ok(Self::Constructor),
            _ => Err(crate::error::AppError::Validation(format!(
                "Unknown symbol type: {s}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Symbol {
    pub id: Uuid,
    pub file_id: Uuid,
    pub repository_id: Uuid,
    pub name: String,
    pub symbol_type: String,
    pub language: String,
    pub line_start: i32,
    pub line_end: i32,
    pub col_start: i32,
    pub col_end: i32,
    pub visibility: Option<String>,
    pub doc_comment: Option<String>,
    pub raw_text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolCreate {
    pub file_id: Uuid,
    pub repository_id: Uuid,
    pub name: String,
    pub symbol_type: SymbolType,
    pub language: String,
    pub line_start: i32,
    pub line_end: i32,
    pub col_start: i32,
    pub col_end: i32,
    pub visibility: Option<String>,
    pub doc_comment: Option<String>,
    pub raw_text: String,
}
