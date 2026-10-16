//! Categories in reverse order: `c:orientation val="maxMin"` on the category axis is read into
//! `Chart.x_reversed` and written back (#1687).

use std::io::{Read, Write};
use std::sync::Arc;

use gridcraft_model::{Anchor, Chart, ChartKind, LegendPos, Series, Sheet, Workbook};

use crate::{read_xlsx, write_xlsx};

fn series(values: &str, secondary: bool, kind: Option<ChartKind>) -> Series {
    Series {
        name: None,
        categories: Some("Data!$A$2:$A$4".into()),
        values: values.into(),
        bubble_sizes: None,
        color: None,
        secondary,
        kind,
        smooth: false,
        trendlines: Vec::new(),
    }
}

fn book(kind: ChartKind, series: Vec<Series>, x_reversed: bool) -> Workbook {
    let mut s = Sheet::new("Data");
    s.charts.push(Chart {
        id: 1,
        kind,
        anchor: Anchor { cell: gridcraft_core::CellRef::new(0, 3), width: 480.0, height: 300.0, ..Anchor::default() },
        title: None,
        series,
        legend: LegendPos::Bottom,
        data_labels: false,
        gridlines: true,
        style: 2,
        x_title: None,
        y_title: None,
        source: None,
        by_rows: false,
        pivot: None,
        x_reversed,
    });
    Workbook { sheets: vec![Arc::new(s)], ..Workbook::default() }
}

fn chart_xml(bytes: &[u8]) -> String {
    let mut xml = String::new();
    zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap().by_name("xl/charts/chart1.xml").unwrap().read_to_string(&mut xml).unwrap();
    xml
}

/// The package with `edit` applied to its chart part.
fn edit_chart(bytes: &[u8], edit: impl Fn(String) -> String) -> Vec<u8> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let name = f.name().to_string();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        if name == "xl/charts/chart1.xml" {
            data = edit(String::from_utf8(data).unwrap()).into_bytes();
        }
        out.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        out.write_all(&data).unwrap();
    }
    out.finish().unwrap().into_inner()
}

/// The `<c:orientation>` of each axis, in file order, as (axis element, orientation).
fn orientations(xml: &str) -> Vec<(String, String)> {
    let mut axes: Vec<(usize, &str)> = Vec::new();
    for (tag, open) in [("catAx", "<c:catAx>"), ("valAx", "<c:valAx>")] {
        axes.extend(xml.match_indices(open).map(|(i, _)| (i, tag)));
    }
    axes.sort();
    axes.into_iter()
        .map(|(i, tag)| {
            let o = xml.get(i..).and_then(|r| r.split_once("<c:orientation val=\"")).and_then(|(_, r)| r.split_once('"')).map(|(v, _)| v.to_string());
            (tag.to_string(), o.unwrap_or_default())
        })
        .collect()
}

#[test]
fn reversed_category_axis_is_written_and_read_back() {
    let wb = book(ChartKind::BarStacked, vec![series("Data!$B$2:$B$4", false, None), series("Data!$C$2:$C$4", false, None)], true);
    let bytes = write_xlsx(&wb).unwrap();
    // Only the category axis runs from its maximum.
    assert_eq!(orientations(&chart_xml(&bytes)), [("catAx".to_string(), "maxMin".to_string()), ("valAx".to_string(), "minMax".to_string())]);
    let (back, _) = read_xlsx(&bytes).unwrap();
    assert!(back.sheets[0].charts[0].x_reversed);

    // Off by default, and off stays off.
    let bytes = write_xlsx(&book(ChartKind::ColumnClustered, vec![series("Data!$B$2:$B$4", false, None)], false)).unwrap();
    assert!(!chart_xml(&bytes).contains("maxMin"));
    assert!(!read_xlsx(&bytes).unwrap().0.sheets[0].charts[0].x_reversed);
}

#[test]
fn combo_chart_reverses_both_category_axes() {
    // The secondary group has its own (hidden) category axis: Excel would draw that group in
    // the original order if only the primary axis were reversed.
    let wb = book(
        ChartKind::Combo,
        vec![series("Data!$B$2:$B$4", false, Some(ChartKind::ColumnClustered)), series("Data!$C$2:$C$4", true, Some(ChartKind::Line))],
        true,
    );
    let xml = chart_xml(&write_xlsx(&wb).unwrap());
    let o = orientations(&xml);
    assert_eq!(o.iter().filter(|(t, v)| t == "catAx" && v == "maxMin").count(), 2, "{o:?}");
    assert!(o.iter().filter(|(t, _)| t == "valAx").all(|(_, v)| v == "minMax"), "{o:?}");
}

#[test]
fn reverse_order_is_read_from_the_category_axis_only() {
    let plain = write_xlsx(&book(ChartKind::BarClustered, vec![series("Data!$B$2:$B$4", false, None)], false)).unwrap();
    // As Excel saves it: maxMin on the category axis.
    let cat = edit_chart(&plain, |x| {
        x.replacen(
            "<c:catAx><c:axId val=\"1\"/><c:scaling><c:orientation val=\"minMax\"/>",
            "<c:catAx><c:axId val=\"1\"/><c:scaling><c:orientation val=\"maxMin\"/>",
            1,
        )
    });
    assert!(read_xlsx(&cat).unwrap().0.sheets[0].charts[0].x_reversed);
    // A reversed value axis is a different setting.
    let val = edit_chart(&plain, |x| {
        x.replacen(
            "<c:valAx><c:axId val=\"2\"/><c:scaling><c:orientation val=\"minMax\"/>",
            "<c:valAx><c:axId val=\"2\"/><c:scaling><c:orientation val=\"maxMin\"/>",
            1,
        )
    });
    assert!(chart_xml(&val).contains("maxMin"));
    assert!(!read_xlsx(&val).unwrap().0.sheets[0].charts[0].x_reversed);
    // A scatter chart's X axis is a value axis: not "categories in reverse order".
    let scatter = write_xlsx(&book(ChartKind::Scatter, vec![series("Data!$B$2:$B$4", false, None)], false)).unwrap();
    let scatter = edit_chart(&scatter, |x| x.replacen("<c:orientation val=\"minMax\"/>", "<c:orientation val=\"maxMin\"/>", 1));
    assert!(chart_xml(&scatter).contains("maxMin"));
    assert!(!read_xlsx(&scatter).unwrap().0.sheets[0].charts[0].x_reversed);
    // Unknown or missing orientation reads as the default.
    let odd = edit_chart(&plain, |x| x.replace("<c:orientation val=\"minMax\"/>", "<c:orientation val=\"sideways\"/>"));
    assert!(!read_xlsx(&odd).unwrap().0.sheets[0].charts[0].x_reversed);
    let none = edit_chart(&plain, |x| x.replace("<c:orientation val=\"minMax\"/>", ""));
    assert!(!read_xlsx(&none).unwrap().0.sheets[0].charts[0].x_reversed);
}
