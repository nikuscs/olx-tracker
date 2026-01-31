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

    #[test]
    fn test_offer_photo_urls() {
        let photo = OfferPhoto {
            id: 1,
            filename: "test.jpg".to_string(),
            link: "https://example.com/img/{width}x{height}/test.jpg".to_string(),
            width: Some(800),
            height: Some(600),
        };

        assert_eq!(photo.thumbnail_url(), "https://example.com/img/400x300/test.jpg");
        assert_eq!(photo.full_url(), "https://example.com/img/1200x900/test.jpg");
    }

    #[test]
    fn test_offer_get_region() {
        let offer = OfferData {
            id: 1,
            title: "Test".to_string(),
            url: "url".to_string(),
            params: vec![],
            location: Some(OfferLocation {
                city: None,
                region: Some(LocationRegion { id: Some(10), name: "Norte".to_string() }),
            }),
            user: None,
            photos: vec![],
            created_time: None,
            last_refresh_time: None,
        };

        assert_eq!(offer.get_region(), Some("Norte".to_string()));
    }

    #[test]
    fn test_offer_get_thumbnails() {
        let offer = OfferData {
            id: 1,
            title: "Test".to_string(),
            url: "url".to_string(),
            params: vec![],
            location: None,
            user: None,
            photos: vec![
                OfferPhoto {
                    id: 1,
                    filename: "1.jpg".to_string(),
                    link: "https://example.com/{width}x{height}/1.jpg".to_string(),
                    width: None,
                    height: None,
                },
                OfferPhoto {
                    id: 2,
                    filename: "2.jpg".to_string(),
                    link: "https://example.com/{width}x{height}/2.jpg".to_string(),
                    width: None,
                    height: None,
                },
            ],
            created_time: None,
            last_refresh_time: None,
        };

        assert_eq!(offer.get_thumbnail(), Some("https://example.com/400x300/1.jpg".to_string()));
        assert_eq!(offer.get_all_thumbnails().len(), 2);
        assert_eq!(offer.get_all_thumbnails()[0], "https://example.com/400x300/1.jpg");
        assert_eq!(offer.get_all_thumbnails()[1], "https://example.com/400x300/2.jpg");
    }

    #[test]
    fn test_offer_no_photos() {
        let offer = OfferData {
            id: 1,
            title: "Test".to_string(),
            url: "url".to_string(),
            params: vec![],
            location: None,
            user: None,
            photos: vec![],
            created_time: None,
            last_refresh_time: None,
        };

        assert_eq!(offer.get_thumbnail(), None);
        assert!(offer.get_all_thumbnails().is_empty());
    }

    #[test]
    fn test_param_value_simple() {
        let param = OfferParam {
            key: "condition".to_string(),
            name: "Condition".to_string(),
            value: Some(ParamValue::Simple("Used".to_string())),
        };

        assert_eq!(param.get_value(), Some("Used".to_string()));
        assert_eq!(param.get_numeric_value(), None);
    }

    #[test]
    fn test_param_value_labeled() {
        let param = OfferParam {
            key: "fuel".to_string(),
            name: "Fuel".to_string(),
            value: Some(ParamValue::Labeled {
                value: Some("diesel".to_string()),
                label: "Diesel".to_string(),
            }),
        };

        assert_eq!(param.get_value(), Some("Diesel".to_string()));
        assert_eq!(param.get_numeric_value(), None);
    }

    #[test]
    fn test_param_value_numeric_string_conversion() {
        let param = OfferParam {
            key: "year".to_string(),
            name: "Year".to_string(),
            value: Some(ParamValue::Simple("2020".to_string())),
        };

        assert_eq!(param.get_numeric_value(), Some(2020.0));
    }

    #[test]
    fn test_param_value_numeric_get_value() {
        // Tests line 91: get_value() on Numeric param returns value as string
        let param = OfferParam {
            key: "price".to_string(),
            name: "Price".to_string(),
            value: Some(ParamValue::Numeric {
                value: 123.45, label: Some("123.45 €".to_string())
            }),
        };

        assert_eq!(param.get_value(), Some("123.45".to_string()));
    }

    #[test]
    fn test_param_value_other() {
        let param = OfferParam {
            key: "complex".to_string(),
            name: "Complex".to_string(),
            value: Some(ParamValue::Other(serde_json::json!({"nested": "data"}))),
        };

        assert_eq!(param.get_value(), None);
        assert_eq!(param.get_numeric_value(), None);
    }

    #[test]
    fn test_param_value_none() {
        let param = OfferParam { key: "empty".to_string(), name: "Empty".to_string(), value: None };

        assert_eq!(param.get_value(), None);
        assert_eq!(param.get_numeric_value(), None);
    }

    #[test]
    fn test_offer_no_location() {
        let offer = OfferData {
            id: 1,
            title: "Test".to_string(),
            url: "url".to_string(),
            params: vec![],
            location: None,
            user: None,
            photos: vec![],
            created_time: None,
            last_refresh_time: None,
        };

        assert_eq!(offer.get_city(), None);
        assert_eq!(offer.get_region(), None);
    }

    #[test]
    fn test_offer_no_user() {
        let offer = OfferData {
            id: 1,
            title: "Test".to_string(),
            url: "url".to_string(),
            params: vec![],
            location: None,
            user: None,
            photos: vec![],
            created_time: None,
            last_refresh_time: None,
        };

        assert_eq!(offer.get_seller_name(), None);
    }

    #[test]
    fn test_location_response_parsing() {
        let json = r#"{
            "data": [
                {
                    "city": {
                        "id": 1,
                        "name": "Porto",
                        "normalized_name": "porto"
                    },
                    "region": {
                        "id": 10,
                        "name": "Norte"
                    },
                    "municipality": {
                        "name": "Porto Municipality"
                    }
                }
            ]
        }"#;

        let response: LocationResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].city.name, "Porto");
        assert_eq!(response.data[0].region.as_ref().unwrap().name, "Norte");
        assert_eq!(response.data[0].municipality.as_ref().unwrap().name, "Porto Municipality");
    }

    #[test]
    fn test_search_metadata_defaults() {
        let json = r#"{"data": []}"#;
        let response: SearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.metadata.total_elements, None);
        assert_eq!(response.metadata.visible_total_count, None);
    }
}
