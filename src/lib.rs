use pyo3::prelude::*;
use std::collections::HashMap;

mod call_node;
mod reporter;

use call_node::{build_call_tree, parse_events};
use reporter::generate_html;

/// Process profiling events and generate an HTML call graph report.
#[pyfunction]
fn process_events(
    py: Python<'_>,
    events: Vec<HashMap<String, Py<PyAny>>>,
    api_name: &str,
    start_ns: u64,
    end_ns: u64,
) -> PyResult<String> {
    let raw_events = parse_events(py, &events)?;
    let root = build_call_tree(raw_events, api_name, start_ns, end_ns);
    let html = generate_html(&root, api_name);
    Ok(html)
}

#[pymodule]
fn _callprofiler(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(process_events, m)?)?;
    Ok(())
}
