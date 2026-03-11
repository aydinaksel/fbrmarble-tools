use chrono::NaiveDateTime;
use printpdf::{
    Color, Line, LinePoint, Mm, Op, PaintMode, ParsedFont, PdfDocument, PdfFontHandle, PdfPage,
    PdfSaveOptions, Point, Polygon, PolygonRing, Pt, RawImage, Rgb, TextItem, WindingOrder,
    XObjectId, XObjectTransform,
};

use crate::database::PurchaseOrderLine;

const REGULAR_TTF: &[u8] = include_bytes!(
    "../atkinson-hyperlegible-next-mono/fonts/ttf/AtkinsonHyperlegibleMono-Regular.ttf"
);
const BOLD_TTF: &[u8] = include_bytes!(
    "../atkinson-hyperlegible-next-mono/fonts/ttf/AtkinsonHyperlegibleMono-Bold.ttf"
);
const LOGO_PNG: &[u8] = include_bytes!("../fbr_logo.png");

const PAGE_WIDTH_MM: f32 = 297.0;
const PAGE_HEIGHT_MM: f32 = 210.0;

// Advance width fraction for Atkinson Hyperlegible Mono (632/1000 em)
const CHAR_ADVANCE_FRACTION: f32 = 0.632;

// Font sizes as stored in the SVG (user units = mm)
const HEADER_LABEL_SIZE_MM: f32 = 4.23333;
const VALUE_SIZE_MM: f32 = 7.05556;
const DISCLAIMER_SIZE_MM: f32 = 3.52778;

// #bfbfbf = 191/255
const HEADER_BACKGROUND_GRAY: f32 = 191.0 / 255.0;

// Logo PNG is 320×317 px; displayed at 40×40 mm
const LOGO_PNG_WIDTH_PX: f32 = 320.0;
const LOGO_PNG_HEIGHT_PX: f32 = 317.0;
const LOGO_DISPLAY_MM: f32 = 40.0;
const LOGO_DPI: f32 = 300.0;

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
    let regular_font =
        ParsedFont::from_bytes(REGULAR_TTF, 0, &mut Vec::new()).expect("regular TTF parse failed");
    let bold_font =
        ParsedFont::from_bytes(BOLD_TTF, 0, &mut Vec::new()).expect("bold TTF parse failed");
    let logo_image =
        RawImage::decode_from_bytes(LOGO_PNG, &mut Vec::new()).expect("logo PNG decode failed");

    let mut doc = PdfDocument::new("FBR Marble Crate Labels");
    let regular_id = doc.add_font(&regular_font);
    let bold_id = doc.add_font(&bold_font);
    let logo_id = doc.add_image(&logo_image);

    let regular = PdfFontHandle::External(regular_id);
    let bold = PdfFontHandle::External(bold_id);

    let pages: Vec<PdfPage> = labels
        .iter()
        .map(|label| {
            let ops = build_page_ops(label, &regular, &bold, &logo_id);
            PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops)
        })
        .collect();

    doc.with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut Vec::new())
}

fn build_page_ops(
    label: &CrateLabel,
    regular: &PdfFontHandle,
    bold: &PdfFontHandle,
    logo_id: &XObjectId,
) -> Vec<Op> {
    let mut ops = Vec::new();
    draw_header_backgrounds(&mut ops);
    draw_grid(&mut ops);
    draw_logo(&mut ops, logo_id);
    draw_all_text(label, &mut ops, regular, bold);
    ops
}

// ── Header background fills ───────────────────────────────────────────────────

