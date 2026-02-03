use rusqlite::{Connection, Result as SqlResult};
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
        let params: Vec<Box<dyn rusqlite::ToSql>> = params.into_iter()
            .map(|v| Box::new(json_to_sql_param(v)) as Box<dyn rusqlite::ToSql>)
            .collect();
        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter()
            .map(|p| p.as_ref())
            .collect();
        self.conn.execute(query, params_refs.as_slice())
    }

    pub fn query(&self, query: &str, params: Vec<Value>) -> SqlResult<Vec<Value>> {
        let mut stmt = self.conn.prepare(query)?;
        let params: Vec<Box<dyn rusqlite::ToSql>> = params.into_iter()
            .map(|v| Box::new(json_to_sql_param(v)) as Box<dyn rusqlite::ToSql>)
            .collect();
        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter()
            .map(|p| p.as_ref())
            .collect();
        
        let column_count = stmt.column_count();
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let mut map = serde_json::Map::new();
            for i in 0..column_count {
                let column_name = row.as_ref().column_name(i).unwrap_or("");
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

// High-level database operations
pub struct Table {
    name: String,
}

impl Table {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn create(&self, schema: &str) -> Result<(), String> {
        let query = format!("CREATE TABLE IF NOT EXISTS {} ({})", self.name, schema);
        execute(&query, vec![])
            .map(|_| ())
    }

    pub fn insert(&self, data: Value) -> Result<i64, String> {
        if let Value::Object(map) = data {
            let columns: Vec<String> = map.keys().cloned().collect();
            let placeholders: Vec<String> = (0..columns.len()).map(|_| "?".to_string()).collect();
            let values: Vec<Value> = map.values().cloned().collect();

            let query = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                self.name,
                columns.join(", "),
                placeholders.join(", ")
            );

            execute(&query, values)?;
            
            // Get last insert rowid
            let result = query_one("SELECT last_insert_rowid() as id", vec![])?;
            Ok(result.get("id").and_then(|v| v.as_i64()).unwrap_or(0))
        } else {
            Err("Data must be an object".to_string())
        }
    }

    pub fn find(&self, id: i64) -> Result<Value, String> {
        let query = format!("SELECT * FROM {} WHERE id = ?", self.name);
        query_one(&query, vec![json!(id)])
    }

    pub fn find_all(&self) -> Result<Vec<Value>, String> {
        let sql = format!("SELECT * FROM {}", self.name);
        query(&sql, vec![])
    }

    pub fn find_where(&self, conditions: Value) -> Result<Vec<Value>, String> {
        if let Value::Object(map) = conditions {
            let where_clause: Vec<String> = map.keys()
                .map(|k| format!("{} = ?", k))
                .collect();
            let values: Vec<Value> = map.values().cloned().collect();

            let sql = format!(
                "SELECT * FROM {} WHERE {}",
                self.name,
                where_clause.join(" AND ")
            );

            query(&sql, values)
        } else {
            Err("Conditions must be an object".to_string())
        }
    }

    pub fn update(&self, id: i64, data: Value) -> Result<usize, String> {
        if let Value::Object(map) = data {
            let set_clause: Vec<String> = map.keys()
                .map(|k| format!("{} = ?", k))
                .collect();
            let mut values: Vec<Value> = map.values().cloned().collect();
            values.push(json!(id));

            let query = format!(
                "UPDATE {} SET {} WHERE id = ?",
                self.name,
                set_clause.join(", ")
            );

            execute(&query, values)
        } else {
            Err("Data must be an object".to_string())
        }
    }

    pub fn delete(&self, id: i64) -> Result<usize, String> {
        let query = format!("DELETE FROM {} WHERE id = ?", self.name);
        execute(&query, vec![json!(id)])
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

pub fn table(name: &str) -> Table {
    Table::new(name)
}

// Conversion helpers
#[derive(Debug, Clone)]
enum SqlParam {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
}

impl rusqlite::ToSql for SqlParam {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            SqlParam::Null => Ok(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Null)),
            SqlParam::Integer(i) => Ok(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Integer(*i))),
            SqlParam::Real(f) => Ok(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Real(*f))),
            SqlParam::Text(s) => Ok(rusqlite::types::ToSqlOutput::Owned(rusqlite::types::Value::Text(s.clone()))),
        }
    }
}

fn json_to_sql_param(value: Value) -> SqlParam {
    match value {
        Value::Null => SqlParam::Null,
        Value::Bool(b) => SqlParam::Integer(if b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SqlParam::Integer(i)
            } else if let Some(f) = n.as_f64() {
                SqlParam::Real(f)
            } else {
                SqlParam::Null
            }
        }
        Value::String(s) => SqlParam::Text(s),
        _ => SqlParam::Text(value.to_string()),
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
