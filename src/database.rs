use rusqlite::{Connection, Result as SqlResult, params};
use serde_json::{json, Value};
use std::sync::Mutex;
use once_cell::sync::Lazy;

static DB: Lazy<Mutex<Database>> = Lazy::new(|| {
    Mutex::new(Database::new("hrml.db").expect("Failed to initialize database"))
});

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn execute(&mut self, query: &str, params: Vec<Value>) -> SqlResult<usize> {
        let mut stmt = self.conn.prepare(query)?;
        let params: Vec<rusqlite::types::Value> = params.into_iter().map(json_to_sql).collect();
        stmt.execute(params.as_slice())
    }

    pub fn query(&self, query: &str, params: Vec<Value>) -> SqlResult<Vec<Value>> {
        let mut stmt = self.conn.prepare(query)?;
        let params: Vec<rusqlite::types::Value> = params.into_iter().map(json_to_sql).collect();
        
        let column_count = stmt.column_count();
        let rows = stmt.query_map(params.as_slice(), |row| {
            let mut map = serde_json::Map::new();
            for i in 0..column_count {
                let column_name = stmt.column_name(i).unwrap_or("");
                let value: rusqlite::types::Value = row.get(i)?;
                map.insert(column_name.to_string(), sql_to_json(value));
            }
            Ok(Value::Object(map))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn query_one(&self, query: &str, params: Vec<Value>) -> SqlResult<Value> {
        let results = self.query(query, params)?;
        Ok(results.into_iter().next().unwrap_or(Value::Null))
    }
}

// Global database access
pub fn execute(query: &str, params: Vec<Value>) -> Result<usize, String> {
    DB.lock()
        .unwrap()
        .execute(query, params)
        .map_err(|e| e.to_string())
}

pub fn query(query: &str, params: Vec<Value>) -> Result<Vec<Value>, String> {
    DB.lock()
        .unwrap()
        .query(query, params)
        .map_err(|e| e.to_string())
}

pub fn query_one(query: &str, params: Vec<Value>) -> Result<Value, String> {
    DB.lock()
        .unwrap()
        .query_one(query, params)
        .map_err(|e| e.to_string())
}

// Conversion helpers
fn json_to_sql(value: Value) -> rusqlite::types::Value {
    match value {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(if b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                rusqlite::types::Value::Real(f)
            } else {
                rusqlite::types::Value::Null
            }
        }
        Value::String(s) => rusqlite::types::Value::Text(s),
        _ => rusqlite::types::Value::Text(value.to_string()),
    }
}

fn sql_to_json(value: rusqlite::types::Value) -> Value {
    match value {
        rusqlite::types::Value::Null => Value::Null,
        rusqlite::types::Value::Integer(i) => json!(i),
        rusqlite::types::Value::Real(f) => json!(f),
        rusqlite::types::Value::Text(s) => json!(s),
        rusqlite::types::Value::Blob(b) => json!(b),
    }
}
