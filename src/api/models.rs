use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub data: Vec<OfferData>,
    #[serde(default)]
    pub metadata: SearchMetadata,
}

/// Response from location autocomplete API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationResponse {
    pub data: Vec<LocationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationResult {
    pub city: LocationCity,
    #[serde(default)]
    pub municipality: Option<LocationMunicipality>,
    #[serde(default)]
    pub region: Option<LocationRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationMunicipality {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchMetadata {
    #[serde(default)]
    pub total_elements: Option<i64>,
    #[serde(default)]
    pub visible_total_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferData {
    pub id: i64,
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub params: Vec<OfferParam>,
    #[serde(default)]
    pub location: Option<OfferLocation>,
    #[serde(default)]
    pub user: Option<OfferUser>,
    #[serde(default)]
    pub photos: Vec<OfferPhoto>,
    #[serde(default)]
    pub created_time: Option<String>,
    #[serde(default)]
    pub last_refresh_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferPhoto {
    pub id: i64,
    pub filename: String,
    pub link: String,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferParam {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub value: Option<ParamValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamValue {
    Simple(String),
    Labeled { value: Option<String>, label: String },
    Numeric { value: f64, label: Option<String> },
    // Catch-all for complex objects we don't need to parse
    Other(serde_json::Value),
}

impl OfferParam {
    pub fn get_value(&self) -> Option<String> {
        match &self.value {
            Some(ParamValue::Simple(s)) => Some(s.clone()),
            Some(ParamValue::Labeled { label, .. }) => Some(label.clone()),
            Some(ParamValue::Numeric { value, .. }) => Some(value.to_string()),
            Some(ParamValue::Other(_)) | None => None,
        }
    }

    pub fn get_numeric_value(&self) -> Option<f64> {
        match &self.value {
            Some(ParamValue::Numeric { value, .. }) => Some(*value),
            Some(ParamValue::Simple(s)) => s.parse().ok(),
            Some(ParamValue::Labeled { value, .. }) => value.as_ref().and_then(|v| v.parse().ok()),
            Some(ParamValue::Other(_)) | None => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferLocation {
    #[serde(default)]
    pub city: Option<LocationCity>,
    #[serde(default)]
    pub region: Option<LocationRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationCity {
    pub id: Option<i64>,
    pub name: String,
    #[serde(default)]
    pub normalized_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationRegion {
    pub id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferUser {
    pub id: Option<i64>,
    pub name: Option<String>,
}

impl OfferPhoto {
    /// Get thumbnail URL (400x300 by default)
    pub fn thumbnail_url(&self) -> String {
        self.link.replace("{width}x{height}", "400x300")
    }

    /// Get full size URL (1200x900)
    pub fn full_url(&self) -> String {
        self.link.replace("{width}x{height}", "1200x900")
    }
}

impl OfferData {
    pub fn get_price(&self) -> Option<f64> {
        self.params.iter().find(|p| p.key == "price").and_then(OfferParam::get_numeric_value)
    }

    pub fn get_city(&self) -> Option<String> {
        self.location.as_ref().and_then(|l| l.city.as_ref()).map(|c| c.name.clone())
    }

    pub fn get_region(&self) -> Option<String> {
        self.location.as_ref().and_then(|l| l.region.as_ref()).map(|r| r.name.clone())
    }

    pub fn get_seller_name(&self) -> Option<String> {
        self.user.as_ref().and_then(|u| u.name.clone())
    }

    /// Get the main image thumbnail URL
    pub fn get_thumbnail(&self) -> Option<String> {
        self.photos.first().map(OfferPhoto::thumbnail_url)
    }

    /// Get all image thumbnail URLs
    pub fn get_all_thumbnails(&self) -> Vec<String> {
        self.photos.iter().map(OfferPhoto::thumbnail_url).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_offer_response() {
        let json = r#"{
            "data": [
                {
                    "id": 12345,
                    "title": "iPhone 12",
                    "url": "https://olx.pt/d/anuncio/12345",
                    "params": [
                        {
                            "key": "price",
                            "name": "Preço",
                            "value": {
                                "value": 400.0,
                                "label": "400 €"
                            }
                        }
                    ],
                    "location": {
                        "city": {
                            "id": 1,
                            "name": "Porto"
                        },
                        "region": {
                            "id": 10,
                            "name": "Norte"
                        }
                    },
                    "user": {
                        "id": 99,
                        "name": "João"
                    }
                }
            ],
            "metadata": {
                "total_elements": 100
            }
        }"#;

        let response: SearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);

        let offer = &response.data[0];
        assert_eq!(offer.id, 12345);
        assert_eq!(offer.title, "iPhone 12");
        assert_eq!(offer.get_price(), Some(400.0));
        assert_eq!(offer.get_city(), Some("Porto".to_string()));
        assert_eq!(offer.get_seller_name(), Some("João".to_string()));
    }
}
