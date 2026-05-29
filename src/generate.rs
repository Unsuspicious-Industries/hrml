use std::collections::BTreeMap;

fn write_attrs(tag: &mut String, attrs: &BTreeMap<String, String>) {
    let mut keys: Vec<&String> = attrs.keys().collect();
    keys.sort();
    for k in keys {
        tag.push(' ');
        tag.push_str(k);
        tag.push_str("=\"");
        tag.push_str(&attrs[k]);
        tag.push('"');
    }
}

pub fn element(name: &str, attrs: &BTreeMap<String, String>, body: &str) -> String {
    let mut tag = String::from("<");
    tag.push_str(name);
    write_attrs(&mut tag, attrs);
    tag.push('>');
    tag.push_str(body);
    tag.push_str("</");
    tag.push_str(name);
    tag.push('>');
    tag
}

pub fn void(name: &str, attrs: &BTreeMap<String, String>) -> String {
    let mut tag = String::from("<");
    tag.push_str(name);
    write_attrs(&mut tag, attrs);
    tag.push('>');
    tag
}

pub fn self_closed(name: &str, attrs: &BTreeMap<String, String>) -> String {
    let mut tag = String::from("<");
    tag.push_str(name);
    write_attrs(&mut tag, attrs);
    tag.push_str(" />");
    tag
}
