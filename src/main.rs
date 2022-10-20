#![warn(clippy::all, clippy::pedantic)]
use convert_case::{Case, Casing};
use itertools::Itertools;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;

use serde_json::Value;

fn main() {
    let paths = fs::read_dir("beans/").unwrap();
    for path in paths {
        convert(path.unwrap().path().display().to_string());
    }
}

fn convert(path: String) {
    let to_write: String;
    let bean_name: String;
    let imports: String;
    let declaration: String;
    let members: String;

    let bean = get_bean(path);
    bean_name = get_bean_name(&bean);

    let is_enum = extract_field(&bean, "isEnum").as_bool().unwrap_or(false);
    if is_enum {
        imports = String::new();

        declaration = get_enum_declaration(&bean, &bean_name);

        members = get_enum_values(&bean);
    } else {
        declaration = get_interface_declaration(&bean, &bean_name);

        members = get_properties(&bean);

        imports = get_imports(&bean, &members);
    }

    to_write = format!("{}{}{}}}", imports, declaration, members);
    write_file(bean_name, to_write);
}

fn get_imports(bean: &Value, properties: &String) -> String {
    let mut imports_to_process: Vec<String> = Vec::new();
    let mut imports = String::new();

    let bean_parents = extract_field(bean, "superTypeNames");
    bean_parents
        .as_array()
        .unwrap()
        .into_iter()
        .map(|p| clean_name(get_simple_name(&p.to_string())))
        .for_each(|p| {
            if !p.is_empty() {
                imports_to_process.push(p);
            }
        });

    let mut lines = properties.split("\n").collect();
    remove_first(&mut lines);

    lines
        .iter()
        .step_by(2)
        .map(|s| s.split(": ").last().unwrap().trim())
        .map(|s| remove_map(remove_array(s.to_string())))
        .map(|s| s.replace(";", ""))
        .filter(|s| s.chars().next().unwrap().is_uppercase())
        .for_each(|s| imports_to_process.push(s));
    imports_to_process.sort();
    imports_to_process.iter().unique().for_each(|t| {
        if !t.is_empty() {
            imports.push_str(&generate_import_line(&t));
        }
    });

    if !imports.is_empty() {
        imports.push_str("\n");
    }

    imports
}

fn get_enum_declaration(bean: &Value, bean_name: &String) -> String {
    let description = clean_name(&extract_string(bean, "description"));
    let documentation = format!("/** {} */\n", description);
    let declaration = (*format!("{}export enum {}\n{{\n", documentation, bean_name)).to_string();
    declaration
}

fn get_interface_declaration(bean: &Value, bean_name: &String) -> String {
    let declaration: String;

    let description = clean_name(&extract_string(bean, "description"));
    let documentation = format!("/** {} */\n", description);

    let bean_parents = extract_field(bean, "superTypeNames");
    let bean_parent_names: Vec<String> = bean_parents
        .as_array()
        .unwrap()
        .into_iter()
        .map(|p| clean_name(get_simple_name(&p.to_string())))
        .collect();

    if !bean_parent_names.is_empty() {
        let declaration_start = format!("{}export interface {} extends ", documentation, bean_name);
        let mut declaration_middle = String::new();

        for name in bean_parent_names {
            if declaration_middle.is_empty() {
                declaration_middle.push_str(&name);
            } else {
                declaration_middle.push_str(&format!(", {}", name));
            }
        }

        declaration = (*format!("{}{}\n{{\n", declaration_start, declaration_middle)).to_string();
    } else {
        declaration =
            (*format!("{}export interface {}\n{{\n", documentation, bean_name)).to_string();
    }
    declaration
}

fn get_enum_values(bean: &Value) -> String {
    let mut values: String = String::new();

    let values_option = extract_field(&bean, "constants");
    let empty_array = &&Vec::new();
    let values_array = values_option.as_array().unwrap_or(empty_array);

    let last_value = values_array.clone().into_iter().last().unwrap();
    let last_name = last_value.as_str().unwrap();

    for value in values_array.into_iter() {
        let clean_name = clean_name(value.as_str().unwrap());
        let name = clean_name.as_str();

        if last_name.eq(name) {
            values.push_str(&format!("  {} = \"{}\"\n", name, name));
        } else {
            values.push_str(&format!("  {} = \"{}\",\n", name, name));
        }
    }

    values
}

