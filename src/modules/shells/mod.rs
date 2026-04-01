pub mod starship;

use super::ConfigGenerator;

pub fn get_all() -> Vec<Box<dyn ConfigGenerator>> {
    vec![Box::new(starship::StarshipGenerator)]
}
