use pyo3::prelude::*;
use std::collections::HashMap;

pub struct RawEvent {
    pub event: String,
    pub func_name: String,
    pub module: String,
    pub filename: String,
    pub lineno: u32,
    pub timestamp_ns: u64,
    pub is_external: bool,
    pub library_name: String,
}

#[allow(dead_code)]
pub struct CallNode {
    pub func_name: String,
    pub module_name: String,
    pub file_path: String,
    pub line_number: u32,
    pub start_time_ns: u64,
    pub end_time_ns: u64,
    pub duration_ns: u64,
    pub is_external: bool,
    pub library_name: String,
    pub children: Vec<CallNode>,
}

impl CallNode {
    fn new(
        func_name: String,
        module_name: String,
        file_path: String,
        line_number: u32,
        start_time_ns: u64,
        is_external: bool,
        library_name: String,
    ) -> Self {
        CallNode {
            func_name,
            module_name,
            file_path,
            line_number,
            start_time_ns,
            end_time_ns: 0,
            duration_ns: 0,
            is_external,
            library_name,
            children: Vec::new(),
        }
    }

    fn finalize(&mut self, end_time_ns: u64) {
        self.end_time_ns = end_time_ns;
        self.duration_ns = end_time_ns.saturating_sub(self.start_time_ns);
    }

    /// Find the slowest internal (non-root, non-external) node by duration.
    /// Returns the index path or a unique key for identification.
    pub fn find_slowest_id(&self) -> Option<usize> {
        let mut slowest_id: Option<usize> = None;
        let mut slowest_dur: u64 = 0;
        let mut counter: usize = 0;
        self.find_slowest_recursive(&mut slowest_id, &mut slowest_dur, &mut counter, true);
        slowest_id
    }

    fn find_slowest_recursive(
        &self,
        slowest_id: &mut Option<usize>,
        slowest_dur: &mut u64,
        counter: &mut usize,
    is_root: bool,
    ) {
        let my_id = *counter;
        *counter += 1;

        if !is_root && !self.is_external && self.duration_ns > *slowest_dur {
            *slowest_dur = self.duration_ns;
            *slowest_id = Some(my_id);
        }

        for child in &self.children {
            child.find_slowest_recursive(slowest_id, slowest_dur, counter, false);
        }
    }

    /// Normalize all timestamps relative to a base timestamp.
    pub fn normalize_times(&mut self, base_ns: u64) {
        self.start_time_ns = self.start_time_ns.saturating_sub(base_ns);
        self.end_time_ns = self.end_time_ns.saturating_sub(base_ns);
        for child in &mut self.children {
            child.normalize_times(base_ns);
        }
    }
}

fn extract_string(py: Python<'_>, map: &HashMap<String, Py<PyAny>>, key: &str) -> PyResult<String> {
    map.get(key)
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err(key.to_string()))?
        .extract::<String>(py)
}

fn extract_u32(py: Python<'_>, map: &HashMap<String, Py<PyAny>>, key: &str) -> PyResult<u32> {
    map.get(key)
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err(key.to_string()))?
        .extract::<u32>(py)
}

fn extract_u64(py: Python<'_>, map: &HashMap<String, Py<PyAny>>, key: &str) -> PyResult<u64> {
    map.get(key)
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err(key.to_string()))?
        .extract::<u64>(py)
}

fn extract_bool(py: Python<'_>, map: &HashMap<String, Py<PyAny>>, key: &str) -> PyResult<bool> {
    map.get(key)
        .ok_or_else(|| pyo3::exceptions::PyKeyError::new_err(key.to_string()))?
        .extract::<bool>(py)
}

pub fn parse_events(
    py: Python<'_>,
    events: &[HashMap<String, Py<PyAny>>],
) -> PyResult<Vec<RawEvent>> {
    let mut result = Vec::with_capacity(events.len());
    for ev in events {
        result.push(RawEvent {
            event: extract_string(py, ev, "event")?,
            func_name: extract_string(py, ev, "func_name")?,
            module: extract_string(py, ev, "module")?,
            filename: extract_string(py, ev, "filename")?,
            lineno: extract_u32(py, ev, "lineno")?,
            timestamp_ns: extract_u64(py, ev, "timestamp_ns")?,
            is_external: extract_bool(py, ev, "is_external")?,
            library_name: extract_string(py, ev, "library_name")?,
        });
    }
    Ok(result)
}

pub fn build_call_tree(
    events: Vec<RawEvent>,
    api_name: &str,
    start_ns: u64,
    end_ns: u64,
) -> CallNode {
    let mut stack: Vec<CallNode> = Vec::new();

    for ev in events {
        match ev.event.as_str() {
            "call" | "c_call" => {
                let node = CallNode::new(
                    ev.func_name,
                    ev.module,
                    ev.filename,
                    ev.lineno,
                    ev.timestamp_ns,
                    ev.is_external,
                    ev.library_name,
                );
                stack.push(node);
            }
            "return" | "c_return" => {
                if stack.len() > 1 {
                    let mut finished = stack.pop().unwrap();
                    finished.finalize(ev.timestamp_ns);

                    // External nodes become leaf nodes: remove their children
                    if finished.is_external {
                        finished.children.clear();
                    }

                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(finished);
                    }
                } else if stack.len() == 1 {
                    // Root node returning
                    stack.last_mut().unwrap().finalize(ev.timestamp_ns);
                }
            }
            _ => {}
        }
    }

    // Finalize any remaining unmatched nodes
    while stack.len() > 1 {
        let mut finished = stack.pop().unwrap();
        finished.finalize(end_ns);
        if finished.is_external {
            finished.children.clear();
        }
        if let Some(parent) = stack.last_mut() {
            parent.children.push(finished);
        }
    }

    let mut root = stack.pop().unwrap_or_else(|| {
        // Fallback: no events captured
        CallNode::new(
            api_name.to_string(),
            String::new(),
            String::new(),
            0,
            start_ns,
            false,
            String::new(),
        )
    });

    if root.end_time_ns == 0 {
        root.finalize(end_ns);
    }
    root.normalize_times(start_ns);
    root
}
