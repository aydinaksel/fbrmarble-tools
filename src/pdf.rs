use chrono::NaiveDateTime;
use lopdf::{
    content::{Content, Operation},
    dictionary, Document, Object, ObjectId, Stream,
};

use crate::database::PurchaseOrderLine;

// Landscape A4 in points (1 inch = 72 pt)
const PAGE_WIDTH_PT: f32 = 841.89;
const PAGE_HEIGHT_PT: f32 = 595.28;
const MARGIN_PT: f32 = 36.0; // 0.5 inch

const CONTENT_WIDTH_PT: f32 = PAGE_WIDTH_PT - 2.0 * MARGIN_PT;
const CONTENT_HEIGHT_PT: f32 = PAGE_HEIGHT_PT - 2.0 * MARGIN_PT;

const HEADER_BAR_HEIGHT_PT: f32 = 43.2; // 0.60 inch
const BOTTOM_ROW_HEIGHT_PT: f32 = 120.24; // 1.67 inches
const DISCLAIMER_WIDTH_PT: f32 = CONTENT_WIDTH_PT * 3.0 / 4.0;
const CRATE_NUMBER_WIDTH_PT: f32 = CONTENT_WIDTH_PT - DISCLAIMER_WIDTH_PT;

const SKU_ROW_HEIGHT_PT: f32 = 86.4; // 1.2 inches
const DESCRIPTION_ROW_HEIGHT_PT: f32 = 72.0; // 1.0 inch
const CUSTOMER_ROW_HEIGHT_PT: f32 = 64.8; // 0.9 inches
const DETAILS_ROW_HEIGHT_PT: f32 = 64.8; // 0.9 inches
const QUANTITY_ROW_HEIGHT_PT: f32 = CONTENT_HEIGHT_PT
    - HEADER_BAR_HEIGHT_PT
    - BOTTOM_ROW_HEIGHT_PT
    - SKU_ROW_HEIGHT_PT
    - DESCRIPTION_ROW_HEIGHT_PT
    - CUSTOMER_ROW_HEIGHT_PT
    - DETAILS_ROW_HEIGHT_PT;

const CELL_HEADER_HEIGHT_PT: f32 = 15.84; // 0.22 inch
const CELL_PADDING_PT: f32 = 5.76; // 0.08 inch

pub struct CrateLabel {
    pub sku_number: String,
    pub description: String,
    pub warehouse_name: String,
    pub location_stock_type: String,
    pub customer: String,
    pub customer_sku: String,
    pub origin: String,
    pub date: NaiveDateTime,
    pub square_footage_per_crate: String,
    pub pieces_per_crate: String,
    pub weight_per_crate_lbs: String,
    pub crate_number: usize,
}

pub fn expand_to_crate_labels(lines: Vec<PurchaseOrderLine>) -> Vec<CrateLabel> {
    let mut labels = Vec::new();
    for line in lines {
        let Some(number_of_crates) = line.number_of_crates else {
            continue;
        };
        if number_of_crates <= 0.0 {
            continue;
        }
        let crate_count = number_of_crates.round() as usize;
        for crate_number in 1..=crate_count {
            labels.push(CrateLabel {
                sku_number: line.sku_number.clone(),
                description: line.description.clone().unwrap_or_default(),
                warehouse_name: line.warehouse_name.clone().unwrap_or_default(),
                location_stock_type: line.location_stock_type.clone(),
                customer: line.customer.clone().unwrap_or_default(),
                customer_sku: line.customer_sku.clone().unwrap_or_default(),
                origin: line.origin.clone().unwrap_or_default(),
                date: line.date,
                square_footage_per_crate: format_square_footage(line.square_footage_per_crate),
                pieces_per_crate: format_whole_number(line.pieces_per_crate.as_deref()),
                weight_per_crate_lbs: format_whole_number(line.weight_per_crate_lbs.as_deref()),
                crate_number,
            });
        }
    }
    labels
}

fn format_square_footage(value: Option<f64>) -> String {
    match value {
        Some(sqf) => format!("{:.0} SQF", sqf.round()),
        None => String::new(),
    }
}

fn format_whole_number(value: Option<&str>) -> String {
    match value {
        Some(text) => text.split('.').next().unwrap_or(text).to_string(),
        None => String::new(),
    }
}

pub fn generate_pdf(labels: &[CrateLabel]) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");

    let helvetica_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let helvetica_bold_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica-Bold",
        "Encoding" => "WinAnsiEncoding",
    });
    let helvetica_oblique_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica-Oblique",
        "Encoding" => "WinAnsiEncoding",
    });

    let pages_id = doc.new_object_id();

    let page_ids: Vec<Object> = labels
        .iter()
        .map(|label| {
            add_label_page(
                &mut doc,
                label,
                pages_id,
                helvetica_id,
                helvetica_bold_id,
                helvetica_oblique_id,
            )
            .into()
        })
        .collect();

    let page_count = page_ids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids,
            "Count" => page_count,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

