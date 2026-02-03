use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use serde_json::Value;
use std::path::PathBuf;

const HRML_PYTHON_LIB: &str = include_str!("runtime/hrml.py");

pub struct Runtime {
    endpoints_path: PathBuf,
}

impl Runtime {
    pub fn new(endpoints_path: &str) -> Self {
        let endpoints_path = PathBuf::from(endpoints_path);

        // Initialize Python with embedded hrml module and database bindings
        Python::with_gil(|py| {
            let sys = py.import("sys").expect("Failed to import sys");
            let sys_path = sys.getattr("path").expect("Failed to get sys.path");

            // IMPORTANT: First create embedded modules BEFORE adding paths that might shadow them

            // Create hrml module from embedded source (do this BEFORE adding lib path)
            eprintln!("[DEBUG] Creating embedded hrml module...");
            match PyModule::from_code(py, HRML_PYTHON_LIB, "hrml.py", "hrml") {
                Ok(module) => {
                    eprintln!("[DEBUG] Successfully created hrml module from embedded code");
                    // Register in sys.modules to make it available for import
                    if let Ok(sys) = py.import("sys") {
                        if let Ok(sys_modules) = sys.getattr("modules") {
                            if let Err(e) = sys_modules.set_item("hrml", module) {
                                eprintln!(
                                    "[WARNING] Failed to register hrml in sys.modules: {}",
                                    e
                                );
                            } else {
                                eprintln!("[DEBUG] Registered hrml module in sys.modules");
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] Failed to load embedded hrml.py: {}", e);
                }
            }

            // Create db module with database functions
            if let Err(e) = Self::create_db_module(py) {
                eprintln!("[WARNING] Failed to create db module: {}", e);
            }

            // Now add project directory to Python path for 'endpoints' module
            // Do NOT add lib directory - we want to use embedded hrml, not lib/hrml.py
            if let Some(parent) = endpoints_path.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                eprintln!(
                    "[DEBUG] Adding endpoints parent to sys.path: {}",
                    parent_str
                );
                if let Err(e) = sys_path.call_method1("insert", (0, parent_str.clone())) {
                    eprintln!(
                        "[WARNING] Failed to add '{}' to sys.path: {}",
                        parent_str, e
                    );
                }
            }

            // Print current sys.path for debugging
            eprintln!(
                "[DEBUG] Current sys.path (first 3): {:?}",
                sys_path
                    .extract::<Vec<String>>()
                    .unwrap_or_default()
                    .get(0..3.min(sys_path.len().unwrap_or(0)))
            );
        });

        Self { endpoints_path }
    }

    fn create_db_module(py: Python) -> PyResult<()> {
        use crate::db;
        use pyo3::wrap_pyfunction;

        let db_module = PyModule::new(py, "db")?;

        #[pyo3::pyfunction]
        fn table_create(name: String, schema: String) -> PyResult<()> {
            db::table(&name)
                .create(&schema)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
        }

        #[pyo3::pyfunction]
        fn table_insert(name: String, data: String) -> PyResult<i64> {
            let value: Value = serde_json::from_str(&data)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            db::table(&name)
                .insert(value)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
        }

        #[pyo3::pyfunction]
        fn table_find(name: String, id: i64) -> PyResult<String> {
            let result = db::table(&name)
                .find(id)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))?;
            Ok(serde_json::to_string(&result).unwrap_or_default())
        }

        #[pyo3::pyfunction]
        fn table_find_all(name: String) -> PyResult<String> {
            let results = db::table(&name)
                .find_all()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))?;
            Ok(serde_json::to_string(&results).unwrap_or_default())
        }

        #[pyo3::pyfunction]
        fn table_update(name: String, id: i64, data: String) -> PyResult<usize> {
            let value: Value = serde_json::from_str(&data)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
            db::table(&name)
                .update(id, value)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
        }

        #[pyo3::pyfunction]
        fn table_delete(name: String, id: i64) -> PyResult<usize> {
            db::table(&name)
                .delete(id)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
        }

        db_module.add_function(wrap_pyfunction!(table_create, db_module)?)?;
        db_module.add_function(wrap_pyfunction!(table_insert, db_module)?)?;
        db_module.add_function(wrap_pyfunction!(table_find, db_module)?)?;
        db_module.add_function(wrap_pyfunction!(table_find_all, db_module)?)?;
        db_module.add_function(wrap_pyfunction!(table_update, db_module)?)?;
        db_module.add_function(wrap_pyfunction!(table_delete, db_module)?)?;

        py.import("sys")?
            .getattr("modules")?
            .set_item("db", db_module)?;

        Ok(())
    }

    pub fn call_endpoint(&self, path: &str, data: &Value) -> Result<Value, String> {
        Python::with_gil(|py| {
            // Parse path to module and function
            // Path format: /api/module/id/action or /api/module/action
            eprintln!("[DEBUG] =========================================");
            eprintln!("[DEBUG] Python call_endpoint with path: '{}'", path);
            let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
            eprintln!("[DEBUG] Parts after split: {:?}", parts);
            eprintln!("[DEBUG] Parts length: {}", parts.len());

            if parts.len() < 2 {
                return Err(
                    "Invalid endpoint path - expected /api/module or /api/module/action"
                        .to_string(),
                );
            }

            // Use first 2 parts for module path (api/todos -> endpoints.api.todos)
            let module_path = format!("endpoints.{}.{}", parts[0], parts[1]);
            eprintln!("[DEBUG] Module path: '{}'", module_path);
            eprintln!("[DEBUG] Attempting to import module: '{}'", module_path);

            // Extract ID and action from remaining parts
            // /api/todos/1/delete -> id=1, action=delete
            // /api/todos/create -> id="", action=create
            let (id, action) = if parts.len() > 2 {
                // Check if parts[2] is numeric (an ID)
                if parts[2].parse::<i64>().is_ok() {
                    // ID exists, action is parts[3] if present
                    let id_val = parts[2];
                    let action_val = if parts.len() > 3 { parts[3] } else { "" };
                    (id_val, action_val)
                } else {
                    // No ID, parts[2] is the action
                    ("", parts[2])
                }
            } else {
                ("", "")
            };

            let function_name = "handler";

            match self.call_python_function(py, &module_path, function_name, id, action, data) {
                Ok(result) => Ok(result),
                Err(e) => Err(format!(
                    "Python error: {} (module_path: {})",
                    e, module_path
                )),
            }
        })
    }

    fn call_python_function(
        &self,
        py: Python,
        module_path: &str,
        func_name: &str,
        id: &str,
        action: &str,
        data: &Value,
    ) -> PyResult<Value> {
        eprintln!("Attempting to import module: {}", module_path);
        let module = PyModule::import(py, module_path)?;
        let func = module.getattr(func_name)?;

        let req_dict = PyDict::new(py);
        req_dict.set_item("id", id)?;
        req_dict.set_item("action", action)?;

        // Convert JSON Value to Python dict
        let data_str = data.to_string();
        let json_module = py.import("json")?;
        let py_data = json_module.call_method1("loads", (data_str,))?;
        req_dict.set_item("data", py_data)?;

        let result = func.call1((req_dict,))?;
        let result_str = result.str()?.to_string();

        Ok(serde_json::from_str(&result_str).unwrap_or_else(|_| serde_json::json!(result_str)))
    }
}
