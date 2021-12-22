use cli::{CliConfig, Cli};
use translator::yaml;
use std::fs;
use yaml_rust::YamlLoader;

fn main() {
    let file = fs::read_to_string("translator/translations.yml").expect("No such file.");
    
    let yaml = YamlLoader::load_from_str(&file).unwrap();
    let cmd_tree = yaml::to_tree(&yaml[0]);
    

    let config = CliConfig::new(
        "$: ", 
        cmd_tree,
    ).expect("Invalid configuration");

    let mut cli = Cli::open(config);

    cli.run();

    println!("Thanks for coming :)");
}
