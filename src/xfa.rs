use roxmltree::{Document, Node};
use serde_json::{Map, Value};


/// Convert XFA XML string to structured JSON string.
/// 
/// If `data_only` is true, metadata fields and large lookup lists are excluded.
pub fn xfa_xml_to_json(xml: &str, data_only: bool) -> Result<String, String> {
    let doc = Document::parse(xml).map_err(|e| format!("XML parse error: {}", e))?;
    
    let data_node = find_data_section(&doc)
        .ok_or_else(|| "Could not locate form data section in XFA XML".to_string())?;
        
    let mut form_data = Map::new();
    
    // Iterate over children of the data section
    for child in data_node.children() {
        if !child.is_element() {
            continue;
        }
        
        let tag_name = child.tag_name().name();
        
        if data_only && is_metadata_field(tag_name) {
            continue;
        }
        
        if let Some(json_val) = element_to_json(child, data_only) {
            // Check for top-level lookup lists if requested
            if data_only && is_lookup_list(tag_name, &json_val) {
                continue;
            }
            
            // For data_only mode, filter children of the "Form" element.
            if data_only && tag_name == "Form" {
                if let Value::Object(ref map) = json_val {
                    let mut filtered_map = Map::new();
                    for (k, v) in map {
                        if !is_lookup_list(k, v) {
                            filtered_map.insert(k.clone(), v.clone());
                        }
                    }
                    if !filtered_map.is_empty() {
                         merge_into_map(&mut form_data, tag_name, Value::Object(filtered_map));
                    }
                } else {
                     merge_into_map(&mut form_data, tag_name, json_val);
                }
            } else {
                merge_into_map(&mut form_data, tag_name, json_val);
            }
        }
    }
    
    if form_data.is_empty() {
        return Err("No valid data found after extraction".to_string());
    }
    
    serde_json::to_string_pretty(&Value::Object(form_data))
        .map_err(|e| format!("JSON serialization error: {}", e))
}

/// Helper to merge a key-value into a JSON map, handling duplicate keys by creating arrays.
fn merge_into_map(map: &mut Map<String, Value>, key: &str, value: Value) {
    if let Some(existing) = map.get_mut(key) {
        if let Value::Array(arr) = existing {
            arr.push(value);
        } else {
            let old_val = existing.take();
            *existing = Value::Array(vec![old_val, value]);
        }
    } else {
        map.insert(key.to_string(), value);
    }
}

fn find_data_section<'a>(doc: &'a Document) -> Option<Node<'a, 'a>> {
    const XFA_DATA_NS: &str = "http://www.xfa.org/schema/xfa-data/1.0/";
    
    // Look for xfa:data or datasets/data specific paths.
    // roxmltree doesn't support XPath, so manual traversal is required.
    
    for node in doc.descendants() {
        if !node.is_element() { continue; }
        
        let tag = node.tag_name();
        let name = tag.name();
        let ns = tag.namespace().unwrap_or("");
        
        // Match 1: Namespace match
        if name == "data" && ns == XFA_DATA_NS {
             return Some(node);
        }
        
        // Match 2: specific path "xfa:datasets/xfa:data" (approximate check)
        if name == "data" {
            if let Some(parent) = node.parent() {
                if parent.tag_name().name() == "datasets" {
                    return Some(node);
                }
            }
        }
    }
    
    // Fallback: Find any element named "data" as a last resort.
    doc.descendants().find(|n| n.is_element() && n.tag_name().name() == "data")
}

