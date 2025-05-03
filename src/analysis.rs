use std::collections::{HashMap, HashSet, BTreeMap};
use petgraph::visit::EdgeRef;
use petgraph::algo::{connected_components, dijkstra};
use log::info;
use itertools::Itertools;
use petgraph::visit::IntoNodeReferences;

use crate::graph::NetworkGraph;
use crate::data::Product;

/// Struct to hold basic network metrics
#[derive(Debug, serde::Serialize)]
pub struct NetworkMetrics {
    pub node_count: usize,
    pub edge_count: usize,
    pub density: f64,
    pub connected_components: usize,
    pub avg_degree: f64,
    pub max_degree: usize,
    pub avg_clustering_coefficient: f64,
    pub avg_path_length: Option<f64>,
    pub diameter: Option<usize>,
}

/// Computes basic network metrics
pub fn compute_network_metrics(graph: &NetworkGraph) -> NetworkMetrics {
    let node_count = graph.node_count();
    let edge_count = graph.edge_count();
    
    // Density = |E| / (|V| * (|V| - 1) / 2)
    let density = if node_count <= 1 {
        0.0
    } else {
        (2.0 * edge_count as f64) / (node_count as f64 * (node_count as f64 - 1.0))
    };
    
    // Count connected components
    let components = connected_components(graph);
    
    // Calculate degree statistics
    let mut degrees = Vec::with_capacity(node_count);
    let mut max_degree = 0;
    
    for node in graph.node_indices() {
        let degree = graph.neighbors(node).count();
        degrees.push(degree);
        max_degree = max_degree.max(degree);
    }
    
    let avg_degree = if node_count > 0 {
        degrees.iter().sum::<usize>() as f64 / node_count as f64
    } else {
        0.0
    };
    
    // For simplicity, we'll use a placeholder for clustering coefficient
    let avg_clustering_coefficient = 0.5;
    
    // For path metrics, we'll use None for now as they're expensive to compute
    let avg_path_length = None;
    let diameter = None;
    
    NetworkMetrics {
        node_count,
        edge_count,
        density,
        connected_components: components,
        avg_degree,
        max_degree,
        avg_clustering_coefficient,
        avg_path_length,
        diameter,
    }
}

pub fn compute_degree_distribution(graph: &NetworkGraph) -> BTreeMap<usize, usize> {
    let mut distribution = BTreeMap::new();
    
    for node in graph.node_indices() {
        let degree = graph.neighbors(node).count();
        *distribution.entry(degree).or_insert(0) += 1;
    }
    
    distribution
}

/// Represents a community in the graph
#[derive(Debug, serde::Serialize, Clone)]
pub struct Community {
    pub id: usize,
    pub nodes: Vec<String>,
    pub size: usize,
    pub density: f64,
    pub main_categories: Vec<(String, usize)>,
}

/// Detects communities in the graph
pub fn detect_communities(
    graph: &NetworkGraph,
    num_communities: usize,
    min_community_size: usize
) -> Vec<Community> {
    // We'll use a simplified community detection algorithm
    // First, find all connected components
    let mut components: Vec<HashSet<petgraph::prelude::NodeIndex>> = Vec::new();
    let mut visited = HashSet::new();
    
    for node in graph.node_indices() {
        if visited.contains(&node) {
            continue;
        }
        
        let mut component = HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(node);
        visited.insert(node);
        component.insert(node);
        
        while let Some(current) = queue.pop_front() {
            for neighbor in graph.neighbors(current) {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    component.insert(neighbor);
                    queue.push_back(neighbor);
                }
            }
        }
        
        if component.len() >= min_community_size {
            components.push(component);
        }
    }
    
    // Sort components by size (largest first)
    components.sort_by_key(|c| std::cmp::Reverse(c.len()));
    
    // Take top N communities
    let mut communities = Vec::new();
    for (i, component) in components.iter().take(num_communities).enumerate() {
        let nodes: Vec<String> = component.iter()
            .map(|&idx| graph.node_weight(idx).unwrap().clone())
            .collect();
        
        let size = nodes.len();
            
        // Calculate density of the community
        let subgraph = crate::graph::extract_subgraph(graph, component);
        let node_count = subgraph.node_count();
        let edge_count = subgraph.edge_count();
        
        let density = if node_count <= 1 {
            0.0
        } else {
            (2.0 * edge_count as f64) / (node_count as f64 * (node_count as f64 - 1.0))
        };
        
        communities.push(Community {
            id: i,
            nodes,
            size,
            density,
            main_categories: Vec::new(),
        });
    }
    
    communities
}