fn draw_header_backgrounds(ops: &mut Vec<Op>) {
    let gray = gray(HEADER_BACKGROUND_GRAY);

    // Row 1: SKU NUMBER | DESCRIPTION | ORIGIN  (y=10..20)
    fill_rect(ops, 11.0, 10.0, 40.0, 10.0, gray.clone());
    fill_rect(ops, 51.0, 10.0, 210.0, 10.0, gray.clone());
    fill_rect(ops, 261.0, 10.0, 25.0, 10.0, gray.clone());

    // Row 2: CUSTOMER | CUSTOMER SKU  (y=45..55)
    fill_rect(ops, 11.0, 45.0, 185.0, 10.0, gray.clone());
    fill_rect(ops, 196.0, 45.0, 90.0, 10.0, gray.clone());

    // Row 3: WAREHOUSE | STOCK TYPE | DATE  (y=80..90)
    fill_rect(ops, 11.0, 80.0, 100.0, 10.0, gray.clone());
    fill_rect(ops, 111.0, 80.0, 95.0, 10.0, gray.clone());
    fill_rect(ops, 206.0, 80.0, 80.0, 10.0, gray.clone());

    // Row 4: QUANTITY IN CRATE | PIECES IN CRATE | WEIGHT OF CRATE | CRATE NUMBER  (y=115..125)
    fill_rect(ops, 11.0, 115.0, 65.0, 10.0, gray.clone());
    fill_rect(ops, 76.0, 115.0, 70.0, 10.0, gray.clone());
    fill_rect(ops, 146.0, 115.0, 70.0, 10.0, gray.clone());
    fill_rect(ops, 216.0, 115.0, 70.0, 10.0, gray.clone());

    // Row 5: DISCLAIMER  (y=150..160)
    fill_rect(ops, 11.0, 150.0, 225.0, 10.0, gray);
}

// ── Grid lines ────────────────────────────────────────────────────────────────

fn draw_grid(ops: &mut Vec<Op>) {
    ops.push(Op::SetOutlineColor {
        col: black(),
    });
    ops.push(Op::SetOutlineThickness { pt: Pt(0.5) });

    // Outer border
    draw_line(ops, 11.0, 10.0, 11.0, 200.0);
    draw_line(ops, 286.0, 10.0, 286.0, 200.0);
    draw_line(ops, 11.0, 10.0, 286.0, 10.0);
    draw_line(ops, 11.0, 200.0, 286.0, 200.0);

    // Internal horizontals
    draw_line(ops, 11.0, 45.0, 286.0, 45.0);
    draw_line(ops, 11.0, 80.0, 286.0, 80.0);
    draw_line(ops, 11.0, 115.0, 286.0, 115.0);
    draw_line(ops, 11.0, 150.0, 286.0, 150.0);

    // Row 1 verticals
    draw_line(ops, 51.0, 10.0, 51.0, 45.0);
    draw_line(ops, 261.0, 10.0, 261.0, 45.0);

    // Row 2 vertical
    draw_line(ops, 196.0, 45.0, 196.0, 80.0);

    // Row 3 verticals
    draw_line(ops, 111.0, 80.0, 111.0, 115.0);
    draw_line(ops, 206.0, 80.0, 206.0, 115.0);

    // Row 4 verticals
    draw_line(ops, 76.0, 115.0, 76.0, 150.0);
    draw_line(ops, 146.0, 115.0, 146.0, 150.0);
    draw_line(ops, 216.0, 115.0, 216.0, 150.0);

    // Row 5 vertical
    draw_line(ops, 236.0, 150.0, 236.0, 200.0);
}

// ── Logo ──────────────────────────────────────────────────────────────────────

fn draw_logo(ops: &mut Vec<Op>, logo_id: &XObjectId) {
    // SVG position: x=241, y=155 (top-down), w=40, h=40
    // PDF bottom-left: x=241mm, y=210-155-40=15mm
    let native_width_mm = LOGO_PNG_WIDTH_PX * 25.4 / LOGO_DPI;
    let native_height_mm = LOGO_PNG_HEIGHT_PX * 25.4 / LOGO_DPI;
    let scale_x = LOGO_DISPLAY_MM / native_width_mm;
    let scale_y = LOGO_DISPLAY_MM / native_height_mm;

    ops.push(Op::UseXobject {
        id: logo_id.clone(),
        transform: XObjectTransform {
            translate_x: Some(Pt(241.0 * 72.0 / 25.4)),
            translate_y: Some(Pt(15.0 * 72.0 / 25.4)),
            scale_x: Some(scale_x),
            scale_y: Some(scale_y),
            dpi: Some(LOGO_DPI),
            rotate: None,
        },
    });
}

