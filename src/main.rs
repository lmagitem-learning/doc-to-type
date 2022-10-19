#![warn(clippy::all, clippy::pedantic)]
use convert_case::{Case, Casing};
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

    let bean = get_bean(path);

    let bean_name = get_bean_name(&bean);

    let interface = get_interface_declaration(&bean, &bean_name);

    let properties = get_properties(&bean);

    to_write = format!("{}{}}}", interface, properties);
    write_file(bean_name, to_write);
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

        properties.push_str(&format!("{}  {}?: {};\n", documentation, name, property_type));
    }

    properties
}

fn get_interface_declaration(bean: &Value, bean_name: &String) -> String {
    let interface: String;

    let description = clean_name(&extract_string(bean, "description"));
    let documentation = format!("/** {} */\n", description);

    let bean_parents = extract_field(bean, "superTypeNames");
    let bean_parent: Option<&Value> = bean_parents.as_array().unwrap().into_iter().last();
    if bean_parent.is_some() {
        let bean_parent_name = clean_name(get_simple_name(&bean_parent.unwrap().to_string()));
        println!("{}", bean_parent_name);
        let import = format!(
            "import {{ {} }} from './{}.ts';\n\n",
            bean_parent_name,
            bean_parent_name.to_case(Case::Kebab)
        );
        interface = (*format!(
            "{}{}export interface {} extends {}\n{{\n",
            import, documentation, bean_name, bean_parent_name
        ))
        .to_string();
    } else {
        interface = (*format!("{}export interface {}\n{{\n", documentation, bean_name)).to_string();
    }
    interface
}

fn get_bean_name(bean: &Value) -> String {
    let name = &extract_string(bean, "name");
    let simple_name = clean_name(get_simple_name(name));
    println!("{}", simple_name);
    simple_name
}

fn write_file(name: String, to_write: String) {
    let mut w = File::create(format!("output/{}.ts", name.to_case(Case::Kebab))).unwrap();
    writeln!(&mut w, "{}", to_write).unwrap();
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

fn clean_name(name: &str) -> String {
    name.replace("\"", "")
}

fn get_simple_name(name: &String) -> &str {
    name.split(".")
        .last()
        .expect("Could not find a simple name")
}

fn extract_string(bean: &Value, field: &str) -> String {
    extract_field(bean, field).to_string()
}

fn extract_field(bean: &Value, field: &str) -> Value {
    bean[field].to_owned()
}
