use crate::call_node::CallNode;
use std::fmt::Write;

fn format_duration(ns: u64) -> String {
    if ns < 1_000 {
        format!("{}ns", ns)
    } else if ns < 1_000_000 {
        format!("{:.2}\u{00b5}s", ns as f64 / 1_000.0)
    } else if ns < 1_000_000_000 {
        format!("{:.2}ms", ns as f64 / 1_000_000.0)
    } else {
        format!("{:.3}s", ns as f64 / 1_000_000_000.0)
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn short_path(file_path: &str) -> &str {
    file_path
        .rsplit('/')
        .next()
        .or_else(|| file_path.rsplit('\\').next())
        .unwrap_or(file_path)
}

pub fn generate_html(root: &CallNode, api_name: &str) -> String {
    let slowest_id = root.find_slowest_id();

    let mut html = String::with_capacity(16384);

    // HTML head with embedded CSS
    write!(
        html,
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>callprofiler: {api_name}</title>
<style>
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{ font-family: 'Segoe UI', -apple-system, BlinkMacSystemFont, sans-serif; margin: 24px; background: #f8f9fa; color: #212529; }}
h1 {{ font-size: 1.5rem; color: #1a1a2e; margin-bottom: 16px; padding-bottom: 10px; border-bottom: 3px solid #4361ee; }}
.summary {{ background: #e9ecef; padding: 14px 20px; border-radius: 8px; margin-bottom: 20px; display: flex; gap: 32px; flex-wrap: wrap; font-size: 0.9rem; }}
.summary .item {{ display: flex; align-items: center; gap: 6px; }}
.summary .label {{ font-weight: 600; color: #495057; }}
.summary .value {{ color: #212529; }}
.summary .slowest-name {{ color: #e63946; font-weight: 700; }}
.tree {{ font-size: 0.88rem; }}
.tree ul {{ list-style: none; padding-left: 28px; border-left: 2px solid #dee2e6; margin: 0; }}
.tree > ul {{ border-left: none; padding-left: 0; }}
.tree li {{ position: relative; padding: 3px 0; }}
.node {{ display: inline-flex; align-items: center; gap: 8px; padding: 5px 12px; border-radius: 6px; border: 1px solid #dee2e6; background: #fff; cursor: default; transition: all 0.15s; flex-wrap: wrap; }}
.node:hover {{ box-shadow: 0 2px 8px rgba(0,0,0,0.08); }}
.node.external {{ background: #f1f3f5; border-color: #ced4da; }}
.node.external .func-name {{ color: #868e96; font-style: italic; }}
.node.slowest {{ background: #e63946; border-color: #c1121f; }}
.node.slowest .func-name {{ color: #fff; }}
.node.slowest .location,
.node.slowest .duration,
.node.slowest .times {{ color: rgba(255,255,255,0.85); }}
.func-name {{ font-weight: 600; color: #1a1a2e; font-family: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace; }}
.location {{ font-size: 0.82em; color: #6c757d; font-family: 'SF Mono', 'Fira Code', monospace; }}
.duration {{ font-size: 0.82em; color: #4361ee; font-weight: 600; }}
.times {{ font-size: 0.78em; color: #adb5bd; }}
.lib-badge {{ font-size: 0.72em; background: #dee2e6; color: #495057; padding: 1px 8px; border-radius: 10px; font-weight: 500; }}
.node.slowest .lib-badge {{ background: rgba(255,255,255,0.25); color: #fff; }}
.toggle {{ display: inline-block; width: 18px; font-size: 0.75em; text-align: center; cursor: pointer; user-select: none; color: #868e96; font-weight: bold; flex-shrink: 0; }}
.toggle:hover {{ color: #4361ee; }}
.hidden {{ display: none; }}
</style>
</head>
<body>
<h1>Call Profile: {api_name}</h1>
"#,
        api_name = html_escape(api_name)
    )
    .unwrap();

    // Summary bar
    write!(html, r#"<div class="summary">"#).unwrap();
    write!(
        html,
        r#"<div class="item"><span class="label">Total Duration:</span><span class="value">{}</span></div>"#,
        format_duration(root.duration_ns)
    )
    .unwrap();

    if let Some(sid) = slowest_id {
        if let Some(node) = find_node_by_id(root, sid) {
            write!(
                html,
                r#"<div class="item"><span class="label">Slowest Function:</span><span class="slowest-name">{} ({})</span></div>"#,
                html_escape(&node.func_name),
                format_duration(node.duration_ns)
            )
            .unwrap();
        }
    }

    let child_count = count_nodes(root) - 1;
    write!(
        html,
        r#"<div class="item"><span class="label">Functions:</span><span class="value">{}</span></div>"#,
        child_count
    )
    .unwrap();
    write!(html, "</div>\n").unwrap();

    // Tree
    let mut counter: usize = 0;
    write!(html, r#"<div class="tree"><ul>"#).unwrap();
    render_node(&mut html, root, &slowest_id, &mut counter);
    write!(html, "</ul></div>\n").unwrap();

    // JavaScript for toggle
    write!(
        html,
        r#"<script>
function toggle(el) {{
    var li = el.closest('li');
    var ul = li.querySelector(':scope > ul');
    if (!ul) return;
    if (ul.classList.contains('hidden')) {{
        ul.classList.remove('hidden');
        el.textContent = '\u25BC';
    }} else {{
        ul.classList.add('hidden');
        el.textContent = '\u25B6';
    }}
}}
</script>
</body>
</html>"#
    )
    .unwrap();

    html
}

fn find_node_by_id(node: &CallNode, target_id: usize) -> Option<&CallNode> {
    let mut counter: usize = 0;
    find_node_recursive(node, target_id, &mut counter)
}

fn find_node_recursive<'a>(
    node: &'a CallNode,
    target_id: usize,
    counter: &mut usize,
) -> Option<&'a CallNode> {
    let my_id = *counter;
    *counter += 1;

    if my_id == target_id {
        return Some(node);
    }

    for child in &node.children {
        if let Some(found) = find_node_recursive(child, target_id, counter) {
            return Some(found);
        }
    }
    None
}

fn count_nodes(node: &CallNode) -> usize {
    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

fn render_node(
    html: &mut String,
    node: &CallNode,
    slowest_id: &Option<usize>,
    counter: &mut usize,
) {
    let my_id = *counter;
    *counter += 1;

    let is_slowest = slowest_id.map_or(false, |id| id == my_id);
    let has_children = !node.children.is_empty();

    write!(html, "<li>").unwrap();

    // Build CSS classes
    let mut classes = String::from("node");
    if node.is_external {
        classes.push_str(" external");
    }
    if is_slowest {
        classes.push_str(" slowest");
    }

    write!(html, r#"<div class="{classes}">"#).unwrap();

    // Toggle button
    if has_children {
        write!(
            html,
            r#"<span class="toggle" onclick="toggle(this)">{}</span>"#,
            "\u{25BC}"
        )
        .unwrap();
    }

    // Function name
    write!(
        html,
        r#"<span class="func-name">{}</span>"#,
        html_escape(&node.func_name)
    )
    .unwrap();

    // File location
    if !node.file_path.is_empty() {
        write!(
            html,
            r#"<span class="location">{}:{}</span>"#,
            html_escape(short_path(&node.file_path)),
            node.line_number
        )
        .unwrap();
    }

    // Duration
    write!(
        html,
        r#"<span class="duration">{}</span>"#,
        format_duration(node.duration_ns)
    )
    .unwrap();

    // Start/end times (relative to profiling start)
    write!(
        html,
        r#"<span class="times">[start: {} | end: {}]</span>"#,
        format_duration(node.start_time_ns),
        format_duration(node.end_time_ns)
    )
    .unwrap();

    // Library badge for external functions
    if node.is_external && !node.library_name.is_empty() {
        write!(
            html,
            r#"<span class="lib-badge">{}</span>"#,
            html_escape(&node.library_name)
        )
        .unwrap();
    }

    write!(html, "</div>").unwrap();

    // Render children
    if has_children {
        write!(html, "<ul>").unwrap();
        for child in &node.children {
            render_node(html, child, slowest_id, counter);
        }
        write!(html, "</ul>").unwrap();
    }

    write!(html, "</li>\n").unwrap();
}
