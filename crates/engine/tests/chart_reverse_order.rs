//! `chart.set {"xReversed": bool}`: Format Axis ▸ Categories in reverse order (#1687).

use gridcraft_engine::{Session, io};
use serde_json::json;

fn reversed(s: &Session) -> bool {
    s.doc().unwrap().wb.sheet(0).unwrap().charts[0].x_reversed
}

#[test]
fn categories_in_reverse_order_is_a_command_that_undoes_and_saves() {
    let mut s = Session::new();
    s.new_workbook();
    s.execute("range.setValues", json!({"range": "A1", "values": [["Task", "Days"], ["One", "3"], ["Two", "5"], ["Three", "2"]]})).unwrap();
    s.execute("insert.chart", json!({"range": "A1:B4", "type": "bar"})).unwrap();
    assert!(!reversed(&s));

    s.execute("chart.set", json!({"xReversed": true})).unwrap();
    assert!(reversed(&s));
    s.execute("edit.undo", json!({})).unwrap();
    assert!(!reversed(&s));
    s.execute("chart.set", json!({"xReversed": true})).unwrap();
    // Other chart.set params leave it alone.
    s.execute("chart.set", json!({"title": "Plan"})).unwrap();
    assert!(reversed(&s));

    let bytes = io::save_bytes(&s.doc().unwrap().wb, "plan.xlsx").unwrap();
    let (wb, _) = io::open_bytes("plan.xlsx", &bytes).unwrap();
    assert!(wb.sheet(0).unwrap().charts[0].x_reversed);

    s.execute("chart.set", json!({"xReversed": false})).unwrap();
    assert!(!reversed(&s));
}
