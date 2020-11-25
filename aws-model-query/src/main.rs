
use std::env;
use aws_model_query::smithy_query::query_models;

fn main() -> Result<(), Box<dyn std::error::Error + 'static>>{
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Must specify directory to search");
        return Ok(())
    }
    let search_dir = args[1].clone();
    query_models(search_dir)
}
