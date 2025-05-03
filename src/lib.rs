//lib.rs
// Library module for Amazon network analysis that can be used by other programs
// Exports the main functionality as a reusable library

use std::error::Error;
use std::path::Path;
use std::collections::HashMap;
use crate::analysis::{
    NetworkMetrics,
    compute_network_metrics,
    compute_degree_distribution,
    Community,
    detect_communities,
    analyze_cross_category_connections,
    calculate_centrality_measures,
    NodeCentrality,
    CentralityMeasures,
    find_densest_subgraph
};

mod data;
mod graph;
mod analysis;
mod visualization;

pub use data::{Product, load_data, load_sample_data};
pub use graph::{NetworkGraph, build_graph, extract_subgraph};
/*
pub use analysis::{
    NetworkMetrics,
    compute_network_metrics,
    compute_degree_distribution,
    Community,
    detect_communities,
    analyze_cross_category_connections,
    calculate_centrality_measures,
    NodeCentrality,
    CentralityMeasures,
    find_densest_subgraph
};
*/
pub use visualization::{
    save_metrics,
    save_degree_distribution,
    save_communities,
    save_cross_category_connections,
    save_centrality_measures,
    save_densest_subgraph
};

/// Performs a complete analysis of the Amazon product co-purchasing network
/// and saves the results to the specified output directory
pub fn analyze_network(
    input_file: &str,
    output_dir: &str,
    num_communities: usize,
    min_community_size: usize,
    sample: bool,
    sample_size: usize
) -> Result<AnalysisResults, Box<dyn Error>> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)?;
    
    // Load and preprocess data
    let products = if sample {
        load_sample_data(input_file, sample_size)?
    } else {
        load_data(input_file)?
    };
    
    // Build the graph
    let graph = build_graph(&products)?;
    
    // Perform analysis
    let metrics = compute_network_metrics(&graph);
    save_metrics(&metrics, Path::new(output_dir).join("metrics.json"))?;
    
    let degree_distribution = compute_degree_distribution(&graph);
    save_degree_distribution(
        &degree_distribution, 
        Path::new(output_dir).join("degree_distribution.csv")
    )?;
    
    let communities = detect_communities(&graph, num_communities, min_community_size);
    save_communities(
        &communities, 
        &products,
        Path::new(output_dir).join("communities.json")
    )?;
    
    let cross_category = analyze_cross_category_connections(&graph, &products);
    save_cross_category_connections(
        &cross_category,
        Path::new(output_dir).join("cross_category.csv")
    )?;
    
    let centrality = calculate_centrality_measures(&graph);
    save_centrality_measures(
        &centrality,
        &products,
        Path::new(output_dir).join("centrality.csv")
    )?;
    
    let densest = find_densest_subgraph(&graph);
    save_densest_subgraph(
        &densest,
        &products,
        Path::new(output_dir).join("densest_subgraph.json")
    )?;
    
    // Return analysis results
    Ok(AnalysisResults {
        metrics,
        communities: communities.len(),
        top_products: get_top_products(&centrality, &products, 10),
        cross_category_count: cross_category.len(),
        densest_subgraph_size: densest.len(),
    })
}

/// Represents the summary results of the network analysis
#[derive(Debug)]
pub struct AnalysisResults {
    pub metrics: NetworkMetrics,
    pub communities: usize,
    pub top_products: Vec<(String, String, usize)>, // (ASIN, Title, Degree)
    pub cross_category_count: usize,
    pub densest_subgraph_size: usize,
}

/// Gets the top N products by degree centrality
fn get_top_products(
    centrality: &CentralityMeasures,
    products: &[Product],
    n: usize
) -> Vec<(String, String, usize)> {
    // Create a mapping from ASIN to product
    let asin_to_product: HashMap<_, _> = products.iter()
        .map(|p| (p.asin.clone(), p))
        .collect();
    
    // Sort centrality measures by degree (descending)
    let mut centrality_vec: Vec<_> = centrality.values().collect();
    centrality_vec.sort_by_key(|c| std::cmp::Reverse(c.degree));
    
    // Get top N products
    centrality_vec.iter()
        .take(n)
        .map(|c| {
            let title = match asin_to_product.get(&c.asin) {
                Some(product) => product.title.clone().unwrap_or_else(|| "Unknown".to_string()),
                _ => "Unknown".to_string(),  // Using underscore pattern
            };
            
            (c.asin.clone(), title, c.degree)
        })
        .collect()
}// Implemented full analysis logic
