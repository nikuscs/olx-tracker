---
name: add-filter
description: Create a new filter for the tracker system. Use when adding custom filtering logic or when the user asks to create a filter.
user-invocable: false
---

# Add New Filter

Create a new filter for the olx-tracker filtering system.

## Filter Structure

Filters implement the `Filter` trait and are applied in a chain to include/exclude listings.

## Steps

### 1. Create the filter file

Create `src/filters/$ARGUMENTS.rs` or `src/filters/my_filter.rs`:

```rust
use crate::api::OfferData;
use crate::db::Search;
use super::Filter;

pub struct MyFilter;

impl Filter for MyFilter {
    fn apply(&self, offer: &OfferData, search: &Search) -> bool {
        // Return true to include listing, false to exclude
        true
    }

    fn name(&self) -> &'static str {
        "MyFilter"
    }
}
```

### 2. Export in filters module

Add to `src/filters/mod.rs`:

```rust
mod my_filter;
pub use my_filter::MyFilter;
```

### 3. Add to default chain (optional)

To apply filter automatically, add to `FilterChain::with_defaults()` in `src/filters/mod.rs`:

```rust
pub fn with_defaults() -> Self {
    let mut chain = Self::new();
    chain.add(Box::new(KeywordFilter));
    chain.add(Box::new(RadiusFilter));
    chain.add(Box::new(PriceFilter));
    chain.add(Box::new(MyFilter));  // Add your filter here
    chain
}
```

### 4. Add tests

Add tests in the same file under `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_filter() {
        let filter = MyFilter;
        // Test filter logic
    }
}
```

## Existing Filters

- `KeywordFilter`: Filters by keyword match in title
- `RadiusFilter`: Filters by location/city match
- `PriceFilter`: Filters by min/max price range

## Filter Trait

```rust
pub trait Filter {
    fn apply(&self, offer: &OfferData, search: &Search) -> bool;
    fn name(&self) -> &'static str;
}
```

Return `true` to include the listing, `false` to exclude it.
