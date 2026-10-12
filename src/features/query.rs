use crate::config::QueryConfig;
use rusqlite::{types::ValueRef, Connection, OpenFlags};
use serde_json::{Map, Number, Value};
use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Preserve the data directive's array-of-column-objects value shape across query sources.
pub(crate) fn load(
    name: &str,
    config: &QueryConfig,
    base_path: &Path,
    fixtures_only: bool,
) -> Result<Value, String> {
    if fixtures_only {
        return load_fixture(name, config, base_path);
    }
    match std::env::var_os("USI_DB_PATH") {
        Some(path) => load_database(name, config, &PathBuf::from(path)),
        None => load_fixture(name, config, base_path),
    }
}

fn load_fixture(name: &str, config: &QueryConfig, base_path: &Path) -> Result<Value, String> {
    let fixture = config.fixture.as_deref().ok_or_else(|| {
        format!("Query '{name}' has no fixture and USI_DB_PATH is not set")
    })?;
    eprintln!("[query] {name}: fixture {fixture}");
    let path = base_path.join(fixture);
    let value: Value = serde_json::from_slice(&fs::read(&path).map_err(|e| {
        format!("Failed to read query fixture {}: {e}", path.display())
    })?)
    .map_err(|e| format!("Invalid query fixture {}: {e}", path.display()))?;
    let Value::Array(rows) = value else {
        return Err(format!(
            "Query fixture {} must be a JSON array of objects",
            path.display()
        ));
    };
    if rows.iter().any(|row| !row.is_object()) {
        return Err(format!(
            "Query fixture {} must contain only objects",
            path.display()
        ));
    }
    Ok(Value::Array(rows))
}

fn load_database(name: &str, config: &QueryConfig, path: &Path) -> Result<Value, String> {
    if !path.is_absolute() {
        return Err(format!("USI_DB_PATH must be absolute: {}", path.display()));
    }
    let uri = format!("file:{}?mode=ro&immutable=1", encode_uri_path(path)?);
    let connection = Connection::open_with_flags(
        &uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("Failed to open USI_DB_PATH {} readonly: {e}", path.display()))?;
    connection
        .pragma_update(None, "query_only", true)
        .map_err(|e| format!("Failed to enforce extract query_only: {e}"))?;
    let (revision, digest) = read_metadata(&connection)?;
    log_metadata_if_changed(&revision, &digest)?;

    let mut statement = connection
        .prepare(&config.sql)
        .map_err(|e| format!("Query '{name}' failed: {e}"))?;
    if !statement.readonly() || !starts_select_or_with(&config.sql) {
        return Err(format!(
            "Query '{name}' must be a readonly SELECT or WITH statement"
        ));
    }
    // SQLite prepares only the first statement; parse its tail to reject extra statements.
    ensure_single_statement(&connection, &config.sql)?;

    let columns = statement
        .column_names()
        .iter()
        .map(|column| column.to_string())
        .collect::<Vec<_>>();
    let mut seen = BTreeSet::new();
    for column in &columns {
        if !seen.insert(column.clone()) {
            return Err(format!(
                "Query '{name}' has duplicate column name '{column}'"
            ));
        }
    }

    let mut rows = statement
        .query([])
        .map_err(|e| format!("Query '{name}' failed: {e}"))?;
    let mut output = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("Query '{name}' failed: {e}"))?
    {
        let mut object = Map::new();
        for (index, column) in columns.iter().enumerate() {
            let value = match row
                .get_ref(index)
                .map_err(|e| format!("Query '{name}' failed: {e}"))?
            {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(value) => Value::Number(value.into()),
                ValueRef::Real(value) => Value::Number(Number::from_f64(value).ok_or_else(|| {
                    format!("Query '{name}' returned non-finite REAL in column '{column}'")
                })?),
                ValueRef::Text(bytes) => Value::String(
                    std::str::from_utf8(bytes)
                        .map_err(|e| {
                            format!("Query '{name}' returned invalid UTF-8 in column '{column}': {e}")
                        })?
                        .to_string(),
                ),
                ValueRef::Blob(_) => {
                    return Err(format!(
                        "Query '{name}' returned unsupported BLOB column '{column}'"
                    ));
                }
            };
            object.insert(column.clone(), value);
        }
        output.push(Value::Object(object));
    }
    Ok(Value::Array(output))
}