fn get_properties(bean: &Value) -> String {
    let mut properties: String = String::new();

    let properties_option = extract_field(&bean, "properties");
    let empty_array = &&Vec::new();
    let properties_array = properties_option.as_array().unwrap_or(empty_array);

    for property in properties_array.into_iter() {
        let name = clean_name(&extract_string(&property, "name"));
        let description = clean_name(&extract_string(property, "description"));
        let documentation = format!("  /** {} */\n", description);
        let property_field = extract_field(property, "type");
        let mut property_type = clean_name(&extract_string(&property_field, "name"));

        match &property_type[..] {
            "object" => property_type = get_object_type(&property_field),
            "array" => property_type = get_array_type(&property_field),
            _ => (),
        }
        match &property_type[..] {
            "Map" => property_type = get_map_type(&property_field),
            _ => (),
        }

        properties.push_str(&format!(
            "{}  {}?: {};\n",
            documentation, name, property_type
        ));
    }

    properties
}

fn generate_import_line(import_name: &String) -> String {
    let import = format!(
        "import {{ {} }} from './{}';\n",
        import_name,
        import_name.to_case(Case::Kebab)
    );
    import
}

fn write_file(name: String, to_write: String) {
    let mut w = File::create(format!("output/{}.ts", name.to_case(Case::Kebab))).unwrap();
    writeln!(&mut w, "{}", to_write).unwrap();
}

fn get_map_type(bean: &Value) -> String {
    let key_type = &extract_field(&bean, "keyType");
    let mut key_name = clean_name(&extract_string(&key_type, "name"));

    match &key_name[..] {
        "object" => key_name = get_object_type(&key_type),
        "array" => key_name = get_array_type(&key_type),
        _ => (),
    }

    let value_type = &extract_field(&bean, "valueType");
    let mut value_name = clean_name(&extract_string(&value_type, "name"));

    match &value_name[..] {
        "object" => value_name = get_object_type(&value_type),
        "array" => value_name = get_array_type(&value_type),
        _ => (),
    }

    let simple_name = format!("Map<{}, {}>", key_name, value_name);
    simple_name
}

fn get_object_type(bean: &Value) -> String {
    let name = &extract_string(&bean, "exactTypeName");
    let mut simple_name = clean_name(get_simple_name(name));

    match &simple_name[..] {
        "null" => simple_name = "any".to_owned(),
        _ => (),
    }

    simple_name
}

fn get_array_type(bean: &Value) -> String {
    let element_type = extract_field(bean, "elementType");
    let mut simple_name = get_object_type(&element_type);

    match &simple_name[..] {
        "object" => simple_name = get_object_type(&element_type),
        "List" => simple_name = get_array_type(&element_type),
        _ => (),
    }

    let array_type = format!("Array<{}>", simple_name);
    array_type
}

fn get_bean(path: String) -> Value {
    let mut file = File::open(path).expect("File not found");
    let mut data = String::new();
    file.read_to_string(&mut data)
        .expect("Error while reading file");
    let json = data.split("] = ").last().expect("Pattern not recognized");
    let bean: Value = serde_json::from_str(json).expect("JSON was not well-formatted");
    bean
}

fn get_bean_name(bean: &Value) -> String {
    let name = &extract_string(bean, "name");
    let simple_name = clean_name(get_simple_name(name));
    simple_name
}

fn get_simple_name(name: &String) -> &str {
    name.split(".")
        .last()
        .expect("Could not find a simple name")
        .trim()
}

fn clean_name(name: &str) -> String {
    name.replace("\"", "")
}

fn extract_string(bean: &Value, field: &str) -> String {
    extract_field(bean, field)
        .to_string()
        .replace("\\n", "\n    ")
}

fn extract_field(bean: &Value, field: &str) -> Value {
    bean[field].to_owned()
}

fn remove_map(s: String) -> String {
    if s.contains("Map") {
        return s
            .replace("Map<", "")
            .replace(">", "")
            .split(", ")
            .last()
            .unwrap()
            .to_string();
    } else {
        return s;
    }
}

fn remove_array(s: String) -> String {
    if s.contains("Array") {
        return s.replace("Array<", "").replace(">", "").to_string();
    } else {
        return s;
    }
}

fn remove_first<T>(vec: &mut Vec<T>) -> Option<T> {
    if vec.is_empty() {
        return None;
    }
    Some(vec.remove(0))
}
