//data.rs
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use serde::{Serialize, Deserialize};
use log::info;
use rand::{seq::IteratorRandom, thread_rng};
use std::collections::{HashMap, HashSet, VecDeque};
use rand::prelude::*;


/// Represents an Amazon product with its metadata and co-purchasing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    /// Amazon Standard Identification Number
    pub asin: String,
    
    /// Product title
    pub title: Option<String>,
    
    /// Product categories (hierarchical path)
    pub categories: Vec<Vec<String>>,
    
    /// Product price
    pub price: Option<f64>,
    
    /// Sales rank (lower is better)
    pub sales_rank: Option<u64>,
    
    /// Average review rating (1-5)
    pub avg_rating: Option<f64>,
    
    /// Number of reviews
    pub review_count: usize,
    
    /// ASINs of co-purchased products
    pub similar: Vec<String>,
}

/// Loads the full Amazon metadata dataset
pub fn load_data(file_path: &str) -> Result<Vec<Product>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    
    parse_amazon_meta(reader)
}

/// Loads a random sample of the Amazon metadata with connected products
pub fn load_sample_data(file_path: &str, sample_size: usize) -> Result<Vec<Product>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    
    let all_products = parse_amazon_meta(reader)?;
    info!("Parsed {} total products from file", all_products.len());
    
    // Create a map from ASIN to product for quick lookups
    let asin_to_product: HashMap<String, &Product> = all_products.iter()
        .map(|p| (p.asin.clone(), p))
        .collect();
    
    // Find products with non-empty similar lists as seed candidates
    let candidates: Vec<&Product> = all_products.iter()
        .filter(|p| !p.similar.is_empty())
        .collect();
    
    info!("Found {} products with similar items", candidates.len());
    
    if candidates.is_empty() {
        return Err("No products with similar items found".into());
    }
    
    // Start with a random product that has similar items
    let mut rng = thread_rng();
    let seed_product = candidates.choose(&mut rng).unwrap().clone();
    
    // Track included products
    let mut included_asins = HashSet::new();
    included_asins.insert(seed_product.asin.clone());
    
    // Result list
    let mut sample = vec![seed_product.clone()];
    
    // Processing queue for breadth-first expansion
    let mut queue = VecDeque::new();
    queue.push_back(seed_product.asin.clone());
    
    // Expand using a breadth-first approach
    while sample.len() < sample_size && !queue.is_empty() {
        let current_asin = queue.pop_front().unwrap();
        
        // Get the current product
        if let Some(&product) = asin_to_product.get(&current_asin) {
            // Add its similar products if not already included
            for similar_asin in &product.similar {
                if !included_asins.contains(similar_asin) {
                    if let Some(&similar_product) = asin_to_product.get(similar_asin) {
                        included_asins.insert(similar_asin.clone());
                        sample.push(similar_product.clone());
                        queue.push_back(similar_asin.clone());
                        
                        // Stop if we reached the desired sample size
                        if sample.len() >= sample_size {
                            break;
                        }
                    }
                }
            }
        }
    }
    
    // If we didn't get enough products, fill with random ones
    if sample.len() < sample_size {
        info!("Could only find {} connected products, filling with random ones", sample.len());
        let remaining = sample_size - sample.len();
        
        let additional: Vec<Product> = all_products.iter()
            .filter(|p| !included_asins.contains(&p.asin))
            .choose_multiple(&mut rng, remaining)
            .into_iter()
            .cloned()
            .collect();
        
        sample.extend(additional);
    }
    
    info!("Final sample contains {} products", sample.len());
    
    Ok(sample)
}

/// Parses the Amazon metadata file format
fn parse_amazon_meta<R: BufRead>(reader: R) -> Result<Vec<Product>, Box<dyn Error>> {
    let mut products = Vec::new();
    let mut current_product: Option<Product> = None;
    let mut current_categories: Vec<Vec<String>> = Vec::new();
    
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        
        // Skip empty lines
        if line.is_empty() {
            continue;
        }
        
        // New product entry starts with "ASIN"
        if line.starts_with("ASIN") {
            // Save previous product if exists
            if let Some(product) = current_product.take() {
                products.push(product);
            }
            
            // Initialize new product
            let asin = line.split_whitespace().nth(1).unwrap_or_default().to_string();
            current_product = Some(Product {
                asin,
                title: None,
                categories: Vec::new(),
                price: None,
                sales_rank: None,
                avg_rating: None,
                review_count: 0,
                similar: Vec::new(),
            });
            current_categories = Vec::new();
        } else if let Some(ref mut product) = current_product {
            // Parse product attributes
            if line.starts_with("  title:") {
                product.title = Some(line[8..].trim().to_string());
            } else if line.starts_with("  group:") {
                // Group is not stored but could be useful for filtering
            } else if line.starts_with("  salesrank:") {
                if let Some(rank_str) = line.split_whitespace().nth(1) {
                    product.sales_rank = rank_str.parse::<u64>().ok();
                }
            } else if line.starts_with("  similar:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                // First item is count, then the ASINs
                if parts.len() > 1 {
                    for i in 2..parts.len() {
                        product.similar.push(parts[i].to_string());
                    }
                }
            } else if line.starts_with("  categories:") {
                // Parse the number of categories, but actual categories come on next lines
            } else if line.starts_with("   |") {
                // This is a category hierarchy line
                let category_path: Vec<String> = line[3..]
                    .split('|')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                if !category_path.is_empty() {
                    current_categories.push(category_path);
                }
            } else if line.starts_with("  reviews:") {
                // Format: reviews: total: n downloaded: m avg rating: x
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 9 && parts[7] == "rating:" {
                    product.avg_rating = parts[8].parse::<f64>().ok();
                }
                if parts.len() >= 5 && parts[3] == "downloaded:" {
                    product.review_count = parts[4].parse::<usize>().unwrap_or(0);
                }
            }
        }
    }
    
    // Don't forget to add the last product
    if let Some(mut product) = current_product {
        product.categories = current_categories;
        products.push(product);
    }
    
    info!("Parsed {} products from metadata", products.len());
    
    Ok(products)
}

/// Generates a synthetic dataset with guaranteed connections for testing
pub fn generate_synthetic_dataset(size: usize) -> Vec<Product> {
    use rand::Rng;
    let mut rng = thread_rng();
    let mut products = Vec::with_capacity(size);
    
    // Create categories
    let categories = vec![
        "Books", "Electronics", "Clothing", "Movies & TV", "Music"
    ];
    
    // Create products
    for i in 0..size {
        // Create a unique ASIN
        let asin = format!("A{:08}", i);
        
        // Generate a title
        let title = Some(format!("Product {}", i));
        
        // Assign to 1 category
        let cat_idx = rng.gen_range(0..categories.len());
        let category = categories[cat_idx].to_string();
        
        // Create the product
        products.push(Product {
            asin,
            title,
            categories: vec![vec![category]],
            price: Some(rng.gen_range(5.0..200.0)),
            sales_rank: Some(rng.gen_range(1..100000)),
            avg_rating: Some(rng.gen_range(1.0..5.0)),
            review_count: rng.gen_range(0..100),
            similar: Vec::new(), // Will fill this later
        });
    }
    
    // Create only a few connections per product (2-4)
    for i in 0..products.len() {
        let num_connections = rng.gen_range(2..=4);
        let mut similar = Vec::new();
        
        for _ in 0..num_connections {
            let other_idx = rng.gen_range(0..products.len());
            if other_idx != i && !similar.contains(&products[other_idx].asin) {
                similar.push(products[other_idx].asin.clone());
            }
        }
        
        products[i].similar = similar;
    }
    
    products
}