fn add_label_page(
    doc: &mut Document,
    label: &CrateLabel,
    pages_id: ObjectId,
    regular_id: ObjectId,
    bold_id: ObjectId,
    oblique_id: ObjectId,
) -> ObjectId {
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => regular_id,
            "F2" => bold_id,
            "F3" => oblique_id,
        },
    });

    let operations = build_label_operations(label);
    let content = Content { operations };
    let content_stream = Stream::new(dictionary! {}, content.encode().unwrap());
    let content_id = doc.add_object(content_stream);

    doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Real(PAGE_WIDTH_PT),
            Object::Real(PAGE_HEIGHT_PT),
        ],
        "Resources" => resources_id,
        "Contents" => content_id,
    })
}

// Converts a y coordinate from top-down layout to PDF bottom-up.
// Returns the bottom edge of the element in PDF coordinates.
fn pdf_y(top_y_pt: f32, height_pt: f32) -> f32 {
    PAGE_HEIGHT_PT - top_y_pt - height_pt
}

fn build_label_operations(label: &CrateLabel) -> Vec<Operation> {
    let mut ops: Vec<Operation> = Vec::new();

    let origin_x = MARGIN_PT;
    let origin_y = MARGIN_PT;

    // ── Header bar ──────────────────────────────────────────────────────────
    filled_rect(
        &mut ops,
        origin_x,
        origin_y,
        CONTENT_WIDTH_PT,
        HEADER_BAR_HEIGHT_PT,
        (0.941, 0.941, 0.941),
    );
    stroked_rect(
        &mut ops,
        origin_x,
        origin_y,
        CONTENT_WIDTH_PT,
        HEADER_BAR_HEIGHT_PT,
    );

    // Logo placeholder box
    let logo_width = 160.0_f32;
    let logo_padding = 5.76_f32;
    filled_rect(
        &mut ops,
        origin_x + logo_padding,
        origin_y + logo_padding,
        logo_width,
        HEADER_BAR_HEIGHT_PT - 2.0 * logo_padding,
        (0.784, 0.784, 0.784),
    );
    place_text(
        &mut ops,
        "[ COMPANY LOGO ]",
        "F3",
        8.0,
        origin_x + CELL_PADDING_PT,
        origin_y + HEADER_BAR_HEIGHT_PT / 2.0 - 3.0,
        (0.392, 0.392, 0.392),
    );

    let mut row_y = origin_y + HEADER_BAR_HEIGHT_PT;

    // ── Row 1: SKU ───────────────────────────────────────────────────────────
    draw_table_cell(
        &mut ops,
        origin_x,
        row_y,
        CONTENT_WIDTH_PT,
        SKU_ROW_HEIGHT_PT,
        "SKU NUMBER",
        &label.sku_number.to_uppercase(),
        24.0,
    );
    row_y += SKU_ROW_HEIGHT_PT;

    // ── Row 2: Description ───────────────────────────────────────────────────
    draw_table_cell(
        &mut ops,
        origin_x,
        row_y,
        CONTENT_WIDTH_PT,
        DESCRIPTION_ROW_HEIGHT_PT,
        "DESCRIPTION",
        &label.description.to_uppercase(),
        13.0,
    );
    row_y += DESCRIPTION_ROW_HEIGHT_PT;

    // ── Row 3: Customer | Customer SKU ───────────────────────────────────────
    let half_width = CONTENT_WIDTH_PT / 2.0;
    draw_table_cell(
        &mut ops,
        origin_x,
        row_y,
        half_width,
        CUSTOMER_ROW_HEIGHT_PT,
        "CUSTOMER",
        &label.customer.to_uppercase(),
        13.0,
    );
    draw_table_cell(
        &mut ops,
        origin_x + half_width,
        row_y,
        half_width,
        CUSTOMER_ROW_HEIGHT_PT,
        "CUSTOMER SKU",
        &label.customer_sku.to_uppercase(),
        13.0,
    );
    row_y += CUSTOMER_ROW_HEIGHT_PT;

    // ── Row 4: Origin | Warehouse | Stock Type | Date ────────────────────────
    let quarter_width = CONTENT_WIDTH_PT / 4.0;
    draw_table_cell(
        &mut ops,
        origin_x,
        row_y,
        quarter_width,
        DETAILS_ROW_HEIGHT_PT,
        "ORIGIN",
        &label.origin.to_uppercase(),
        11.0,
    );
    draw_table_cell(
        &mut ops,
        origin_x + quarter_width,
        row_y,
        quarter_width,
        DETAILS_ROW_HEIGHT_PT,
        "WAREHOUSE",
        &label.warehouse_name.to_uppercase(),
        11.0,
    );
    draw_table_cell(
        &mut ops,
        origin_x + 2.0 * quarter_width,
        row_y,
        quarter_width,
        DETAILS_ROW_HEIGHT_PT,
        "STOCK TYPE",
        &label.location_stock_type.to_uppercase(),
        11.0,
    );
    let date_text = label.date.format("%-d %B %Y").to_string().to_uppercase();
    draw_table_cell(
        &mut ops,
        origin_x + 3.0 * quarter_width,
        row_y,
        quarter_width,
        DETAILS_ROW_HEIGHT_PT,
        "DATE",
        &date_text,
        11.0,
    );
    row_y += DETAILS_ROW_HEIGHT_PT;

    // ── Row 5: Quantity | Pieces | Weight ────────────────────────────────────
    let third_width = CONTENT_WIDTH_PT / 3.0;
    draw_table_cell(
        &mut ops,
        origin_x,
        row_y,
        third_width,
        QUANTITY_ROW_HEIGHT_PT,
        "QUANTITY PER CRATE",
        &label.square_footage_per_crate,
        22.0,
    );
    draw_table_cell(
        &mut ops,
        origin_x + third_width,
        row_y,
        third_width,
        QUANTITY_ROW_HEIGHT_PT,
        "PIECES PER CRATE",
        &label.pieces_per_crate,
        22.0,
    );
    draw_table_cell(
        &mut ops,
        origin_x + 2.0 * third_width,
        row_y,
        third_width,
        QUANTITY_ROW_HEIGHT_PT,
        "WEIGHT PER CRATE",
        &format!("{} LBS", label.weight_per_crate_lbs),
        22.0,
    );
    row_y += QUANTITY_ROW_HEIGHT_PT;

    // ── Row 6: Disclaimer | Crate Number ─────────────────────────────────────
    draw_disclaimer_cell(&mut ops, origin_x, row_y, DISCLAIMER_WIDTH_PT, BOTTOM_ROW_HEIGHT_PT);
    draw_crate_number_cell(
        &mut ops,
        origin_x + DISCLAIMER_WIDTH_PT,
        row_y,
        CRATE_NUMBER_WIDTH_PT,
        BOTTOM_ROW_HEIGHT_PT,
        label.crate_number,
    );

    ops
}

