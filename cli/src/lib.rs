use std::io::{self, BufRead, Write};
use translator::{Tree, Node, NodeId, Depth};

#[derive(Debug)]
pub enum CliError<'a> {
    InvalidConfig(&'a str),
}

pub struct CliConfig<'a> {
    prompt: &'a str,
    valid_cmds: Tree<'a>,
}

pub struct Cli<'a> {
    config: CliConfig<'a>,
    current_prompt: String,
    current_root: NodeId,
    prev_root: Option<NodeId>,
}

#[derive(Debug, PartialEq)]
struct CliCmd<'a> {
    cmd: &'a str,
    depth: Depth,
}

impl<'a> PartialEq<Node<'a>> for CliCmd<'a> {
    fn eq(&self, other: &Node) -> bool {
        self.cmd == other.name && self.depth == other.depth
    }
}

impl<'a> CliConfig<'a> {
    pub fn new(prompt: &'a str, valid_cmds: Tree<'a>) -> Result<CliConfig<'a>, CliError<'a>> {
        if prompt.is_empty() {
            Err(CliError::InvalidConfig("Empty prompt not allowed"))
        } else {
            Ok(CliConfig {
                prompt,
                valid_cmds,
            })
        }
    }
}

impl<'a> Cli<'a> {
    pub fn open(config: CliConfig) -> Cli {
        let root = config.valid_cmds.root;
        Cli { 
            config: config, 
            current_prompt: String::new(), 
            current_root: root, 
            prev_root: None 
        }
    }
}

impl<'a, 'b> Cli<'a> {
    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut input = String::new();

        loop {
            print!("{}{}", self.current_prompt, self.config.prompt);
            io::stdout().flush().expect("Failed to flush");

            input.clear();
            match handle.read_line(&mut input) {
                Ok(n) => {
                    if Cli::should_exit(&input, n) {
                        break;
                    } else if Cli::should_new_prompt(&input) {
                        continue;
                    } else if Cli::should_change_root(&input) {
                        println!("Change root!");
                        let (new_root, new_prompt) = self.change_root(&input);
                        self.current_root = new_root;
                        self.current_prompt = new_prompt;
                    } else {
                        self.handle_input(&input);
                    }
                }
                Err(e) => {
                    println!("Got error: {}", e);
                    break;
                }
            }
        }
        println!("");
        io::stdout().flush().expect("Failed to flush stdout");
    }

    fn should_exit(input: &'a str, nbytes: usize) -> bool {
        nbytes == 0 || input == "exit\n" || input == "quit\n"
    }

    fn should_new_prompt(input: &'a str) -> bool {
        input == "\n"
    }

    fn should_change_root(input: &'a str) -> bool {
        if input.len() >= 2 {
            &input[..2] == "cd"
        } else {
            false
        }
    }

    fn change_root(&self, input: &'b str) -> (NodeId, String) {
        let mut new_root = self.config.valid_cmds.root;
        let mut construct_input = None;
        
        let mut input_stripped: String 
            = input.chars().filter(|c| !c.is_whitespace()).collect();

        if input_stripped == "cd" {
        } else if input_stripped == "cd -" {
            // Back to previous root if it exists
            if let Some(proot) = self.prev_root {
                new_root = proot;
                construct_input = Some(&new_root);
            }
        } else if input == "cd ..\n" {
                if let Some(parent) = self.current_root.ancestors(&self.config.valid_cmds.arena).next() {
                    new_root = parent;
                    construct_input = Some(&new_root);
                }
        } else if input.starts_with("cd /") {
            // Absolute path
            println!("{}", input);
        } else if input.starts_with("cd ") {
            // Relative path
            if input_stripped.ends_with('/') {
                input_stripped.pop();
            }
            let clicmds = Cli::construct_clicmds(&input_stripped[2..], '/');
            let (_, root) = self.build_subtree(&clicmds);
            new_root = root;

            construct_input = Some(&new_root);
        }
        
        (new_root, self.construct_prompt(construct_input))
    }

    fn construct_prompt(&self, root: Option<&NodeId>) -> String {
        let mut prompt = String::new();
        
        if let Some(root) = root {
            let arena = &self.config.valid_cmds.arena;
    
            let mut prompt_vec: Vec<String> = Vec::new();
            for node in root.ancestors(&self.config.valid_cmds.arena) {
                prompt_vec.push(
                    Node::from_id(&node, &arena).name.to_string()
                );
            }
    
            for s in prompt_vec.into_iter().rev() {
                if s != "root" {
                    prompt.push_str(&s);
                    prompt.push('/');
                }
            }
        }
        
        prompt
    }

    fn handle_input(&self, input: &'a str) {
        let clicmds = Cli::construct_clicmds(&input, ' ');
        let (sequence_tree, leaf) = self.build_subtree(&clicmds);
        println!("{:?}", sequence_tree);
        
        let sequence_tree_count = translator::subtree_count(
            &sequence_tree.root, 
            &sequence_tree.arena
        );
        let nodes_below_leaf = translator::subtree_count(
            &leaf, 
            &self.config.valid_cmds.arena
        );

        println!("seq count: {}", sequence_tree_count);
        println!("leaf below count: {}", nodes_below_leaf);

        if sequence_tree_count == clicmds.len() && nodes_below_leaf == 0 {
            println!("ACCEPTED");
        } else {
            println!("USAGE");
            self.print_usage(&leaf, &sequence_tree);
        }
    }

    fn construct_clicmds(input: &'a str, delim: char) -> Vec<CliCmd> {
        let mut clicmds = vec![];
        for (i, split) in input.split(delim).enumerate() {
            clicmds.push(
                CliCmd {
                    cmd: if split.ends_with('\n') {
                        &split[0..split.len() - 1]
                    } else {
                        split
                    },
                    depth: Depth::Some(i + 1)
                }
            );
        }

        clicmds
    }

    fn build_subtree(&self, clicmds: &Vec<CliCmd>) -> (Tree<'a>, NodeId) {
        /*
        At this point we may have a validation tree looking like this:

                        root
                  -------|-------
                 sat            gs
             -----|-----    -----|-----
            cmd        ft radio     config
         ----|-------
        obc  adcs  pay
         |
        ping

        Given an input like: "sat cmd obc ping", we want to reach the ping leaf
        from the root node. If the algorithm does not find its necessary node,
        it will end prematurely, which in turn outputs a shorter tree than expected.
        */
        
        let validation_tree = &self.config.valid_cmds;
        let mut root = self.current_root;
        let mut seq_tree = Tree::new();

        let up_clicmd = CliCmd { cmd: "..", depth: Depth::Any };

        'upper: for cmd in clicmds {
            if *cmd == up_clicmd {
                if let Some(node) = root.ancestors(&validation_tree.arena).next() {
                    let append = Node::from_id(&node, &validation_tree.arena);
                    seq_tree.root.append(
                        Node::from_node_to_id(append, &mut seq_tree.arena),
                        &mut seq_tree.arena
                    );

                    root = node;
                }
            }
            for child in root.children(&validation_tree.arena) {
                let node = Node::from_id(&child, &validation_tree.arena);
                println!("data: {:?}", node);
                println!("cmd: {:?}", cmd);
                if *cmd == node {

                    // Build up the sequence tree so we can return it later
                    seq_tree.root.append(
                        Node::from_node_to_id(node, &mut seq_tree.arena), 
                        &mut seq_tree.arena
                    );
                    
                    // Update root so next iterations begins from the subtree
                    root = child;
                    continue 'upper;
                }
            }

            // cmd did not match any node in the tree; end prematurely
            break;
        }

        // On success, root has become a leaf
        (seq_tree, root)
    }

    fn print_usage(&self, last_valid_node: &NodeId, sequence_tree: &Tree) {
        print!("Usage: ");
        
        for node in sequence_tree.root.descendants(&sequence_tree.arena).skip(1) {
            let node = Node::from_id(&node, &sequence_tree.arena);
            print!("{} ", node.name);
        }

        print!("<cmd>\nWhere 'cmd' can be either of\n");

        let validation_tree = &self.config.valid_cmds;
        for node in last_valid_node.children(&validation_tree.arena) {
            let node = Node::from_id(&node, &validation_tree.arena);
            print!("\t* {}", node.name);
            
            if let Some(exp) = node.explanation {
                print!(": {}", exp);
            }

            println!("");
        }    
    }
}

