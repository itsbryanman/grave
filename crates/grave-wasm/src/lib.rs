use grave_core::{
    decay_snapshot, inspect_grave, prognosis, render_grave, GraveHeader, RenderedPayload,
    TERMINAL_Q,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn read_header(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let inspection = inspect_grave(bytes).map_err(js_error)?;
    serde_wasm_bindgen::to_value(&HeaderResponse::from_inspection(inspection))
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen]
pub fn render_at(bytes: &[u8], timestamp: u64) -> Result<JsValue, JsValue> {
    let inspection = inspect_grave(bytes).map_err(js_error)?;
    let prognosis_at = prognosis(&inspection.header, timestamp).map_err(js_error)?;
    let snapshot = if inspection.disturbed {
        grave_core::DecaySnapshot {
            age_days: day_delta(inspection.header.buried_at, timestamp),
            neglect_days: day_delta(inspection.header.last_opened, timestamp),
            effective_half_life_days: inspection.header.half_life_days as f64,
            intensity: 1.0,
            q: 10_000,
        }
    } else {
        decay_snapshot(&inspection.header, timestamp).map_err(js_error)?
    };

    if !inspection.disturbed && snapshot.q >= TERMINAL_Q {
        return serde_wasm_bindgen::to_value(&RenderResponse {
            header: HeaderView::from_header(&inspection.header),
            disturbed: false,
            prognosis: prognosis_at,
            q: snapshot.q,
            intensity: snapshot.intensity,
            age_days: snapshot.age_days,
            neglect_days: snapshot.neglect_days,
            payload: RenderPayload::Terminal,
        })
        .map_err(|error| JsValue::from_str(&error.to_string()));
    }

    let render = render_grave(bytes, timestamp).map_err(js_error)?;
    let payload = match render.payload {
        RenderedPayload::Image(image) => RenderPayload::Image {
            rgba: image.rgba,
            width: image.width,
            height: image.height,
        },
        RenderedPayload::Text(text) => RenderPayload::Text {
            text: text.body,
            is_hex_dump: text.is_hex_dump,
        },
    };

    serde_wasm_bindgen::to_value(&RenderResponse {
        header: HeaderView::from_header(&render.header),
        disturbed: render.disturbed,
        prognosis: prognosis_at,
        q: render.snapshot.q,
        intensity: render.snapshot.intensity,
        age_days: render.snapshot.age_days,
        neglect_days: render.snapshot.neglect_days,
        payload,
    })
    .map_err(|error| JsValue::from_str(&error.to_string()))
}

fn js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[derive(Serialize)]
struct HeaderResponse {
    header: HeaderView,
    disturbed: bool,
    compressed_len: u64,
    original_len: u64,
}

impl HeaderResponse {
    fn from_inspection(inspection: grave_core::GraveInspection) -> Self {
        Self {
            header: HeaderView::from_header(&inspection.header),
            disturbed: inspection.disturbed,
            compressed_len: inspection.compressed_len,
            original_len: inspection.original_len,
        }
    }
}

#[derive(Serialize)]
struct RenderResponse {
    header: HeaderView,
    disturbed: bool,
    prognosis: u64,
    q: u32,
    intensity: f64,
    age_days: u64,
    neglect_days: u64,
    payload: RenderPayload,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum RenderPayload {
    Terminal,
    Image {
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    },
    Text {
        text: String,
        is_hex_dump: bool,
    },
}

#[derive(Serialize)]
struct HeaderView {
    version: u16,
    burial_id: String,
    buried_at: u64,
    last_opened: u64,
    open_count: u32,
    profile: &'static str,
    hardcore: bool,
    mourned_recently: bool,
    half_life_days: u32,
    mourn_credit: u32,
    epitaph: String,
    original_filename: String,
    mimetype: String,
}

impl HeaderView {
    fn from_header(header: &GraveHeader) -> Self {
        Self {
            version: header.version,
            burial_id: hex_string(&header.burial_id),
            buried_at: header.buried_at,
            last_opened: header.last_opened,
            open_count: header.open_count,
            profile: header.profile.label(),
            hardcore: header.flags.hardcore(),
            mourned_recently: header.flags.mourned_recently(),
            half_life_days: header.half_life_days,
            mourn_credit: header.mourn_credit,
            epitaph: header.epitaph.clone(),
            original_filename: header.original_filename.clone(),
            mimetype: header.mimetype.clone(),
        }
    }
}

fn hex_string(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn day_delta(earlier: u64, later: u64) -> u64 {
    later
        .saturating_div(grave_core::DAY_SECONDS)
        .saturating_sub(earlier.saturating_div(grave_core::DAY_SECONDS))
}
