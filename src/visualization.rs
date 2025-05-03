use std::error::Error;
use std::path::Path;
use std::fs::File;
use std::io::BufWriter;
use std::collections::HashMap;
use serde_json;
use csv;
use itertools::Itertools;

use crate::graph::NetworkGraph;
use crate::data::Product;

type NetworkMetrics = crate::analysis::NetworkMetrics;
type Community = crate::analysis::Community;
type NodeCentrality = crate::analysis::NodeCentrality;

/// Saves network metrics to a JSON file
pub fn save_metrics(
    metrics: &NetworkMetrics,
    output_path: impl AsRef<Path>
) -> Result<(), Box<dyn Error>> {
    let file = File::create(output_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, metrics)?;
    Ok(())
}

/// Saves degree distribution to a CSV file
pub fn save_degree_distribution(
    distribution: &std::collections::BTreeMap<usize, usize>,
    output_path: impl AsRef<Path>
) -> Result<(), Box<dyn Error>> {
    let file = File::create(output_path)?;
    let mut writer = csv::Writer::from_writer(BufWriter::new(file));
    
    writer.write_record(&["Degree", "Count"])?;
    
    for (&degree, &count) in distribution {
        writer.write_record(&[degree.to_string(), count.to_string()])?;
    }
    
    writer.flush()?;
    Ok(())
}

/// Saves community detection results to a JSON file
pub fn save_communities(
    communities: &[Community],
    products: &[Product],
    output_path: impl AsRef<Path>
) -> Result<(), Box<dyn Error>> {
    // Create a mapping from ASIN to product
    let asin_to_product: HashMap<_, _> = products.iter()
        .map(|p| (p.asin.clone(), p))
        .collect();
    
    // For each community, add category information
    let mut enriched_communities = Vec::with_capacity(communities.len());
    
    for community in communities {
        // Count categories in this community
        let mut category_counts = HashMap::new();
        
        for asin in &community.nodes {
            if let Some(product) = asin_to_product.get(asin) {
                for cat_path in &product.categories {
                    if let Some(category) = cat_path.last() {
                        *category_counts.entry(category.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
        
        // Extract main categories (top 5)
        let main_categories: Vec<(String, usize)> = category_counts.into_iter()
            .sorted_by_key(|(_, count)| std::cmp::Reverse(*count))
            .take(5)
            .collect();
        
        // Create enriched community with category information
        let mut enriched = community.clone();
        enriched.main_categories = main_categories;
        
        enriched_communities.push(enriched);
    }
    
    // Save to output file
    let file = File::create(output_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &enriched_communities)?;
    
    Ok(())
}

/// Saves cross-category connection analysis to a CSV file
pub fn save_cross_category_connections(
    connections: &[(String, String, usize)],
    output_path: impl AsRef<Path>
) -> Result<(), Box<dyn Error>> {
    let file = File::create(output_path)?;
    let mut writer = csv::Writer::from_writer(BufWriter::new(file));
    
    writer.write_record(&["Category1", "Category2", "ConnectionCount"])?;
    
    for (cat1, cat2, count) in connections {
        writer.write_record(&[cat1, cat2, &count.to_string()])?;
    }
    
    writer.flush()?;
    Ok(())
}

/// Saves centrality measures to a CSV file
pub fn save_centrality_measures(
    centrality: &HashMap<String, NodeCentrality>,
    products: &[Product],
    output_path: impl AsRef<Path>
) -> Result<(), Box<dyn Error>> {
    let file = File::create(output_path)?;
    let mut writer = csv::Writer::from_writer(BufWriter::new(file));
    
    // Create a mapping from ASIN to product
    let asin_to_product: HashMap<_, _> = products.iter()
        .map(|p| (p.asin.clone(), p))
        .collect();
    
    // Write header
    writer.write_record(&[
        "ASIN", 
        "Title", 
        "MainCategory", 
        "Degree", 
        "Betweenness", 
        "Closeness"
    ])?;
    
    // Sort by degree centrality (descending)
    let sorted_centrality: Vec<_> = centrality.values()
        .sorted_by_key(|c| std::cmp::Reverse(c.degree))
        .collect();
    
    for central in sorted_centrality {
        let asin = &central.asin;
        
        // Get product info
        let (title, category) = if let Some(product) = asin_to_product.get(asin) {
            let title = product.title.as_deref().unwrap_or("Unknown");
            
            let category = product.categories.first()
                .and_then(|path| path.last())
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());
            
            (title, category)
        } else {
            ("Unknown", "Unknown".to_string())
        };
        
        // Format betweenness and closeness as strings
        let betweenness = central.betweenness
            .map(|v| format!("{:.6}", v))
            .unwrap_or_else(|| "N/A".to_string());
            
        let closeness = central.closeness
            .map(|v| format!("{:.6}", v))
            .unwrap_or_else(|| "N/A".to_string());
        
        writer.write_record(&[
            asin,
            title,
            &category,
            &central.degree.to_string(),
            &betweenness,
            &closeness
        ])?;
    }
    
    writer.flush()?;
    Ok(())
}

/// Saves densest subgraph results to a JSON file
pub fn save_densest_subgraph(
    nodes: &[String],
    products: &[Product],
    output_path: impl AsRef<Path>
) -> Result<(), Box<dyn Error>> {
    // Create a mapping from ASIN to product
    let asin_to_product: HashMap<_, _> = products.iter()
        .map(|p| (p.asin.clone(), p))
        .collect();
    
    // Create enriched node information
    #[derive(serde::Serialize)]
    struct EnrichedNode {
        asin: String,
        title: String,
        category: String,
        price: Option<f64>,
    }
    
    let enriched_nodes: Vec<_> = nodes.iter()
        .filter_map(|asin| {
            asin_to_product.get(asin).map(|product| {
                EnrichedNode {
                    asin: asin.clone(),
                    title: product.title.clone().unwrap_or_else(|| "Unknown".to_string()),
                    category: product.categories.first()
                        .and_then(|path| path.last().cloned())
                        .unwrap_or_else(|| "Unknown".to_string()),
                    price: product.price,
                }
            })
        })
        .collect();
    
    // Save to output file
    let file = File::create(output_path)?;
    let writer = BufWriter::new(file);
    
    #[derive(serde::Serialize)]
    struct DensestSubgraph {
        size: usize,
        nodes: Vec<EnrichedNode>,
    }
    
    let result = DensestSubgraph {
        size: enriched_nodes.len(),
        nodes: enriched_nodes,
    };
    
    serde_json::to_writer_pretty(writer, &result)?;
    
    Ok(())
}