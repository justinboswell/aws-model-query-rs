

pub mod smithy_query {

    use std::fs;
    use std::collections::HashMap;
    use std::rc::Rc;

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
    struct Operation {
        name: String,
        input: Option<Rc<Structure>>,
        output: Option<Rc<Structure>>,
    }

    #[derive(Debug)]
    struct Service {
        name: String,
        operations: HashMap<String, Rc<Operation>>,
        structures: HashMap<String, Rc<Structure>>,
    }

    impl Service {
        fn new() -> Service {
            Service {
                name: String::from("UNKNOWN"),
                operations: HashMap::new(),
                structures: HashMap::new()
            }
        }
    }

    pub fn query_models(search_path: String) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let model_paths = find_models(search_path);
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
                        service.structures.insert(String::from(name), Rc::new(Structure {
                            name: String::from(name),
                            members: members
                        }));
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
                        Some(input) => Some(service.structures[input].clone()),
                        None => None
                    },
                    output: match output {
                        Some(output) => Some(service.structures[output].clone()),
                        None => None
                    }
                };
                service.operations.insert(name.to_string(), Rc::new(operation));
            }

            let ops = service.operations.keys()
                .filter(|name| name.contains("#List"))
                .chain(
                    service.operations.keys().filter(|name| name.contains("#Describe"))
                );
            for op in ops.map(|op| &service.operations[op]) {
                println!("    Query Operation: {}", op.name);
                match &op.input {
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
}