fn draw_table_cell(
    ops: &mut Vec<Operation>,
    x: f32,
    top_y: f32,
    width: f32,
    height: f32,
    header: &str,
    value: &str,
    value_size_pt: f32,
) {
    stroked_rect(ops, x, top_y, width, height);

    filled_rect(
        ops,
        x,
        top_y,
        width,
        CELL_HEADER_HEIGHT_PT,
        (0.824, 0.824, 0.824),
    );

    // Header label — baseline just above bottom of header band
    place_text(
        ops,
        header,
        "F1",
        7.0,
        x + CELL_PADDING_PT,
        top_y + CELL_HEADER_HEIGHT_PT - 4.5,
        (0.196, 0.196, 0.196),
    );

    // Value: centred vertically in the area below the header band
    let value_area_top = top_y + CELL_HEADER_HEIGHT_PT;
    let value_area_height = height - CELL_HEADER_HEIGHT_PT;
    // Approximate text height as 70% of point size
    let approx_text_height = value_size_pt * 0.7;
    let baseline_y = value_area_top + (value_area_height - approx_text_height) / 2.0 + approx_text_height;

    place_text(
        ops,
        value,
        "F2",
        value_size_pt,
        x + CELL_PADDING_PT,
        baseline_y,
        (0.0, 0.0, 0.0),
    );
}

fn draw_disclaimer_cell(
    ops: &mut Vec<Operation>,
    x: f32,
    top_y: f32,
    width: f32,
    height: f32,
) {
    stroked_rect(ops, x, top_y, width, height);
    filled_rect(ops, x, top_y, width, CELL_HEADER_HEIGHT_PT, (0.824, 0.824, 0.824));

    place_text(
        ops,
        "DISCLAIMER",
        "F1",
        7.0,
        x + CELL_PADDING_PT,
        top_y + CELL_HEADER_HEIGHT_PT - 4.5,
        (0.196, 0.196, 0.196),
    );

    let disclaimer = "This label is intended solely for the use of the named recipient and contains confidential \
information pertaining to the shipment of goods. The contents of this crate have been packed and verified \
in accordance with the applicable purchase order. Any discrepancies in quantity, condition, or identity of \
goods must be reported immediately to the warehouse manager upon receipt. Unauthorised reproduction or \
distribution of this label is strictly prohibited. FBR Marble accepts no liability for errors arising from \
illegible or damaged labels.";

    let max_chars_per_line = ((width - 2.0 * CELL_PADDING_PT) / 4.2) as usize;
    let lines = wrap_text(disclaimer, max_chars_per_line);
    let line_height_pt = 9.5_f32;
    let text_start_y = top_y + CELL_HEADER_HEIGHT_PT + line_height_pt + 2.0;

    for (index, line) in lines.iter().enumerate() {
        let line_y = text_start_y + index as f32 * line_height_pt;
        if line_y + line_height_pt > top_y + height {
            break;
        }
        place_text(
            ops,
            line,
            "F1",
            7.5,
            x + CELL_PADDING_PT,
            line_y,
            (0.235, 0.235, 0.235),
        );
    }
}

