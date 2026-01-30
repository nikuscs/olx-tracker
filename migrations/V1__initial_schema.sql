-- Saved searches (user's watchlist)
CREATE TABLE IF NOT EXISTS searches (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    keyword TEXT NOT NULL,
    min_price REAL,
    max_price REAL,
    city TEXT,
    radius_km INTEGER,
    category_id INTEGER,
    sort_order TEXT DEFAULT 'newest',
    active INTEGER DEFAULT 1,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT
);

-- Listings found from searches
CREATE TABLE IF NOT EXISTS listings (
    id INTEGER PRIMARY KEY,
    search_id INTEGER REFERENCES searches(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    price REAL,
    currency TEXT DEFAULT 'EUR',
    url TEXT NOT NULL,
    city TEXT,
    region TEXT,
    seller_name TEXT,
    first_seen_at TEXT DEFAULT CURRENT_TIMESTAMP,
    last_seen_at TEXT,
    is_deal INTEGER DEFAULT 0
);

-- Price history for tracking changes
CREATE TABLE IF NOT EXISTS price_history (
    id INTEGER PRIMARY KEY,
    listing_id INTEGER REFERENCES listings(id) ON DELETE CASCADE,
    price REAL,
    recorded_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Search statistics (for avg price calculation)
CREATE TABLE IF NOT EXISTS search_stats (
    search_id INTEGER PRIMARY KEY REFERENCES searches(id) ON DELETE CASCADE,
    avg_price REAL,
    min_price REAL,
    max_price REAL,
    total_listings INTEGER,
    last_updated_at TEXT
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_listings_search_id ON listings(search_id);
CREATE INDEX IF NOT EXISTS idx_listings_price ON listings(price);
CREATE INDEX IF NOT EXISTS idx_listings_is_deal ON listings(is_deal);
CREATE INDEX IF NOT EXISTS idx_price_history_listing_id ON price_history(listing_id);
