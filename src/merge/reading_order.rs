use crate::model::DocumentModel;

pub struct ReadingOrderEngine;

impl ReadingOrderEngine {
    pub fn assign_natural_order(document: &mut DocumentModel) {
        let mut global = 1_u32;
        for page in &mut document.pages {
            for (idx, element) in page.elements.iter_mut().enumerate() {
                element.reading_order = Some((idx + 1) as u32);
                element.global_order = Some(global);
                global += 1;
            }
        }
        document.extra.insert(
            "reading_order_strategy".to_string(),
            serde_json::json!("natural"),
        );
    }
}
