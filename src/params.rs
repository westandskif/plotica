/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::data_set::{DataPoint, DataSet};
use crate::screen::Size;
use chrono::prelude::*;
use js_sys::Reflect;
use std::str::{from_utf8_unchecked, FromStr};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Copy, Clone)]
pub enum DataType {
    Number,
    DateTime { tz_offset: FixedOffset },
    Date,
}
impl FromStr for DataType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "number" => Ok(DataType::Number),
            "date" => Ok(DataType::Date),
            "datetime" => {
                let tz_offset_ms = js_sys::Date::new_0().get_timezone_offset() as i32 * 60;
                Ok(DataType::DateTime {
                    tz_offset: FixedOffset::west_opt(tz_offset_ms)
                        .ok_or_else(|| "invalid timezone offset".to_string())?,
                })
            }
            v => Err(format!(
                "invalid data type: '{}'; use 'number' or 'date'",
                v
            )),
        }
    }
}
impl DataType {
    pub fn get_min_period(&self) -> Option<f64> {
        match self {
            Self::Number => None,
            Self::DateTime { .. } => None,
            Self::Date => Some(86400000.0),
        }
    }
}

const SUFFIXES: [&'static str; 4] = ["", "K", "M", "B"];

#[derive(Clone)]
pub enum VerboseFormat {
    Number {
        precision: usize,
        scale: usize,
    },
    NumberConcise,
    Date {
        fmt_str: String,
    },
    DateTime {
        fmt_str: String,
        tz_offset: FixedOffset,
    },
}
impl VerboseFormat {
    pub fn from_data_type(
        data_type: &DataType,
        chart_config: &ChartConfig,
        concise: bool,
    ) -> VerboseFormat {
        match data_type {
            DataType::Date => VerboseFormat::Date {
                fmt_str: "%b %d, %Y".to_string(),
            },
            DataType::DateTime { tz_offset } => VerboseFormat::DateTime {
                fmt_str: "%b %d, %Y %H:%M:%S".to_string(),
                tz_offset: *tz_offset,
            },
            DataType::Number => {
                if concise {
                    Self::NumberConcise
                } else {
                    Self::Number {
                        precision: chart_config.exp_fmt_significant_digits,
                        scale: chart_config.exp_fmt_significant_digits - 1,
                    }
                }
            }
        }
    }
    pub fn format_values<T, U, F>(
        &self,
        values: T,
        getter: F,
        min_value: f64,
        max_value: f64,
    ) -> Vec<String>
    where
        T: Iterator<Item = U>,
        F: Fn(U) -> f64,
    {
        match self {
            Self::Date { fmt_str } => values
                .map(getter)
                .map(|value| {
                    DateTime::from_timestamp_millis(value as i64)
                        .unwrap()
                        .format(fmt_str)
                        .to_string()
                })
                .collect(),
            Self::DateTime { fmt_str, tz_offset } => values
                .map(getter)
                .map(|value| {
                    DateTime::from_timestamp(value as i64, 0).unwrap().with_timezone(tz_offset)
                        .format(fmt_str)
                        .to_string()
                })
                .collect(),
            Self::Number {precision, scale} => values
                .map(getter)
                .map(|value| {
                    let value_abs = value.abs();
                    if value_abs > 1e12 || value_abs < 1e-3 {
                        format!("{:precision$.scale$e}", value, precision = precision, scale = scale)
                    } else {
                        let integral = (value.abs() as i64).to_string();
                        let parts: Vec<&str> = integral.as_bytes().rchunks(3).map(|b| unsafe {from_utf8_unchecked(b)}).rev().collect();

                        let formatted_value = format!("{:.scale$}", value, scale = scale);
                        // let formatted_value = value.to_string();
                        match formatted_value.trim_end_matches("0").split_once(".") {
                            Some((_, right)) => {
                                if right.len() > 0 {
                                    format!{"{}{}.{}", if value < 0.0 {"-"} else {""}, parts.join(","), right}
                                } else {
                                    format!{"{}{}", if value < 0.0 {"-"} else {""}, parts.join(",")}
                                }
                            }
                            None => {
                                format!{"{}{}", if value < 0.0 {"-"} else {""}, parts.join(",")}
                            }
                        }
                    }

                })
                .collect(),
            Self::NumberConcise => {
                if min_value < -1e12 || max_value > 1e12 {
                    values
                        .map(getter)
                        .map(|value| format!("{:3.2e}", value))
                        .collect()
                } else {
                    values
                        .map(getter)
                        .map(|value| {
                            let mut index = 0;
                            let mut value_abs = value.abs();
                            if value_abs < 1e-12 {
                                format!("{:3.2e}", value)
                            } else {
                                while value_abs >= 1000.0 {
                                    index += 1;
                                    value_abs *= 0.001
                                }
                                if value_abs < 10.0 {
                                    unsafe {
                                        format!(
                                            "{:.2}{}",
                                            value_abs * value.signum(),
                                            SUFFIXES.get_unchecked(index)
                                        )
                                    }
                                } else if value_abs < 100.0 {
                                    unsafe {
                                        format!(
                                            "{:.1}{}",
                                            value_abs * value.signum(),
                                            SUFFIXES.get_unchecked(index)
                                        )
                                    }
                                } else {
                                    unsafe {
                                        format!(
                                            "{:.0}{}",
                                            value_abs * value.signum(),
                                            SUFFIXES.get_unchecked(index)
                                        )
                                    }
                                }
                            }
                        })
                        .collect()
                }
            }
        }
    }
}

pub fn js_value_to_f64<O: Fn() -> String>(value: &JsValue, path: &O) -> Result<f64, String> {
    if let Some(v) = value.as_f64() {
        return Ok(v);
    }
    let string_value = value
        .as_string()
        .ok_or_else(|| format!("neither a number nor a string: {}", path()))?;

    f64::from_str(string_value.as_str())
        .map_err(|_| format!("failed to parse as f64: {}", path()))
        .and_then(|v| {
            if v.is_finite() {
                Ok(v)
            } else {
                Err(format!(
                    "inf values are not supported: {}",
                    string_value.as_str()
                ))
            }
        })
}
fn js_value_to_date_as_f64<O: Fn() -> String>(value: &JsValue, path: &O) -> Result<f64, String> {
    let value = match value.clone().dyn_into::<js_sys::Date>() {
        Ok(dt) => dt.value_of(),
        Err(item) => js_sys::Date::new(&item).value_of(),
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{} not a date", path()))
    }
}
fn js_value_to_u8<O: Fn() -> String>(value: &JsValue, path: &O) -> Result<u8, String> {
    if let Some(v) = value.as_f64() {
        if v < 0.0 || v > 255.0 {
            Err(format!("should be 0-255: {}", path()))
        } else {
            Ok(v as u8)
        }
    } else {
        let string_value = value
            .as_string()
            .ok_or_else(|| format!("neither a number nor a string: {}", path()))?;
        u8::from_str(string_value.as_str())
            .map_err(|_| format!("failed to parse as u8: {}", path()))
    }
}
fn js_value_to_rgb<O: Fn() -> String>(value: &JsValue, path: &O) -> Result<(u8, u8, u8), String> {
    let items: Vec<JsValue> = value
        .clone()
        .dyn_into::<js_sys::Array>()
        .map_err(|_| format!("not an array: {}", path()))?
        .iter()
        .collect();

    if items.len() != 3 {
        return Err(format!("color is an array of length 3: {}", path()));
    }

    Ok((
        js_value_to_u8(&items[0], &|| format!("{}.{}", path(), 0))?,
        js_value_to_u8(&items[1], &|| format!("{}.{}", path(), 1))?,
        js_value_to_u8(&items[2], &|| format!("{}.{}", path(), 2))?,
    ))
}
fn get_by_str_key<O: Fn() -> String>(
    obj: &JsValue,
    key: &str,
    path: &O,
) -> Result<JsValue, String> {
    Reflect::get(obj, &JsValue::from_str(key))
        .map_err(|_| format!("not an object to fetch: '{}'", path()))
}

fn get_string_by_str_key<O: Fn() -> String>(
    obj: &JsValue,
    key: &str,
    path: &O,
) -> Result<String, String> {
    get_by_str_key(obj, key, path)?
        .as_string()
        .ok_or_else(|| format!("{} is missing", path()))
}

// fn get_bool_by_str_key<O: Fn() -> String>(
//     obj: &JsValue,
//     key: &str,
//     path: &O,
// ) -> Result<bool, String> {
//     get_by_str_key(obj, key, path)?
//         .as_bool()
//         .ok_or_else(|| format!("not a bool: {}", path().as_str()))
// }

fn get_f64_by_str_key<O: Fn() -> String>(
    obj: &JsValue,
    key: &str,
    path: &O,
) -> Result<f64, String> {
    js_value_to_f64(&get_by_str_key(obj, key, path)?, path)
}

fn get_u8_by_str_key<O: Fn() -> String>(obj: &JsValue, key: &str, path: &O) -> Result<u8, String> {
    js_value_to_u8(&get_by_str_key(obj, key, path)?, path)
}

fn get_array_by_str_key<O: Fn() -> String>(
    obj: &JsValue,
    key: &str,
    path: &O,
) -> Result<js_sys::Array, String> {
    get_by_str_key(obj, key, path)?
        .dyn_into::<js_sys::Array>()
        .map_err(|_| format!("not an array: {}", path()))
}

fn get_rgb_by_str_key<O: Fn() -> String>(
    obj: &JsValue,
    key: &str,
    path: &O,
) -> Result<(u8, u8, u8), String> {
    js_value_to_rgb(&get_by_str_key(obj, key, path)?, path)
}

fn get_rgba_by_str_key<O: Fn() -> String>(
    obj: &JsValue,
    key: &str,
    path: &O,
) -> Result<(u8, u8, u8, f64), String> {
    let items: Vec<JsValue> = get_array_by_str_key(obj, key, path)?.iter().collect();
    if items.len() != 4 {
        return Err(format!("color is an array of length 4: {}", path()));
    }
    Ok((
        js_value_to_u8(&items[0], &|| format!("{}.0", path()))?,
        js_value_to_u8(&items[1], &|| format!("{}.1", path()))?,
        js_value_to_u8(&items[2], &|| format!("{}.2", path()))?,
        js_value_to_f64(&items[3], &|| format!("{}.3", path()))?,
    ))
}

pub fn parse_js_values<O: Fn() -> String>(
    value: js_sys::Array,
    data_type: DataType,
    path: &O,
) -> Result<Vec<f64>, String> {
    let mut result: Vec<f64> = Vec::with_capacity(value.length() as usize);
    match data_type {
        DataType::Number => {
            for (index, item) in value.iter().enumerate() {
                result.push(js_value_to_f64(&item, &|| format!("{}.{}", path(), index))?);
            }
        }
        DataType::Date => {
            for (index, item) in value.iter().enumerate() {
                result.push(js_value_to_date_as_f64(&item, &|| {
                    format!("{}.{}", path(), index)
                })?);
            }
        }
        DataType::DateTime { .. } => {
            for (index, item) in value.iter().enumerate() {
                result.push(js_value_to_date_as_f64(&item, &|| {
                    format!("{}.{}", path(), index)
                })?);
            }
        }
    }
    Ok(result)
}

pub struct Content {
    pub name: Option<String>,
    pub coord_type: DataType,
    pub coord_verbose_format: VerboseFormat,
    pub coord_verbose_format_short: VerboseFormat,
    pub coord_short_verbose_len: usize,
    pub value_type: DataType,
    pub value_verbose_format: VerboseFormat,
    pub value_verbose_format_short: VerboseFormat,
    pub value_short_verbose_len: usize,
    pub data_sets: Vec<DataSet>,
    pub global_coord_min: f64,
    pub global_coord_max: f64,
    pub global_value_min: f64,
    pub global_value_max: f64,
}
// TODO: panic on empty or zero height data
impl Content {
    pub fn new(
        name: Option<String>,
        coord_type: DataType,
        value_type: DataType,
        chart_config: &ChartConfig,
    ) -> Content {
        Content {
            name,
            coord_type,
            coord_verbose_format: VerboseFormat::from_data_type(&coord_type, chart_config, false),
            coord_verbose_format_short: VerboseFormat::from_data_type(
                &coord_type,
                chart_config,
                true,
            ),
            coord_short_verbose_len: 0,
            value_type,
            value_verbose_format: VerboseFormat::from_data_type(&value_type, chart_config, false),
            value_verbose_format_short: VerboseFormat::from_data_type(
                &value_type,
                chart_config,
                true,
            ),
            value_short_verbose_len: 0,
            data_sets: Vec::new(),
            global_coord_min: f64::MAX,
            global_coord_max: f64::MIN,
            global_value_min: f64::MAX,
            global_value_max: f64::MIN,
        }
    }
    pub fn parse_and_add_data_set(
        &mut self,
        name: &str,
        coords: Vec<f64>,
        values: Vec<f64>,
        rgb: (u8, u8, u8),
    ) -> Result<(), String> {
        if coords.len() != values.len() {
            return Err(format!(
                "coords and values have different lengths: {}",
                name
            ));
        }
        if coords.is_empty() {
            return Err(format!("data set is empty: {}", name));
        }
        if self.data_sets.iter().any(|item| item.name == name) {
            return Err(format!("duplicate data set name: {}", name));
        }
        let mut data_points: Vec<DataPoint> = coords
            .into_iter()
            .zip(values)
            .map(|(coord, value)| DataPoint { coord, value })
            .collect();

        data_points.sort_by(|p1, p2| p1.coord.partial_cmp(&p2.coord).unwrap());
        for (index, (current, next)) in data_points
            .iter()
            .zip(data_points.iter().skip(1))
            .enumerate()
        {
            if current.coord == next.coord {
                return Err(format!(
                    "data set '{}' - duplicate coordinate found at index: {}",
                    name,
                    index + 1
                ));
            }
        }
        let data_set = DataSet::new(name, rgb, data_points);
        self.coord_short_verbose_len = self.coord_short_verbose_len.max(
            self.coord_verbose_format_short
                .format_values(
                    data_set.data_points.iter().take(30),
                    |p| p.coord,
                    data_set.data_points.get(0).unwrap().coord,
                    data_set
                        .data_points
                        .get(data_set.data_points.len() - 1)
                        .unwrap()
                        .coord,
                )
                .into_iter()
                .map(|s| s.chars().count())
                .max()
                .unwrap(),
        );
        self.value_short_verbose_len = self.value_short_verbose_len.max(
            self.value_verbose_format_short
                .format_values(
                    data_set.data_points.iter().take(30),
                    |p| p.value,
                    data_set.meta.min,
                    data_set.meta.max,
                )
                .into_iter()
                .map(|s| s.chars().count())
                .max()
                .unwrap(),
        );
        self.global_coord_min = self.global_coord_min.min(data_set.data_points[0].coord);
        self.global_coord_max = self
            .global_coord_max
            .max(data_set.data_points[data_set.data_points.len() - 1].coord);
        self.global_value_min = self.global_value_min.min(data_set.meta.min);
        self.global_value_max = self.global_value_max.max(data_set.meta.max);
        self.data_sets.push(data_set);
        Ok(())
    }

    pub fn sort_data_sets(&mut self, strategy: &DataSetSorting) {
        match strategy {
            DataSetSorting::MaxAsc => {
                self.data_sets
                    .sort_by(|a, b| a.meta.max.partial_cmp(&b.meta.max).unwrap());
            }
            DataSetSorting::MaxDesc => {
                self.data_sets
                    .sort_by(|a, b| b.meta.max.partial_cmp(&a.meta.max).unwrap());
            }
            DataSetSorting::MinAsc => {
                self.data_sets
                    .sort_by(|a, b| a.meta.min.partial_cmp(&b.meta.min).unwrap());
            }
            DataSetSorting::MinDesc => {
                self.data_sets
                    .sort_by(|a, b| b.meta.min.partial_cmp(&a.meta.min).unwrap());
            }
            DataSetSorting::MedianAsc => {
                self.data_sets
                    .sort_by(|a, b| a.meta.p50.partial_cmp(&b.meta.p50).unwrap());
            }
            DataSetSorting::MedianDesc => {
                self.data_sets
                    .sort_by(|a, b| b.meta.p50.partial_cmp(&a.meta.p50).unwrap());
            }
            DataSetSorting::None => {}
        }
    }
    pub fn get_min_max(&mut self) -> [f64; 4] {
        let mut coord_min: f64 = f64::MAX;
        let mut coord_max: f64 = f64::MIN;
        let mut value_min: f64 = f64::MAX;
        let mut value_max: f64 = f64::MIN;
        for data_set in self.data_sets.iter_mut() {
            if data_set.alpha.get_end_value() > 0.0 {
                coord_min = coord_min.min(data_set.data_points[0].coord);
                coord_max =
                    coord_min.max(data_set.data_points[data_set.data_points.len() - 1].coord);
                for data_point in data_set.data_points.iter() {
                    value_min = value_min.min(data_point.value);
                    value_max = value_max.max(data_point.value);
                }
            }
        }
        [coord_min, coord_max, value_min, value_max]
    }
}

pub enum DataSetSorting {
    MaxAsc,
    MaxDesc,
    MinAsc,
    MinDesc,
    MedianAsc,
    MedianDesc,
    None,
}
impl FromStr for DataSetSorting {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "maxAsc" => Ok(Self::MaxAsc),
            "maxDesc" => Ok(Self::MaxDesc),
            "minAsc" => Ok(Self::MinAsc),
            "minDesc" => Ok(Self::MinDesc),
            "medianAsc" => Ok(Self::MedianAsc),
            "medianDesc" => Ok(Self::MedianDesc),
            "none" => Ok(Self::None),
            v => Err(format!("unsupported DataSetSorting strategy: {}", v)),
        }
    }
}

