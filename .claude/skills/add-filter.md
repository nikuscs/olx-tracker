# Add New Filter

Create a new filter for the tracker.

## Steps

1. Create new file `src/filters/my_filter.rs`:

```rust
use crate::api::OfferData;
use crate::db::Search;
use super::Filter;

pub struct MyFilter;

impl Filter for MyFilter {
    fn apply(&self, offer: &OfferData, search: &Search) -> bool {
        // Return true to include, false to exclude
        true
    }

    fn name(&self) -> &'static str {
        "MyFilter"
    }
}
```

2. Export in `src/filters/mod.rs`:

```rust
mod my_filter;
pub use my_filter::MyFilter;
```

3. Optionally add to default chain in `FilterChain::with_defaults()`:

```rust
pub fn with_defaults() -> Self {
    let mut chain = Self::new();
    chain.add(Box::new(KeywordFilter));
    chain.add(Box::new(RadiusFilter));
    chain.add(Box::new(MyFilter));  // Add here
    chain
}
```

4. Add tests in the same file.
