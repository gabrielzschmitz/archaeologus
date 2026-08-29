//! `archaeologist-search` — symbol and code search using `PostgreSQL` `pg_trgm`.
//!
//! # Quick start
//! ```no_run
//! use archaeologist_search::symbol_search::{search_symbols, SymbolQuery};
//!
//! # async fn example(pool: sqlx::PgPool) -> Result<(), sqlx::Error> {
//! let result = search_symbols(
//!     &pool,
//!     &SymbolQuery::new("main").language("rust").limit(10),
//! ).await?;
//! println!("{} symbols found", result.total);
//! # Ok(())
//! # }
//! ```

pub mod code_search;
pub mod symbol_search;

pub use code_search::{search_code, search_files, CodeQuery, CodeSearchResult};
pub use symbol_search::{search_symbols, SymbolQuery, SymbolSearchResult};