fn element_to_json(node: Node, data_only: bool) -> Option<Value> {
    let tag_name = node.tag_name().name();
    
    // Skip system elements
    if ["schema", "datamodel", "dataDescription"].contains(&tag_name) {
        return None;
    }

    let mut map = Map::new();
    
    // 1. Attributes
    if node.attributes().len() > 0 {
        let mut attr_map = Map::new();
        for attr in node.attributes() {
            let name = attr.name();
            // Skip namespace definitions handled by parser usually, 
            // but roxmltree keeps them if they are attributes? 
            // roxmltree separates namespaces. `attributes()` returns regular attributes.
            if !name.starts_with("xmlns") { // specific check just in case
                attr_map.insert(name.to_string(), Value::String(attr.value().to_string()));
            }
        }
        if !attr_map.is_empty() {
            map.insert("_attributes".to_string(), Value::Object(attr_map));
        }
    }
    
    // 2. Text value
    if let Some(text) = node.text() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            map.insert("_value".to_string(), Value::String(trimmed.to_string()));
        }
    }
    
    // 3. Children
    let mut has_children = false;
    for child in node.children() {
        if !child.is_element() { continue; }
        has_children = true;
        
        let child_name = child.tag_name().name();
        
        // recursive call
        if let Some(child_val) = element_to_json(child, data_only) {
             merge_into_map(&mut map, child_name, child_val);
        }
    }
    
    // Simplification logic:
    // If the node contains only a value (no children or attributes), return the value directly.
    
    if !has_children && map.len() == 1 && map.contains_key("_value") {
        // Safe unwrap: we just checked that the key exists.
        return Some(map.remove("_value").unwrap());
    }
    
    if map.is_empty() {
        return None;
    }
    
    Some(Value::Object(map))
}

fn is_metadata_field(name: &str) -> bool {
    let prefixes = [
        "FS", "fs", "_", "TEMPLATE", "QUERY", "TRANSFORMATION", 
        "template", "config", "xdp"
    ];
    prefixes.iter().any(|&p| name.starts_with(p))
}

fn is_lookup_list(name: &str, value: &Value) -> bool {
    // Patterns
    let patterns = [
        "List", "Options", "Choices", "Lookup", "Reference",
        "Country", "Port", "State", "City", "Dropdown"
    ];
    
    let name_match = patterns.iter().any(|&p| name.contains(p));
    if !name_match {
        return false;
    }
    
    // Check if the value is an object containing an array with more than 10 items.
    
    if let Value::Object(map) = value {
        for v in map.values() {
            if let Value::Array(arr) = v {
                if arr.len() > 10 {
                    return true;
                }
            }
        }
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_structure() {
        let xml = r#"<data><name>John</name><age>30</age></data>"#;
        let json_str = xfa_xml_to_json(xml, false).unwrap();
        let v: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(v["name"], "John");
        assert_eq!(v["age"], "30");
    }

    #[test]
    fn test_attributes_and_value() {
        let xml = r#"<data><field id="1">Value</field></data>"#;
        let json_str = xfa_xml_to_json(xml, false).unwrap();
        let v: Value = serde_json::from_str(&json_str).unwrap();
        // Since it has attributes, it should be an object with _value and _attributes
        assert_eq!(v["field"]["_value"], "Value");
        assert_eq!(v["field"]["_attributes"]["id"], "1");
    }

    #[test]
    fn test_metadata_filtering() {
        let xml = r#"<data><_sys>Hidden</_sys><visible>Shown</visible></data>"#;
        let json_str = xfa_xml_to_json(xml, true).unwrap();
        let v: Value = serde_json::from_str(&json_str).unwrap();
        assert!(v.get("_sys").is_none());
        assert_eq!(v["visible"], "Shown");
    }
    
    #[test]
    fn test_lookup_list_detection() {
        // Construct a list with 11 items
        let mut list_items = String::new();
        for i in 0..11 {
            list_items.push_str(&format!("<item>{}</item>", i));
        }
        let xml = format!(r#"<data><MyDropdown><options>{}</options></MyDropdown></data>"#, list_items);
        
        let json_str = xfa_xml_to_json(&xml, true).unwrap();
        let v: Value = serde_json::from_str(&json_str).unwrap();
        
        // Test a simpler structure where the list is direct children.
        let xml2 = format!(r#"<data><MyList>{}</MyList></data>"#, list_items);
         
        // With data_only=true, it should be skipped and result in empty data error.
        let result = xfa_xml_to_json(&xml2, true);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), "No valid data found after extraction");
        
        // Let's add a valid field
        let xml3 = format!(r#"<data><MyList>{}</MyList><real>Data</real></data>"#, list_items);
        let json_str3 = xfa_xml_to_json(&xml3, true).unwrap();
        let v3: Value = serde_json::from_str(&json_str3).unwrap();
        
        assert!(v3.get("MyList").is_none());
        assert_eq!(v3["real"], "Data");
    }
}