fn draw_crate_number_cell(
    ops: &mut Vec<Operation>,
    x: f32,
    top_y: f32,
    width: f32,
    height: f32,
    crate_number: usize,
) {
    stroked_rect(ops, x, top_y, width, height);
    filled_rect(ops, x, top_y, width, CELL_HEADER_HEIGHT_PT, (0.824, 0.824, 0.824));

    place_text(
        ops,
        "CRATE NUMBER",
        "F1",
        7.0,
        x + CELL_PADDING_PT,
        top_y + CELL_HEADER_HEIGHT_PT - 4.5,
        (0.196, 0.196, 0.196),
    );

    let number_size = 38.0_f32;
    let value_area_top = top_y + CELL_HEADER_HEIGHT_PT;
    let value_area_height = height - CELL_HEADER_HEIGHT_PT;
    let approx_text_height = number_size * 0.7;
    let baseline_y = value_area_top + (value_area_height - approx_text_height) / 2.0 + approx_text_height;

    let text = crate_number.to_string();
    // Approximate centering: Helvetica-Bold digit width ≈ 0.6 × size
    let approx_text_width = text.len() as f32 * number_size * 0.6;
    let centered_x = x + (width - approx_text_width) / 2.0;

    place_text(ops, &text, "F2", number_size, centered_x, baseline_y, (0.0, 0.0, 0.0));
}

// ── PDF drawing primitives ───────────────────────────────────────────────────

fn filled_rect(
    ops: &mut Vec<Operation>,
    x: f32,
    top_y: f32,
    width: f32,
    height: f32,
    rgb: (f32, f32, f32),
) {
    let bottom = pdf_y(top_y, height);
    ops.extend([
        Operation::new("q", vec![]),
        Operation::new("rg", vec![rgb.0.into(), rgb.1.into(), rgb.2.into()]),
        Operation::new("re", vec![x.into(), bottom.into(), width.into(), height.into()]),
        Operation::new("f", vec![]),
        Operation::new("Q", vec![]),
    ]);
}

fn stroked_rect(ops: &mut Vec<Operation>, x: f32, top_y: f32, width: f32, height: f32) {
    let bottom = pdf_y(top_y, height);
    ops.extend([
        Operation::new("q", vec![]),
        Operation::new("RG", vec![0.0_f32.into(), 0.0_f32.into(), 0.0_f32.into()]),
        Operation::new("w", vec![Object::Real(0.5)]),
        Operation::new("re", vec![x.into(), bottom.into(), width.into(), height.into()]),
        Operation::new("S", vec![]),
        Operation::new("Q", vec![]),
    ]);
}

/// Places a single line of text. `baseline_top_y` is measured from the top of the page
/// down to the text baseline.
fn place_text(
    ops: &mut Vec<Operation>,
    text: &str,
    font_name: &str,
    size_pt: f32,
    x: f32,
    baseline_top_y: f32,
    rgb: (f32, f32, f32),
) {
    let baseline_pdf_y = PAGE_HEIGHT_PT - baseline_top_y;
    let safe_text = escape_pdf_string(text);
    ops.extend([
        Operation::new("q", vec![]),
        Operation::new("rg", vec![rgb.0.into(), rgb.1.into(), rgb.2.into()]),
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec![Object::Name(font_name.as_bytes().to_vec()), Object::Real(size_pt)]),
        Operation::new("Tm", vec![
            1.0_f32.into(), 0.0_f32.into(),
            0.0_f32.into(), 1.0_f32.into(),
            x.into(), baseline_pdf_y.into(),
        ]),
        Operation::new("Tj", vec![Object::String(safe_text, lopdf::StringFormat::Literal)]),
        Operation::new("ET", vec![]),
        Operation::new("Q", vec![]),
    ]);
}

fn escape_pdf_string(text: &str) -> Vec<u8> {
    let mut result = Vec::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'(' => { result.push(b'\\'); result.push(b'('); }
            b')' => { result.push(b'\\'); result.push(b')'); }
            b'\\' => { result.push(b'\\'); result.push(b'\\'); }
            other => result.push(other),
        }
    }
    result
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current.clone());
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