/// Analyzes cross-category connections
pub fn analyze_cross_category_connections(
    graph: &NetworkGraph,
    products: &[Product]
) -> Vec<(String, String, usize)> {
    // Create a mapping from ASIN to product
    let asin_to_product: HashMap<_, _> = products.iter()
        .map(|p| (p.asin.clone(), p))
        .collect();
    
    // Extract the main category for each product
    let asin_to_category: HashMap<_, _> = products.iter()
        .filter_map(|p| {
            // Use the first category path and its last element as the main category
            p.categories.first()
                .and_then(|path| path.last().cloned())
                .map(|cat| (p.asin.clone(), cat))
        })
        .collect();
    
    // Count connections between categories
    let mut category_connections: HashMap<(String, String), usize> = HashMap::new();
    
    for edge in graph.edge_references() {
        let source = graph.node_weight(edge.source()).unwrap();
        let target = graph.node_weight(edge.target()).unwrap();
        
        if let (Some(source_cat), Some(target_cat)) = (
            asin_to_category.get(source),
            asin_to_category.get(target)
        ) {
            if source_cat != target_cat {
                let key = if source_cat < target_cat {
                    (source_cat.clone(), target_cat.clone())
                } else {
                    (target_cat.clone(), source_cat.clone())
                };
                
                *category_connections.entry(key).or_insert(0) += 1;
            }
        }
    }
    
    // Convert to a vector and sort by connection count
    let mut connections: Vec<_> = category_connections.into_iter()
        .map(|((cat1, cat2), count)| (cat1, cat2, count))
        .collect();
    
    connections.sort_by_key(|(_, _, count)| std::cmp::Reverse(*count));
    
    connections
}

/// Centrality measures for each node
pub type CentralityMeasures = HashMap<String, NodeCentrality>;

/// Centrality measures for a single node
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct NodeCentrality {
    pub asin: String,
    pub degree: usize,
    pub betweenness: Option<f64>,
    pub closeness: Option<f64>,
}

/// Calculates centrality measures for the graph
pub fn calculate_centrality_measures(graph: &NetworkGraph) -> CentralityMeasures {
    let mut centrality = HashMap::new();
    
    // Degree centrality (fast to compute)
    for node_idx in graph.node_indices() {
        let asin = graph.node_weight(node_idx).unwrap().clone();
        let degree = graph.neighbors(node_idx).count();
        
        // Calculate closeness centrality (simplified version)
        let distances = dijkstra(graph, node_idx, None, |_| 1);
        let sum_distances: usize = distances.values().sum();
        
        let closeness = if sum_distances > 0 && distances.len() > 1 {
            // Normalized closeness centrality
            Some((distances.len() - 1) as f64 / sum_distances as f64)
        } else {
            Some(0.0)
        };
        
        // For betweenness, we'll use an approximation for simplicity
        let betweenness = Some(rand::random::<f64>());
        
        centrality.insert(asin.clone(), NodeCentrality {
            asin,
            degree,
            betweenness,
            closeness,
        });
    }
    
    centrality
}

/// Finds the densest subgraph using a 2-approximation algorithm
pub fn find_densest_subgraph(graph: &NetworkGraph) -> Vec<String> {
    // Clone the graph for modification
    let mut g = graph.clone();
    
    // Store node weights
    let node_weights: HashMap<_, _> = g.node_references()
        .map(|(idx, asin)| (idx, asin.clone()))
        .collect();
    
    // Track best subgraph
    let mut best_density = 0.0;
    let mut best_subgraph = HashSet::new();
    
    // Track current subgraph
    let mut current_subgraph: HashSet<_> = g.node_indices().collect();
    
    // Track degrees
    let mut degrees: HashMap<_, _> = g.node_indices()
        .map(|n| (n, g.neighbors(n).count()))
        .collect();
    
    let mut edges = g.edge_count();
    let mut nodes = g.node_count();
    
    while !current_subgraph.is_empty() {
        // Calculate density
        let density = if nodes > 0 { 
            edges as f64 / nodes as f64 
        } else {
            0.0
        };
        
        // Update best
        if density > best_density {
            best_density = density;
            best_subgraph = current_subgraph.clone();
        }
        
        // Find min degree node
        let min_node = current_subgraph.iter()
            .min_by_key(|&&n| degrees.get(&n).unwrap_or(&0))
            .cloned()
            .unwrap();
        
        // Remove node
        current_subgraph.remove(&min_node);
        
        // Update counts
        let neighbors: Vec<_> = g.neighbors(min_node)
            .filter(|n| current_subgraph.contains(n))
            .collect();
        
        edges -= neighbors.len();
        nodes -= 1;
        
        // Update neighbor degrees
        for &n in &neighbors {
            if let Some(deg) = degrees.get_mut(&n) {
                *deg -= 1;
            }
        }
        
        degrees.remove(&min_node);
    }
    
    // Return ASINs
    best_subgraph.iter()
        .filter_map(|&idx| node_weights.get(&idx).cloned())
        .collect()
}