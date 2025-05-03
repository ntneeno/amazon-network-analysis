//graph.rs
use std::error::Error;
use std::collections::{HashMap, HashSet};
use petgraph::graph::{Graph, NodeIndex};
use petgraph::Undirected;
use log::info;
use crate::data::Product;

/// Type alias for our co-purchasing network graph
pub type NetworkGraph = Graph<String, (), Undirected>;

/// Builds the product co-purchasing network graph
pub fn build_graph(products: &[Product]) -> Result<NetworkGraph, Box<dyn Error>> {
    // Create an undirected graph
    let mut graph = Graph::new_undirected();
    
    // Map ASINs to node indices
    let mut asin_to_node = HashMap::new();
    
    // First, add all products as nodes
    for product in products {
        let node_idx = graph.add_node(product.asin.clone());
        asin_to_node.insert(product.asin.clone(), node_idx);
    }
    
    // Count products with similar items
    let products_with_similars = products.iter()
        .filter(|p| !p.similar.is_empty())
        .count();
    
    info!("Products with similar items: {}/{}", products_with_similars, products.len());
    
    // Then add edges for co-purchasing relationships
    let mut edge_count = 0;
    let mut missing_similar_count = 0;
    
    for product in products {
        if let Some(&source_idx) = asin_to_node.get(&product.asin) {
            for similar_asin in &product.similar {
                // Skip self-loops
                if *similar_asin == product.asin {
                    continue;
                }
                
                // Check if similar product exists in our product list
                if let Some(&target_idx) = asin_to_node.get(similar_asin) {
                    // Check if edge already exists to avoid duplicates
                    if !graph.contains_edge(source_idx, target_idx) {
                        graph.add_edge(source_idx, target_idx, ());
                        edge_count += 1;
                    }
                } else {
                    missing_similar_count += 1;
                }
            }
        }
    }
    
    info!("Built graph with {} nodes and {} edges", graph.node_count(), edge_count);
    info!("Missing similar products: {}", missing_similar_count);
    
    // Quick check to see if any nodes have edges
    let nodes_with_edges = graph.node_indices()
        .filter(|&n| graph.neighbors(n).count() > 0)
        .count();
    
    info!("Nodes with at least one edge: {}/{}", nodes_with_edges, graph.node_count());
    
    Ok(graph)
}

/// Extracts a subgraph containing only the specified nodes
pub fn extract_subgraph(graph: &NetworkGraph, nodes: &HashSet<NodeIndex>) -> NetworkGraph {
    let mut subgraph = Graph::new_undirected();
    let mut old_to_new = HashMap::new();
    
    // Add nodes to the subgraph
    for &node_idx in nodes {
        let asin = graph.node_weight(node_idx).unwrap();
        let new_idx = subgraph.add_node(asin.clone());
        old_to_new.insert(node_idx, new_idx);
    }
    
    // Add edges between nodes in the subgraph
    for &node_idx in nodes {
        for neighbor in graph.neighbors(node_idx) {
            if nodes.contains(&neighbor) && neighbor > node_idx {
                let new_source = old_to_new[&node_idx];
                let new_target = old_to_new[&neighbor];
                subgraph.add_edge(new_source, new_target, ());
            }
        }
    }
    
    subgraph
}