fn encode_uri_path(path: &Path) -> Result<String, String> {
    let path = path
        .to_str()
        .ok_or_else(|| "USI_DB_PATH is not valid UTF-8 for SQLite URI".to_string())?;
    Ok(path
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric()
                || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~' | b':')
            {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect())
}

fn read_metadata(connection: &Connection) -> Result<(String, String), String> {
    let count: i64 = connection
        .query_row("SELECT count(*) FROM meta", [], |row| row.get(0))
        .map_err(|e| format!("Extract metadata row count check failed: {e}"))?;
    if count != 1 {
        return Err(format!(
            "Extract metadata row count must be exactly 1, got {count}"
        ));
    }

    let mut statement = connection
        .prepare("SELECT revision, published_at, source_digest, id FROM meta")
        .map_err(|e| format!("Extract metadata columns check failed: {e}"))?;
    let mut rows = statement
        .query([])
        .map_err(|e| format!("Extract metadata read failed: {e}"))?;
    let row = rows
        .next()
        .map_err(|e| format!("Extract metadata read failed: {e}"))?
        .ok_or_else(|| "Extract metadata row is missing".to_string())?;

    match row
        .get_ref(3)
        .map_err(|e| format!("Extract metadata id read failed: {e}"))?
    {
        ValueRef::Integer(1) => (),
        _ => return Err("Extract metadata id must be INTEGER 1".to_string()),
    }

    let revision = match row
        .get_ref(0)
        .map_err(|e| format!("Extract metadata revision read failed: {e}"))?
    {
        ValueRef::Text(bytes) => std::str::from_utf8(bytes)
            .map_err(|e| format!("Extract metadata revision is invalid UTF-8: {e}"))?
            .to_string(),
        _ => return Err("Extract metadata revision must be TEXT".to_string()),
    };
    if revision.is_empty() {
        return Err("Extract metadata revision must be non-empty TEXT".to_string());
    }

    match row
        .get_ref(1)
        .map_err(|e| format!("Extract metadata published_at read failed: {e}"))?
    {
        ValueRef::Integer(seconds) if seconds > 0 => (),
        ValueRef::Integer(_) => {
            return Err("Extract metadata published_at must be positive UTC Unix seconds".to_string())
        }
        _ => return Err("Extract metadata published_at must be INTEGER UTC Unix seconds".to_string()),
    }

    let digest = match row
        .get_ref(2)
        .map_err(|e| format!("Extract metadata source_digest read failed: {e}"))?
    {
        ValueRef::Text(bytes) => std::str::from_utf8(bytes)
            .map_err(|e| format!("Extract metadata source_digest is invalid UTF-8: {e}"))?
            .to_string(),
        _ => return Err("Extract metadata source_digest must be TEXT".to_string()),
    };
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(
            "Extract metadata source_digest must be 64 lowercase hexadecimal characters"
                .to_string(),
        );
    }
    if rows
        .next()
        .map_err(|e| format!("Extract metadata read failed: {e}"))?
        .is_some()
    {
        return Err("Extract metadata row count must be exactly 1".to_string());
    }
    Ok((revision, digest))
}

fn log_metadata_if_changed(revision: &str, digest: &str) -> Result<(), String> {
    static LAST: OnceLock<Mutex<Option<(String, String)>>> = OnceLock::new();
    let mut last = LAST
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Extract metadata change logger lock is poisoned".to_string())?;
    let observed = (revision.to_string(), digest.to_string());
    if last.as_ref() != Some(&observed) {
        eprintln!("[query] extract revision {revision}, source_digest {digest}");
        *last = Some(observed);
    }
    Ok(())
}

fn starts_select_or_with(sql: &str) -> bool {
    let mut rest = sql.trim_start();
    loop {
        if let Some(comment) = rest.strip_prefix("--") {
            let Some((_, tail)) = comment.split_once('\n') else {
                return false;
            };
            rest = tail.trim_start();
        } else if let Some(comment) = rest.strip_prefix("/*") {
            let Some((_, tail)) = comment.split_once("*/") else {
                return false;
            };
            rest = tail.trim_start();
        } else {
            break;
        }
    }
    let Some(word) = rest.split(|character: char| !character.is_ascii_alphabetic()).next() else {
        return false;
    };
    word.eq_ignore_ascii_case("SELECT") || word.eq_ignore_ascii_case("WITH")
}

fn ensure_single_statement(connection: &Connection, sql: &str) -> Result<(), String> {
    let sql = CString::new(sql).map_err(|_| "Query SQL contains a NUL byte".to_string())?;
    let mut statement = std::ptr::null_mut();
    let mut tail = std::ptr::null();
    let code = unsafe {
        rusqlite::ffi::sqlite3_prepare_v2(
            connection.handle(),
            sql.as_ptr(),
            -1,
            &mut statement,
            &mut tail,
        )
    };
    if code != rusqlite::ffi::SQLITE_OK {
        if !statement.is_null() {
            unsafe { rusqlite::ffi::sqlite3_finalize(statement) };
        }
        let message = unsafe {
            std::ffi::CStr::from_ptr(rusqlite::ffi::sqlite3_errmsg(connection.handle()))
                .to_string_lossy()
                .into_owned()
        };
        return Err(format!("SQLite could not parse query: {message}"));
    }
    if !statement.is_null() {
        unsafe { rusqlite::ffi::sqlite3_finalize(statement) };
    }

    let remaining = unsafe { std::ffi::CStr::from_ptr(tail) }.to_bytes();
    let remaining = CString::new(remaining).map_err(|_| "Query SQL contains a NUL byte".to_string())?;
    let mut trailing = std::ptr::null_mut();
    let mut tail = std::ptr::null();
    let code = unsafe {
        rusqlite::ffi::sqlite3_prepare_v2(
            connection.handle(),
            remaining.as_ptr(),
            -1,
            &mut trailing,
            &mut tail,
        )
    };
    if code != rusqlite::ffi::SQLITE_OK {
        if !trailing.is_null() {
            unsafe { rusqlite::ffi::sqlite3_finalize(trailing) };
        }
        return Err("Query contains invalid trailing SQL".to_string());
    }
    if !trailing.is_null() {
        unsafe { rusqlite::ffi::sqlite3_finalize(trailing) };
        return Err("Query must contain exactly one SQL statement".to_string());
    }
    Ok(())
}
