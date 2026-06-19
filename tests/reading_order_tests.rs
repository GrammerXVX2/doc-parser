use document_parser::layout::{
    LayoutRegion, LayoutRegionType, LayoutSource, ReadingOrderOptions,
    assign_layout_aware_reading_order,
};
use document_parser::model::{ContentMode, Element, ElementType, Page, PageProfile, PageType};
use document_parser::utils::geometry::BBox;

fn text_element(id: &str, bbox: [f32; 4]) -> Element {
    Element {
        element_id: id.to_string(),
        element_type: ElementType::Paragraph,
        tag: Some("p".to_string()),
        role: None,
        reading_order: None,
        global_order: None,
        bbox: Some(bbox),
        polygon: None,
        content: serde_json::json!({"text": id, "markdown": id}),
        style: serde_json::json!({}),
        provenance: serde_json::json!({}),
        confidence: serde_json::json!({"overall":0.9}),
        warnings: vec![],
        extra: Default::default(),
    }
}

#[test]
fn two_column_order_is_left_then_right() {
    let mut pages = vec![Page {
        page_number: 1,
        page_type: PageType::DocumentPage,
        width: Some(1000.0),
        height: Some(1400.0),
        dpi: None,
        rotation_degrees: 0.0,
        page_image_asset_id: None,
        page_profile: PageProfile {
            content_mode: ContentMode::Digital,
            has_native_text: true,
            has_ocr_required_regions: false,
            has_tables: false,
            has_images: false,
            has_formulas: false,
            has_handwriting: false,
            language: Some("ru".to_string()),
            language_info: Default::default(),
            confidence: 0.9,
        },
        elements: vec![
            text_element("l1", [50.0, 100.0, 420.0, 130.0]),
            text_element("l2", [50.0, 180.0, 420.0, 210.0]),
            text_element("r1", [580.0, 110.0, 940.0, 140.0]),
            text_element("r2", [580.0, 190.0, 940.0, 220.0]),
        ],
        text: String::new(),
        markdown: String::new(),
        html: String::new(),
        warnings: vec![],
        extra: Default::default(),
    }];

    assign_layout_aware_reading_order(&mut pages, &[], ReadingOrderOptions::default())
        .expect("assign works");

    let ids = pages[0]
        .elements
        .iter()
        .map(|e| e.element_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["l1", "l2", "r1", "r2"]);
}

#[test]
fn caption_follows_figure() {
    let mut page = Page {
        page_number: 1,
        page_type: PageType::DocumentPage,
        width: Some(1000.0),
        height: Some(1400.0),
        dpi: None,
        rotation_degrees: 0.0,
        page_image_asset_id: None,
        page_profile: PageProfile {
            content_mode: ContentMode::Digital,
            has_native_text: true,
            has_ocr_required_regions: false,
            has_tables: false,
            has_images: true,
            has_formulas: false,
            has_handwriting: false,
            language: Some("ru".to_string()),
            language_info: Default::default(),
            confidence: 0.9,
        },
        elements: vec![
            Element {
                element_id: "caption".to_string(),
                element_type: ElementType::Caption,
                tag: Some("figcaption".to_string()),
                role: Some("caption".to_string()),
                reading_order: None,
                global_order: None,
                bbox: Some([100.0, 510.0, 900.0, 540.0]),
                polygon: None,
                content: serde_json::json!({"text":"Рисунок 1"}),
                style: serde_json::json!({}),
                provenance: serde_json::json!({}),
                confidence: serde_json::json!({"overall":0.9}),
                warnings: vec![],
                extra: Default::default(),
            },
            Element {
                element_id: "figure".to_string(),
                element_type: ElementType::Image,
                tag: Some("img".to_string()),
                role: Some("figure".to_string()),
                reading_order: None,
                global_order: None,
                bbox: Some([100.0, 300.0, 900.0, 500.0]),
                polygon: None,
                content: serde_json::json!({"text":""}),
                style: serde_json::json!({}),
                provenance: serde_json::json!({}),
                confidence: serde_json::json!({"overall":0.9}),
                warnings: vec![],
                extra: Default::default(),
            },
        ],
        text: String::new(),
        markdown: String::new(),
        html: String::new(),
        warnings: vec![],
        extra: Default::default(),
    };

    let regions = vec![LayoutRegion {
        region_id: "fig_r1".to_string(),
        page_number: 1,
        region_type: LayoutRegionType::Figure,
        bbox: BBox {
            x0: 100.0,
            y0: 300.0,
            x1: 900.0,
            y1: 500.0,
        },
        polygon: None,
        confidence: 0.9,
        reading_order: None,
        source: LayoutSource::Heuristic,
    }];

    assign_layout_aware_reading_order(
        std::slice::from_mut(&mut page),
        &regions,
        ReadingOrderOptions::default(),
    )
    .expect("assign works");

    assert_eq!(page.elements[0].element_id, "figure");
    assert_eq!(page.elements[1].element_id, "caption");
}