pub struct ChartConfig {
    pub font_standard: String,
    pub font_monospace: String,
    pub font_size_small: Size,
    pub font_size_normal: Size,
    pub font_size_large: Size,
    pub font_width_coeff: f64,
    pub line_width: Size,
    pub circle_diameter: Size,
    pub color_grid: (u8, u8, u8),
    pub color_tick: (u8, u8, u8),
    pub color_camera_grip: (u8, u8, u8, f64),
    pub color_preview_overlay: (u8, u8, u8, f64),
    pub color_preview_hint: (u8, u8, u8, f64),
    pub color_tooltip: (u8, u8, u8, f64),
    pub color_tooltip_font: (u8, u8, u8, f64),
    pub sort_data_sets_by: DataSetSorting,
    pub layout_content_height: f64,
    pub layout_preview_height: f64,
    pub layout_legend_height: f64,
    pub color_palette: Vec<(u8, u8, u8)>,
    pub us_long_press: f64,
    pub auto_log_scale_threshold: f64,
    pub exp_fmt_significant_digits: usize,
}
impl ChartConfig {
    pub fn from_raw(raw_config: &JsValue) -> Result<Self, String> {
        let layout_content_height = get_f64_by_str_key(raw_config, "layoutContentHeight", &|| {
            "layoutContentHeight".to_string()
        })?;
        let layout_preview_height = get_f64_by_str_key(raw_config, "layoutPreviewHeight", &|| {
            "layoutPreviewHeight".to_string()
        })?;
        let layout_legend_height = get_f64_by_str_key(raw_config, "layoutLegendHeight", &|| {
            "layoutLegendHeight".to_string()
        })?;
        let total_height_norm =
            (layout_content_height + layout_preview_height + layout_legend_height).recip();

        let color_palette: Result<Vec<(u8, u8, u8)>, String> =
            get_array_by_str_key(raw_config, "colorPalette", &|| "colorPalette".to_string())?
                .iter()
                .enumerate()
                .map(|(index, item)| js_value_to_rgb(&item, &|| format!("colorPalette.{}", index)))
                .collect();

        Ok(Self {
            font_standard: get_string_by_str_key(raw_config, "fontStandard", &|| {
                "fontStandard".to_string()
            })?,
            font_monospace: get_string_by_str_key(raw_config, "fontMonospace", &|| {
                "fontMonospace".to_string()
            })?,
            font_size_small: Size::TextLine {
                font_size: get_f64_by_str_key(raw_config, "fontSizeSmall", &|| {
                    "fontSizeSmall".to_string()
                })?,
                columns: 1.0,
            },
            font_size_normal: Size::TextLine {
                font_size: get_f64_by_str_key(raw_config, "fontSizeNormal", &|| {
                    "fontSizeNormal".to_string()
                })?,
                columns: 1.0,
            },
            font_size_large: Size::TextLine {
                font_size: get_f64_by_str_key(raw_config, "fontSizeLarge", &|| {
                    "fontSizeLarge".to_string()
                })?,
                columns: 1.0,
            },
            font_width_coeff: get_f64_by_str_key(raw_config, "fontWidthCoeff", &|| {
                "fontWidthCoeff".to_string()
            })?,
            line_width: Size::Px(get_f64_by_str_key(raw_config, "lineWidth", &|| {
                "lineWidth".to_string()
            })?),
            circle_diameter: Size::Px(
                get_f64_by_str_key(raw_config, "circleRadius", &|| "circleRadius".to_string())?
                    * 2.0,
            ),
            color_grid: get_rgb_by_str_key(raw_config, "colorGrid", &|| "colorGrid".to_string())?,
            color_tick: get_rgb_by_str_key(raw_config, "colorTick", &|| "colorTick".to_string())?,
            color_camera_grip: get_rgba_by_str_key(raw_config, "colorCameraGrip", &|| {
                "colorCameraGrip".to_string()
            })?,
            color_preview_overlay: get_rgba_by_str_key(raw_config, "colorPreviewOverlay", &|| {
                "colorPreviewOverlay".to_string()
            })?,
            color_preview_hint: get_rgba_by_str_key(raw_config, "colorPreviewHint", &|| {
                "colorPreviewHint".to_string()
            })?,
            color_tooltip: get_rgba_by_str_key(raw_config, "colorTooltip", &|| {
                "colorTooltip".to_string()
            })?,
            color_tooltip_font: get_rgba_by_str_key(raw_config, "colorTooltipFont", &|| {
                "colorTooltipFont".to_string()
            })?,
            sort_data_sets_by: DataSetSorting::from_str(&get_string_by_str_key(
                raw_config,
                "sortDataSetsBy",
                &|| "sortDataSetsBy".to_string(),
            )?)?,
            layout_content_height: layout_content_height * total_height_norm,
            layout_preview_height: layout_preview_height * total_height_norm,
            layout_legend_height: layout_legend_height * total_height_norm,
            color_palette: color_palette?,
            us_long_press: get_f64_by_str_key(raw_config, "msLongPress", &|| {
                "msLongPress".to_string()
            })? * 1000.0,
            auto_log_scale_threshold: get_f64_by_str_key(
                raw_config,
                "autoLogScaleThreshold",
                &|| "autoLogScaleThreshold".to_string(),
            )?,
            exp_fmt_significant_digits: get_u8_by_str_key(
                raw_config,
                "expFmtSignificantDigits",
                &|| "expFmtSignificantDigits".to_string(),
            )? as usize,
        })
    }
}