// ── All text ──────────────────────────────────────────────────────────────────

fn draw_all_text(
    label: &CrateLabel,
    ops: &mut Vec<Op>,
    regular: &PdfFontHandle,
    bold: &PdfFontHandle,
) {
    // Row 1 — header labels
    left_text(ops, "SKU NUMBER",  17.614212, 16.413929, regular, HEADER_LABEL_SIZE_MM);
    left_text(ops, "DESCRIPTION", 141.28918, 16.413929, regular, HEADER_LABEL_SIZE_MM);
    left_text(ops, "ORIGIN",      265.5477,  16.41394,  regular, HEADER_LABEL_SIZE_MM);

    // Row 1 — data values
    centered_text(ops, &label.sku_number.to_uppercase(),  30.340418, 34.91534, bold, VALUE_SIZE_MM);
    centered_text(ops, &label.description.to_uppercase(), 155.59077, 34.91534, bold, VALUE_SIZE_MM);
    centered_text(ops, &label.origin.to_uppercase(),      273.13312, 35.0,     bold, VALUE_SIZE_MM);

    // Row 2 — header labels
    left_text(ops, "CUSTOMER",     92.810844, 51.413925, regular, HEADER_LABEL_SIZE_MM);
    left_text(ops, "CUSTOMER SKU", 224.99377, 51.413929, regular, HEADER_LABEL_SIZE_MM);

    // Row 2 — data values
    centered_text(ops, &label.customer.to_uppercase(),     102.76622, 69.804832, bold, VALUE_SIZE_MM);
    centered_text(ops, &label.customer_sku.to_uppercase(), 240.27325, 69.797783, bold, VALUE_SIZE_MM);

    // Row 3 — header labels
    left_text(ops, "WAREHOUSE",  49.121277, 86.413933, regular, HEADER_LABEL_SIZE_MM);
    left_text(ops, "STOCK TYPE", 145.17348, 86.413933, regular, HEADER_LABEL_SIZE_MM);
    left_text(ops, "DATE",       240.67871, 86.41394,  regular, HEADER_LABEL_SIZE_MM);

    // Row 3 — data values
    let date_text = label.date.format("%-d %B %Y").to_string().to_uppercase();
    centered_text(ops, &label.warehouse_name.to_uppercase(),      60.753056, 104.91534, bold, VALUE_SIZE_MM);
    centered_text(ops, &label.location_stock_type.to_uppercase(), 158.17545, 104.91534, bold, VALUE_SIZE_MM);
    centered_text(ops, &date_text,                                 245.5414,  104.80484, bold, VALUE_SIZE_MM);

    // Row 4 — header labels
    left_text(ops, "QUANTITY IN CRATE", 20.858034, 121.41181, regular, HEADER_LABEL_SIZE_MM);
    left_text(ops, "PIECES IN CRATE",   90.898033, 121.41393, regular, HEADER_LABEL_SIZE_MM);
    left_text(ops, "WEIGHT OF CRATE",   161.09489, 121.41393, regular, HEADER_LABEL_SIZE_MM);
    left_text(ops, "CRATE NUMBER",      234.95992, 121.41393, regular, HEADER_LABEL_SIZE_MM);

    // Row 4 — data values
    let weight_text = format!("{} LBS", label.weight_per_crate_lbs);
    centered_text(ops, &label.square_footage_per_crate,    43.034294, 139.90121, bold, VALUE_SIZE_MM);
    centered_text(ops, &label.pieces_per_crate,            110.74601, 139.80484, bold, VALUE_SIZE_MM);
    centered_text(ops, &weight_text,                       180.56961, 139.91534, bold, VALUE_SIZE_MM);
    centered_text(ops, &label.crate_number.to_string(),    250.9612,  139.91534, bold, VALUE_SIZE_MM);

    // Row 5 — disclaimer header label
    left_text(ops, "DISCLAIMER", 15.690967, 156.41394, regular, HEADER_LABEL_SIZE_MM);

    // Row 5 — disclaimer body (pre-wrapped lines taken verbatim from SVG)
    let disclaimer_lines: &[(&str, f32)] = &[
        ("This label is intended solely for the use of the named recipient and contains confidential", 167.52942),
        ("information pertaining to the shipment of goods. The contents of this crate have been packed and", 172.46831),
        ("verified in accordance with the applicable purchase order. Any discrepancies in quantity,", 177.4072),
        ("condition, or identity of goods must be reported immediately to the warehouse manager upon", 182.34609),
        ("receipt. Unauthorised reproduction or distribution of this label is strictly prohibited. FBR", 187.28498),
        ("Marble accepts no liability for errors arising from illegible or damaged labels.", 192.22387),
    ];
    for (line, svg_y) in disclaimer_lines {
        left_text(ops, line, 15.813029, *svg_y, regular, DISCLAIMER_SIZE_MM);
    }
}

