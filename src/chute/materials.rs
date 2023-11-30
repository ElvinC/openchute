
use serde::{Serialize, Deserialize};

const LB_TO_N: f64 = 4.4482216153;

// Cord, string, or rope
#[derive(Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cord {
    name: String,
    linear_density_g_per_m: f64,
    strength_newton: f64,
    diameter_mm: f64,
}

impl Cord {
    pub fn new(name: &str, strength_newton: f64, linear_density_g_per_m: f64, diameter_mm: f64) -> Self {
        Self {
            name: name.to_string(),
            linear_density_g_per_m, 
            strength_newton,
            diameter_mm,
        }
    }

    // Helper function that converts from commonly listed (imperial) units
    pub fn new_convert(name: &str, strength_lb: f64, linear_density_oz_per_100ft: f64, diameter_mm: f64) -> Self {
        Self {
            name: name.to_string(),
            linear_density_g_per_m: linear_density_oz_per_100ft * 28.3495 / 30.48, 
            strength_newton: strength_lb * LB_TO_N,
            diameter_mm,
        }
    }

    pub fn get_types() -> Vec<Cord> {
        // Default cord types
        
        vec![
            // Kevlar based on https://aliexpress.com/item/1005002176036650.html
            Self::new_convert("Kevlar", 100.0, 0.32, 0.8),
            Self::new_convert("Kevlar", 150.0, 0.55, 1.0),
            Self::new_convert("Kevlar", 200.0, 0.85, 1.1),
            Self::new_convert("Kevlar", 300.0, 1.06, 1.3),
            Self::new_convert("Kevlar", 400.0, 1.69, 1.6),
            Self::new_convert("Kevlar", 500.0, 2.36, 2.0),
            Self::new_convert("Kevlar", 750.0, 3.63, 2.3),
            Self::new_convert("Kevlar", 1000.0, 5.15, 3.0),
            Self::new_convert("Kevlar", 1500.0, 8.82, 3.5),
            Self::new_convert("Kevlar", 2000.0, 10.69, 4.0),
            Self::new_convert("Kevlar", 3000.0, 16.47, 4.8),
            Self::new_convert("Kevlar", 5000.0, 25.75, 6.8),

            // Based on aliexpress.com/item/1005004988870613.html
            Self::new_convert("UHMWPE", 100.0, 0.42, 0.5),
            Self::new_convert("UHMWPE", 220.0, 0.63, 0.8),
            Self::new_convert("UHMWPE", 350.0, 0.81, 1.0),
            Self::new_convert("UHMWPE", 580.0, 1.48, 1.3),
            Self::new_convert("UHMWPE", 750.0, 1.8, 1.6),
        ]


    }
}