pub struct ChartParams {
    pub selector: String,
    pub content: Content,
}

impl ChartParams {
    pub fn from(raw_params: &JsValue, chart_config: &ChartConfig) -> Result<Self, String> {
        let content_name =
            get_string_by_str_key(raw_params, "contentName", &|| "contentName".to_string()).ok();
        let selector = get_string_by_str_key(raw_params, "selector", &|| "selector".to_string())?;

        let coord_type = DataType::from_str(
            get_string_by_str_key(raw_params, "coordType", &|| "coordType".to_string())?.as_str(),
        )?;
        let value_type = DataType::from_str(
            get_string_by_str_key(raw_params, "valueType", &|| "valueType".to_string())?.as_str(),
        )?;

        let mut content = Content::new(content_name, coord_type, value_type, chart_config);

        let color_palette = &chart_config.color_palette;
        let colors_number = color_palette.len();

        for (index, raw_data_set) in
            get_by_str_key(&raw_params, "dataSets", &|| "dataSets".to_string())?
                .dyn_into::<js_sys::Array>()
                .map_err(|_| "dataSets is not an array".to_string())?
                .iter()
                .enumerate()
        {
            let data_set_name = get_string_by_str_key(&raw_data_set, "name", &|| {
                format!("dataSets[{}].name", index)
            })?;

            let coords = get_array_by_str_key(&raw_data_set, "coords", &|| {
                format!("dataSets[{}].coords", index)
            })?;
            let coords = parse_js_values(coords, coord_type, &|| {
                format!("dataSets[{}].coords", index)
            })?;
            let values = get_array_by_str_key(&raw_data_set, "values", &|| {
                format!("dataSets[{}].values", index)
            })?;
            let values = parse_js_values(values, value_type, &|| {
                format!("dataSets[{}].values", index)
            })?;

            let color = color_palette[index % colors_number];

            content.parse_and_add_data_set(data_set_name.as_str(), coords, values, color)?;
        }
        Ok(ChartParams { selector, content })
    }
}
#[derive(Debug, Clone)]
pub struct ClientCaps {
    pub touch_device: bool,
    pub device_pixel_ratio: f64,
    pub css_to_physical_scale: f64,
    pub screen_orientation: bool,
}
impl ClientCaps {
    pub fn detect() -> Self {
        let window = web_sys::window().unwrap();
        let touch_device = !Reflect::get(&window, &JsValue::from_str("ontouchstart"))
            .unwrap()
            .is_undefined()
            && window.navigator().max_touch_points() > 0;
        let device_pixel_ratio = window.device_pixel_ratio();
        let visual_viewport = Reflect::get(&window, &JsValue::from_str("visualViewport")).unwrap();
        let viewport_scale = Reflect::get(&visual_viewport, &JsValue::from_str("scale"))
            .unwrap()
            .as_f64()
            .unwrap();
        let css_to_physical_scale = viewport_scale * device_pixel_ratio;

        let screen_orientation = !Reflect::get(&window, &JsValue::from_str("screen"))
            .and_then(|screen| Reflect::get(&screen, &JsValue::from_str("orientation")))
            .unwrap()
            .is_undefined();
        Self {
            touch_device,
            device_pixel_ratio,
            css_to_physical_scale,
            screen_orientation,
        }
    }
}
