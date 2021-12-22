pub use indextree::NodeId;
use std::fmt::{Debug, Formatter, Result};

pub type NodeArena<'a> = indextree::Arena<Node<'a>>;

pub struct Tree<'a> {
    pub root: NodeId,
    pub arena: NodeArena<'a>,
}

impl<'a> Tree<'a> {
    pub fn new() -> Tree<'a> {
        Tree::new_depth(Depth::Some(0))
    }

    fn new_depth(depth: Depth) -> Tree<'a> {
        let mut arena = NodeArena::new();
        let root = arena.new_node(Node::new("root", "", depth));
        Tree { root, arena }
    }
}

pub fn subtree_count(node: &NodeId, arena: &NodeArena) -> usize {
    node.descendants(arena).into_iter().count() - 1
}

impl<'a> Debug for Tree<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let root = self.root;

        for node in root.descendants(&self.arena) {
            let data = Node::from_id(&node, &self.arena);
            
            if let Depth::Some(d) = data.depth {
                write!(f, "{}>", "\t".repeat(d))?;
            }
            write!(f, "{}", data.name)?;
            
            if let Some(exp) = data.explanation {
                write!(f, ": {}", exp)?;
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

/* struct TreeIntoIterator {
    tree: Tree,
    index: usize,
}

impl<'a> Iterator for TreeIntoIterator<'a> {
    type Item = Node<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        
    }
}

impl<'a> IntoIterator for NodeArena<'a> {
    type Item = NodeId;
    type IntoIter = TreeIntoIterator;

    fn into_iter() -> Self::IntoIter {

    }
} */

#[derive(Debug, Copy, Clone)]
pub enum Depth {
    Any,
    Some(usize),
}

impl PartialEq for Depth {
    fn eq(&self, other: &Self) -> bool {
        if let (Depth::Some(d1), Depth::Some(d2)) = (self, other) {
            d1 == d2
        } else {
            true
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Node<'a> {
    pub name: &'a str,
    pub explanation: Option<&'a str>,
    pub depth: Depth,
}

impl<'a> PartialEq for Node<'a> {
    fn eq(&self, other: &Self) -> bool {
        (self.name == other.name) && 
        (self.explanation == other.explanation) &&
        (self.depth == other.depth)
    }
}

impl<'a> Node<'a> {
    pub fn new(name: &'a str, explanation: &'a str, depth: Depth) -> Node<'a> {
        Node {
            name,
            explanation: 
                if explanation.is_empty() { None } else { Some(explanation) },
            depth
        }
    }

    pub fn from_data_to_id(name: &'a str, explanation: &'a str, depth: Depth, arena: &mut NodeArena<'a>) -> NodeId {
        Node::from_node_to_id(
            Node::new(name, explanation, depth),
            arena
        )
    }

    pub fn from_node_to_id(node: Node<'a>, arena: &mut NodeArena<'a>) -> NodeId {
        arena.new_node(node)
    }

    pub fn from_id(nid: &NodeId, arena: &NodeArena<'a>) -> Node<'a> {
        *arena.get(*nid).unwrap().get()
    }
}

pub mod yaml {
    use yaml_rust::{Yaml, yaml::Hash};
    use super::{Node, NodeId, NodeArena, Tree, Depth};
    
    pub fn to_tree<'a>(yaml: &'a Yaml) -> Tree {
        let mut tree = Tree::new();
        
        match yaml.as_hash() {
            Some(h) => {
                tree.root = to_tree_rec(tree.root, &mut tree.arena, &h);
            }
            None => {}
        }
    
        tree
    }

    fn to_tree_rec<'a>(root: NodeId, arena: &mut NodeArena<'a>, hash: &'a Hash) -> NodeId {
        for (key, val) in hash.iter() {
            if let Yaml::String(s) = key {
                let root_depth = Node::from_id(&root, &arena).depth;
                let node = Node::from_data_to_id(
                    s, 
                    get_exp(val), 
                    if let Depth::Some(d) = root_depth { Depth::Some(d + 1) } else { Depth::Any }, 
                    arena
                );
                root.append(node, arena);

                if let Yaml::Array(vec) = val {
                    for elem in vec {
                        if let Yaml::Hash(h) = elem {
                            let subroot = to_tree_rec(node, arena, h);
                            root.append(subroot, arena);
                        } else if let Yaml::String(s) = elem {
                            let new_node = Node::from_data_to_id(
                                s,
                                "",
                                if let Depth::Some(d) = root_depth { Depth::Some(d + 1) } else { Depth::Any }, 
                                arena
                            );
                            node.append(new_node, arena);
                        }
                    }
                }
            }
        }
        root
    }

    fn get_exp(yaml: &Yaml) -> &str {
        if let Yaml::String(exp) = yaml {
            exp
        } else {
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod nodes {
        use super::*;

        #[test]
        fn same_empty() {
            let node1 = Node::new("", "", 0);
            let node2 = Node::new("", "", 0);

            assert_eq!(node1, node2);
        }

        #[test]
        fn same() {
            let node1 = Node::new("node1", "exp", 1);
            let node2 = Node::new("node1", "exp", 1);

            assert_eq!(node1, node2)
        }

        #[test]
        fn different_names() {
            let node1 = Node::new("node1", "exp1", 1);
            let node2 = Node::new("", "exp1", 1);
            
            assert_ne!(node1, node2);
        }

        #[test]
        fn differnet_exps() {
            let node1 = Node::new("node1", "exp1", 1);
            let node2 = Node::new("node1", "", 1);
            
            assert_ne!(node1, node2);
        }

        #[test]
        fn different_depths() {
            let node1 = Node::new("node1", "exp1", 0);
            let node2 = Node::new("node1", "exp1", 1);
            
            assert_ne!(node1, node2);
        }

    }

    mod tree {
        use super::*;

        impl<'a> PartialEq for Tree<'a> {
            fn eq(&self, other: &Self) -> bool {
                for (nid1, nid2) in self.root.descendants(&self.arena).zip(other.root.descendants(&other.arena)) {
                    let (n1, n2) = (
                        Node::from_id(&nid1, &self.arena),
                        Node::from_id(&nid2, &other.arena)
                    );

                    if n1 != n2 {
                        return false;
                    }
                }
                true
            }
        }

        fn generate_tree<'a>(nodes: Vec<(&'a str, &'a str, usize)>) -> Tree<'a> {
            let mut tree = Tree::new();
            for node in nodes {
                let new = Node::from_data_to_id(node.0, node.1, node.2, &mut tree.arena);
                tree.root.append(
                    new,
                    &mut tree.arena
                );
            }
            tree
        }
    
        #[test]
        fn new() {
            let tree = Tree::new();
            let tree2 = Tree::new();
            assert_eq!(
                tree,
                tree2
            );
        }
    
        #[test]
        fn new_depth() {
            let tree = Tree::new_depth(5);
            let tree2 = Tree::new_depth(5);
            assert_eq!(
                tree,
                tree2
            );
        }
    
        #[test]
        fn same() {
            let nodes = vec!(
                ("node1", "node1exp", 0),
                ("node2", "node2exp", 0),
                ("node2", "node2exp", 0),
                ("", "", 0),
                ("dep", "", 55),
                ("dep2", "", 1),
            );
    
            let tree1 = generate_tree(nodes.clone());
            let tree2 = generate_tree(nodes);
    
            assert_eq!(tree1, tree2);
        }
    
        #[test]
        fn different_names() {
            let tree1 = generate_tree(vec![("name1", "", 0)]);
            let tree2 = generate_tree(vec![("name2", "", 0)]);
    
            assert_ne!(tree1, tree2);
        }

        #[test]
        fn different_exps() {
            let tree1 = generate_tree(vec![("node", "exp1", 0)]);
            let tree2 = generate_tree(vec![("node", "exp2", 0)]);

            assert_ne!(tree1, tree2);
        }

        #[test]
        fn different_depths() {
            let tree1 = generate_tree(vec![("node", "exp", 0)]);
            let tree2 = generate_tree(vec![("node", "exp", 1)]);

            assert_ne!(tree1, tree2);
        }
    }

    mod yaml {
        use super::*;
        use crate::yaml::to_tree;
        use yaml_rust::YamlLoader;
        
        const YAMLDOC: &str =
        "
        node1:
        - subnode1:
          - subsubnode1:
            'subsubnode1 explanation'
          - subsubnode2:
            'subsubnode2 explanation'
        - subnode2:
            'subnode2 explanation'
        node2:
          'node2 explanation'
        node3:
          - subnode1:
            'subnode1 explanation'
        ";


        #[test]
        fn tree() {
            let yaml = YamlLoader::load_from_str(YAMLDOC).unwrap();
            let tree = yaml::to_tree(&yaml[0]);

            let nodes = vec![
                Node::new("root", "", 0),
                Node::new("node1", "", 1),
                Node::new("subnode1", "", 2),
                Node::new("subsubnode1", "subsubnode1 explanation", 3),
                Node::new("subsubnode2", "subsubnode2 explanation", 3),
                Node::new("subnode2", "subnode2 explanation", 2),
                Node::new("node2", "node2 explanation", 1),
                Node::new("node3", "", 1),
                Node::new("subnode1", "subnode1 explanation", 2),
            ];

            for (i, node) in tree.root.descendants(&tree.arena).enumerate() {
                assert_eq!(
                    Node::from_id(&node, &tree.arena), 
                    nodes[i]
                );
            }            
        }
    }
}