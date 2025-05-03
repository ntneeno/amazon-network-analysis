use std::error::Error;
use std::path::Path;
use clap::Parser;
use log::{info, error};
use std::collections::HashSet;
use rand::Rng;

mod data;
mod graph;
mod analysis;
mod visualization;

/// Command line arguments for the Amazon Network Analysis tool
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to the Amazon metadata file
    #[clap(short, long, default_value = "data/amazon-meta.txt")]
    input_file: String,
    
    /// Output directory for results
    #[clap(short, long, default_value = "results")]
    output_dir: String,
    
    /// Number of communities to detect
    #[clap(short, long, default_value_t = 10)]
    num_communities: usize,
    
    /// Minimum number of nodes in a community
    #[clap(short, long, default_value_t = 3)]
    min_community_size: usize,
    
    /// Whether to run analysis on a sample of the data
    #[clap(long)]
    sample: bool,
    
    /// Sample size (if sampling)
    #[clap(long, default_value_t = 2000)]
    sample_size: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger
    env_logger::init();
    
    // Parse command line arguments
    let args = Args::parse();
    
    info!("Starting Amazon product co-purchasing network analysis");
    info!("Input file: {}", args.input_file);
    
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&args.output_dir)?;
    
    // Load and preprocess data
    info!("Loading data");
    let products = if args.sample {
        data::generate_synthetic_dataset(args.sample_size)
    } else {
        data::load_data(&args.input_file)?
    };
    
    info!("Loaded {} products", products.len());
    
    // Build the graph
    info!("Building co-purchasing network graph");
    let graph = graph::build_graph(&products)?;
    
    info!("Graph built with {} nodes and {} edges", 
          graph.node_count(), 
          graph.edge_count());
    
    // Perform analysis
    info!("Running network analysis");
    
    // Compute and save basic network metrics
    let metrics = analysis::compute_network_metrics(&graph);
    visualization::save_metrics(&metrics, Path::new(&args.output_dir).join("metrics.json"))?;
    
    // Compute degree distribution
    let degree_distribution = analysis::compute_degree_distribution(&graph);
    visualization::save_degree_distribution(
        &degree_distribution, 
        Path::new(&args.output_dir).join("degree_distribution.csv")
    )?;
    
    // Detect communities
    let communities = analysis::detect_communities(
        &graph, 
        args.num_communities,
        args.min_community_size
    );
    visualization::save_communities(
        &communities, 
        &products,
        Path::new(&args.output_dir).join("communities.json")
    )?;
    
    // Analyze cross-category connections
    let cross_category = analysis::analyze_cross_category_connections(&graph, &products);
    visualization::save_cross_category_connections(
        &cross_category,
        Path::new(&args.output_dir).join("cross_category.csv")
    )?;
    
    // Calculate centrality measures
    let centrality = analysis::calculate_centrality_measures(&graph);
    visualization::save_centrality_measures(
        &centrality,
        &products,
        Path::new(&args.output_dir).join("centrality.csv")
    )?;
    
    // Find densest subgraph
    let densest = analysis::find_densest_subgraph(&graph);
    visualization::save_densest_subgraph(
        &densest,
        &products,
        Path::new(&args.output_dir).join("densest_subgraph.json")
    )?;
    
    info!("Analysis complete. Results saved to {}", args.output_dir);
    
    Ok(())
}// Final tweaks