/* fn _check_timeout(_prev_timeout: Duration, timeout: Option<Duration>) -> bool {
    match timeout {
        Some(_t) => false,
        None => false,
    }
} */

#[cfg(test)]
mod tests {
    use super::*;
    use yaml_rust::YamlLoader;
    use translator::yaml;

    const YAMLDOC: &str =
    "
    sat:
    - obc
      - ping
      - set
    - adcs
      - ping
      - set
    - pay
      - ping
      - take_pic
    
    gs:
    - radio
      - ping
      - set_freq
    - sys
      - config
    ";

    fn get_cli<'a>(yaml: &'a yaml_rust::Yaml) -> Cli<'a> {
        let cmd_tree = yaml::to_tree(&yaml);
        
        let config = CliConfig::new(
            "$: ", 
            cmd_tree,
        ).expect("Invalid configuration");
    
        Cli::open(config)
    }

    mod cd {
        use super::*;

        #[test]
        fn should_change_root() {
            assert_eq!(true, Cli::should_change_root("cd"));
            assert_ne!(true, Cli::should_change_root("c"));
            assert_ne!(true, Cli::should_change_root("d"));
            assert_ne!(true, Cli::should_change_root(""));
            assert_eq!(true, Cli::should_change_root("cd -"));
            assert_eq!(true, Cli::should_change_root("cd ."));
            assert_eq!(true, Cli::should_change_root("cd .."));
            assert_eq!(true, Cli::should_change_root("cd sat"));
            assert_eq!(true, Cli::should_change_root("cd /sat"));
            assert_eq!(true, Cli::should_change_root("cd /.."));
            assert_eq!(true, Cli::should_change_root("cd \t"));
            assert_eq!(true, Cli::should_change_root("cd\n"));
        }

        #[test]
        fn change_root_normal() {
            let yaml = YamlLoader::load_from_str(&YAMLDOC).unwrap();
            let cli = get_cli(&yaml[0]);
            let arena = &cli.config.valid_cmds.arena;

            let (node, prompt) = cli.change_root("cd sat");
            assert_eq!(prompt, "sat");
            assert_eq!(
                Node::from_id(&node, &arena),
                Node::new("sat", "", Depth::Some(1))
            );

            let (node, prompt) = cli.change_root("cd -");
            assert_eq!(prompt, "");
            assert_eq!(
                Node::from_id(&node, &arena),
                Node::new("root", "", Depth::Some(0))
            );
        }
    }
}