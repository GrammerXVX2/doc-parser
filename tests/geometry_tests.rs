use document_parser::merge::geometry::{BBox, bbox_area, bbox_intersection, bbox_iou};

#[test]
fn bbox_area_is_computed_correctly() {
    let b = BBox {
        x0: 0.0,
        y0: 0.0,
        x1: 10.0,
        y1: 20.0,
    };
    assert_eq!(bbox_area(&b), 200.0);
}

#[test]
fn bbox_intersection_and_iou_overlap() {
    let a = BBox {
        x0: 0.0,
        y0: 0.0,
        x1: 10.0,
        y1: 10.0,
    };
    let b = BBox {
        x0: 5.0,
        y0: 5.0,
        x1: 15.0,
        y1: 15.0,
    };

    assert_eq!(bbox_intersection(&a, &b), 25.0);
    let iou = bbox_iou(&a, &b);
    assert!(iou > 0.14 && iou < 0.15);
}

#[test]
fn bbox_non_overlap_returns_zero() {
    let a = BBox {
        x0: 0.0,
        y0: 0.0,
        x1: 10.0,
        y1: 10.0,
    };
    let b = BBox {
        x0: 20.0,
        y0: 20.0,
        x1: 30.0,
        y1: 30.0,
    };

    assert_eq!(bbox_intersection(&a, &b), 0.0);
    assert_eq!(bbox_iou(&a, &b), 0.0);
}

#[test]
fn bbox_empty_box_area_is_zero() {
    let b = BBox {
        x0: 10.0,
        y0: 10.0,
        x1: 10.0,
        y1: 30.0,
    };
    assert_eq!(bbox_area(&b), 0.0);
}
