
use std::env;
use std::fs;
use std::collections::HashMap;

//#[macro_use]
extern crate json;

use walkdir::WalkDir;

fn is_model(entry: &walkdir::DirEntry) -> bool {
    entry.file_name().eq("model.json")
}

fn find_models(path: String) -> Vec<String> {
    let mut models: Vec<String> = Vec::new();
    let walker = WalkDir::new(path);
    for entry in walker.into_iter().filter_entry(|e| is_model(e) || e.path().is_dir()) {
        let entry = entry.unwrap();
        if !entry.path().is_dir() {
            models.push(String::from(entry.into_path().to_str().unwrap()));
        }
    }

    models
}

#[derive(Debug)]
struct Structure {
    name: String,
    members: HashMap<String, String>
}

#[derive(Debug)]
struct Operation<'a> {
    name: String,
    input: Option<&'a Structure>,
    output: Option<&'a Structure>,
}

#[derive(Debug)]
struct Service<'a> {
    name: String,
    operations: HashMap<String, Operation<'a>>,
    structures: HashMap<String, Structure>,
}

impl<'a> Service<'_> {
    fn new() -> Service<'a> {
        Service {
            name: String::from("UNKNOWN"),
            operations: HashMap::new(),
            structures: HashMap::new()
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + 'static>>{
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Must specify directory to search");
        return Ok(())
    }
    let search_dir = args[1].clone();
    let model_paths = find_models(search_dir);
    for model_path in model_paths {
        println!("Reading {}", model_path);
        let contents = fs::read_to_string(model_path)?;
        let model = json::parse(&contents).unwrap();
        let shapes = &model["shapes"];
        let mut operations = Vec::new();
        let mut service = Service::new();
        for (name, shape) in shapes.entries() {
            let shape_type = shape["type"].as_str().unwrap();
            match shape_type {
                "service" =>  {
                    println!("  Scanning service {}", name);
                    service.name = String::from(name);
                },
                "structure" => {
                    let mut members = HashMap::new();
                    for (name, member) in shape["members"].entries() {
                        members.insert(String::from(name), String::from(member["target"].as_str().unwrap()));
                    }
                    service.structures.insert(String::from(name), Structure {
                        name: String::from(name),
                        members: members
                    });
                },
                // cache operations, resolve them once all structures are resolved
                "operation" => {
                    operations.push((name, shape["input"]["target"].as_str(), shape["output"]["target"].as_str()));
                },
                _ => ()
            }
        }
        // Resolve operation inputs/outputs
        for (name, input, output) in operations.into_iter() {
            let operation = Operation {
                name: String::from(name),
                input: match input {
                    Some(input) => Some(&service.structures[input]),
                    None => None
                },
                output: match output {
                    Some(output) => Some(&service.structures[output]),
                    None => None
                }
            };
            service.operations.insert(name.to_string(), operation);
        }

        let ops = service.operations.keys()
            .filter(|name| name.contains("#List"))
            .chain(
                service.operations.keys().filter(|name| name.contains("#Describe"))
            );
        for op in ops.map(|op| &service.operations[op]) {
            println!("    Query Operation: {}", op.name);
            match op.input {
                Some(input) => {
                    for name in input.members.keys() {
                        if name.to_ascii_lowercase().starts_with("tag") {
                            println!("      Supports tag filtering via {} field", name);
                        }
                    }
                }
                None => ()
            }
        }
    }
    Ok(())
}