// ── Drawing primitives ────────────────────────────────────────────────────────

fn fill_rect(ops: &mut Vec<Op>, x_mm: f32, y_top_svg_mm: f32, w_mm: f32, h_mm: f32, color: Color) {
    let y_bottom_pdf_mm = PAGE_HEIGHT_MM - y_top_svg_mm - h_mm;
    ops.push(Op::SetFillColor { col: color });
    ops.push(Op::DrawPolygon {
        polygon: Polygon {
            rings: vec![PolygonRing {
                points: vec![
                    LinePoint { p: Point::new(Mm(x_mm),        Mm(y_bottom_pdf_mm)),        bezier: false },
                    LinePoint { p: Point::new(Mm(x_mm + w_mm), Mm(y_bottom_pdf_mm)),        bezier: false },
                    LinePoint { p: Point::new(Mm(x_mm + w_mm), Mm(y_bottom_pdf_mm + h_mm)), bezier: false },
                    LinePoint { p: Point::new(Mm(x_mm),        Mm(y_bottom_pdf_mm + h_mm)), bezier: false },
                ],
            }],
            mode: PaintMode::Fill,
            winding_order: WindingOrder::NonZero,
        },
    });
}

fn draw_line(ops: &mut Vec<Op>, x1_mm: f32, y1_svg_mm: f32, x2_mm: f32, y2_svg_mm: f32) {
    ops.push(Op::DrawLine {
        line: Line {
            points: vec![
                LinePoint { p: Point::new(Mm(x1_mm), Mm(PAGE_HEIGHT_MM - y1_svg_mm)), bezier: false },
                LinePoint { p: Point::new(Mm(x2_mm), Mm(PAGE_HEIGHT_MM - y2_svg_mm)), bezier: false },
            ],
            is_closed: false,
        },
    });
}

fn left_text(
    ops: &mut Vec<Op>,
    text: &str,
    x_mm: f32,
    baseline_svg_y_mm: f32,
    font: &PdfFontHandle,
    size_mm: f32,
) {
    let y_pdf_mm = PAGE_HEIGHT_MM - baseline_svg_y_mm;
    let size_pt = size_mm * 72.0 / 25.4;
    ops.extend([
        Op::SetFillColor { col: black() },
        Op::StartTextSection,
        Op::SetTextCursor { pos: Point::new(Mm(x_mm), Mm(y_pdf_mm)) },
        Op::SetFont { font: font.clone(), size: Pt(size_pt) },
        Op::SetLineHeight { lh: Pt(size_pt) },
        Op::ShowText { items: vec![TextItem::Text(text.to_string())] },
        Op::EndTextSection,
    ]);
}

fn centered_text(
    ops: &mut Vec<Op>,
    text: &str,
    center_x_mm: f32,
    baseline_svg_y_mm: f32,
    font: &PdfFontHandle,
    size_mm: f32,
) {
    let text_width_mm = text.len() as f32 * CHAR_ADVANCE_FRACTION * size_mm;
    let x_mm = center_x_mm - text_width_mm / 2.0;
    left_text(ops, text, x_mm, baseline_svg_y_mm, font, size_mm);
}

fn black() -> Color {
    Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None })
}

fn gray(value: f32) -> Color {
    Color::Rgb(Rgb { r: value, g: value, b: value, icc_profile: None })
}
