use crate::errors::SortOrderError;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::MAIN_SEPARATOR;

#[derive(PartialEq)]
pub enum SortOrder {
    Asc,
    Desc,
}

pub enum Source {
    Regex(Regex, usize, Option<usize>),
    Map(Vec<(String, String)>),
    Sort(SortOrder),
}

impl Source {
    pub fn new_regex(
        pattern: &str,
        depth: Option<usize>,
        max_depth: Option<usize>,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self::Regex(
            Regex::new(pattern)?,
            depth.unwrap_or(pattern.chars().filter(|c| *c == MAIN_SEPARATOR).count() + 1),
            max_depth,
        ))
    }

    pub fn new_map(filename: &str) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(filename)?;
        Ok(Self::Map(serde_json::from_str(contents.as_str()).map(
            |map: HashMap<String, String>| map.into_iter().collect(),
        )?))
    }

    pub fn new_sort(order: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self::Sort(match order.to_lowercase().as_str() {
            "asc" => SortOrder::Asc,
            "desc" => SortOrder::Desc,
            _ => return Err(Box::new(SortOrderError::new(order))),
        }))
    }
}
