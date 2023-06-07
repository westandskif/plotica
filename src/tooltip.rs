/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::animate::AnimatedNumber;
use crate::data_set::{DataPoint, DataSet};
use crate::params::{ChartConfig, Content};
use crate::scale::Scale;
use crate::screen::{Screen, ScreenArea};
use crate::utils::place_rect_inside;
use std::f64::consts::PI;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

pub struct Tooltip {
    pub chart_config: Rc<ChartConfig>,
    min_width: AnimatedNumber,
    pub visible: bool,
    pub mouse_click_at: Option<(f64, f64)>,
}

impl Tooltip {
    pub fn new(chart_config: Rc<ChartConfig>) -> Self {
        Self {
            chart_config,
            min_width: AnimatedNumber::custom(0.0, 500000.0, 500000.0),
            visible: false,
            mouse_click_at: None,
        }
    }

    pub fn draw<'a, T>(
        &'a mut self,
        content: &mut Content,
        screen: &mut Screen,
        screen_area: &ScreenArea<T>,
        mut mouse_position: &'a Option<(f64, f64)>,
        time_us: f64,
    ) where
        T: Scale,
    {
        if self.mouse_click_at.is_some() {
            mouse_position = &self.mouse_click_at;
        }
        let data = match mouse_position {
            Some((x, y)) => match (screen_area.x_to_coord(*x), screen_area.y_to_value(*y)) {
                (Some(coord_), Some(value_)) => Some((
                    coord_,
                    value_,
                    screen_area.x_to_cx(*x),
                    screen_area.y_to_cy(*y),
                )),
                _ => None,
            },
            _ => None,
        };

        if data.is_none() {
            self.visible = false;
            return;
        }
        let (coord, value, pointer_cx, pointer_cy) = data.unwrap();
        self.visible = true;

        let mut max_coord: f64 = f64::MIN;
        let mut left_matches: Vec<(&DataSet, &DataPoint)> =
            Vec::with_capacity(content.data_sets.len());
        for data_set in content.data_sets.iter() {
            if data_set.alpha.get_end_value() == 0.0 {
                continue;
            }
            if let Some(index) = data_set.bin_search_right_bound(coord) {
                let data_point = &data_set.data_points[index];
                if max_coord < data_point.coord {
                    max_coord = data_point.coord;
                }
                left_matches.push((data_set, data_point));
            }
        }
        left_matches.retain(|m| m.1.coord == max_coord);

        if left_matches.len() == 0 {
            return;
        }

        let mut min_coord: f64 = f64::MAX;
        let mut right_matches: Vec<(&DataSet, &DataPoint)> =
            Vec::with_capacity(content.data_sets.len());
        for data_set in content.data_sets.iter() {
            if data_set.alpha.get_end_value() == 0.0 {
                continue;
            }
            if let Some(index) = data_set.bin_search_left_bound(coord) {
                let data_point = &data_set.data_points[index];
                if min_coord > data_point.coord {
                    min_coord = data_point.coord;
                }
                right_matches.push((data_set, data_point));
            }
        }
        right_matches.retain(|m| m.1.coord == min_coord);

        let cx_step_size = screen_area.get_cx(min_coord) - screen_area.get_cx(max_coord);
        let matched_coord: f64;
        let mut matches = if (coord - min_coord).abs() < (coord - max_coord).abs() {
            matched_coord = min_coord;
            right_matches
        } else {
            matched_coord = max_coord;
            left_matches
        };

        let mut min_diff: f64 = f64::MAX;
        let mut index_with_min_diff_by_value: usize = 0;
        for (index, (_, data_point)) in matches.iter().cloned().enumerate() {
            let diff = (data_point.value - value).abs();
            if min_diff > diff {
                min_diff = diff;
                index_with_min_diff_by_value = index;
            }
        }
        drop(min_diff);

        let coord_format = &content.coord_verbose_format;
        let value_format = &content.value_verbose_format;

        let formatted_coord = coord_format
            .format_values(
                Some(matched_coord).into_iter(),
                |x| x,
                screen_area.global_scale.get_coord_min(),
                screen_area.global_scale.get_coord_max(),
            )
            .into_iter()
            .next()
            .unwrap();

        let max_name_length = matches
            .iter()
            .cloned()
            .map(|t| t.0.name.len())
            .max()
            .unwrap();
        let formatted_values = value_format.format_values(
            matches.iter().cloned(),
            |t| t.1.value,
            screen_area.global_scale.get_value_min(),
            screen_area.global_scale.get_value_max(),
        );
        let max_formatted_value_length: usize =
            formatted_values.iter().map(|v| v.len()).max().unwrap();

        let context = &screen.context;

        let c_line_width = screen.apx_to_cpx(1.0);
        let c_padding: f64 = screen.apx_to_cpx(5.0);
        let c_additional_gap_after_heading: f64 = screen.apx_to_cpx(10.0);
        let c_gap_between_lines: f64 = screen.apx_to_cpx(2.0);
        let c_gap_between_colors_n_names: f64 = screen.apx_to_cpx(5.0);
        let c_gap_between_names_n_values: f64 = screen.apx_to_cpx(5.0);
        let c_heading_lines: usize = 1;
        let c_font_size: f64 = screen.apx_to_cpx(self.chart_config.font_size_normal);
        let c_color_size: f64 = c_font_size;
        let c_expected_tooltip_shift_x: f64 = screen.apx_to_cpx(25.0).max(cx_step_size * 0.125);
        let c_font_width = c_font_size * self.chart_config.font_width_coeff;
        let c_heading_width = content.coord_short_verbose_len as f64 * c_font_width;

        let mut tooltip_width = c_heading_width.max(
            c_color_size
                + c_gap_between_colors_n_names
                + (max_name_length + max_formatted_value_length) as f64 * c_font_width
                + c_gap_between_names_n_values,
        ) + c_padding * 2.0;
        let tooltip_min_width = self.min_width.get_value(time_us);
        if tooltip_width > tooltip_min_width {
            self.min_width.set_value(tooltip_width, None);
        }
        if tooltip_width < tooltip_min_width {
            if tooltip_width < self.min_width.get_end_value() {
                self.min_width.set_value(tooltip_width, Some(time_us));
            }
            tooltip_width = tooltip_min_width;
        }

        let tooltip_height = (formatted_values.len() + c_heading_lines) as f64 * c_font_size
            + (formatted_values.len() + c_heading_lines - 1) as f64 * c_gap_between_lines
            + c_additional_gap_after_heading
            + c_padding * 2.0;
        let tooltip_height = tooltip_height.min(screen_area.get_content_cheight());

        let bottom_cy = screen_area.bottom_cy() - c_line_width;
        let (tooltip_x, tooltip_y) = place_rect_inside(
            pointer_cx,
            pointer_cy,
            tooltip_width,
            tooltip_height,
            c_line_width,
            screen_area.right_cx() - c_line_width,
            bottom_cy,
            c_expected_tooltip_shift_x,
        );

        let color_x = tooltip_x + c_padding;
        let name_x = color_x + c_color_size + c_gap_between_colors_n_names;
        let value_x = tooltip_x + tooltip_width - c_padding;
        let heading_y = tooltip_y + c_padding;
        let start_y = heading_y
            + c_heading_lines as f64 * c_font_size
            + (c_heading_lines - 1) as f64 * c_gap_between_lines
            + c_additional_gap_after_heading;

        let delta_y = c_font_size + c_gap_between_lines;
        let lines_number = ((bottom_cy - start_y) / delta_y) as usize;

        let hidden_lines: usize;
        if lines_number > 0 && lines_number < matches.len() {
            hidden_lines = matches.len() - lines_number + 1;
            matches.truncate(lines_number - 1);
        } else {
            hidden_lines = 0;
        }

        let transparent_color = JsValue::from_str("rgba(0, 0, 0, 0)");
        for (index, (data_set, data_point)) in matches.iter().enumerate() {
            let color = JsValue::from_str(data_set.to_css_color(1.0).as_str());
            context.begin_path();
            context.set_line_width(screen.apx_to_cpx(self.chart_config.line_width));
            if index_with_min_diff_by_value == index {
                context.set_fill_style(&color);
            } else {
                context.set_fill_style(&transparent_color);
            }

            context.set_stroke_style(&color);
            context
                .arc(
                    screen_area.get_cx(data_point.coord),
                    screen_area.get_cy(data_point.value),
                    screen.apx_to_cpx(self.chart_config.circle_diameter),
                    0.0,
                    PI * 2.0,
                )
                .unwrap();
            context.fill();
            context.stroke();
        }

        let v = &self.chart_config.color_tooltip_font;
        let font_color =
            JsValue::from_str(format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3,).as_str());

        let v = &self.chart_config.color_tooltip;
        let background_color =
            JsValue::from_str(format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3,).as_str());

        context.set_line_width(c_line_width);
        context.set_fill_style(&background_color);
        context.set_stroke_style(&font_color);
        context.stroke_rect(tooltip_x, tooltip_y, tooltip_width, tooltip_height);
        context.fill_rect(tooltip_x, tooltip_y, tooltip_width, tooltip_height);

        context.set_font(
            format!(
                "bold {:.0}px {}",
                screen.apx_to_cpx(self.chart_config.font_size_normal),
                self.chart_config.font_monospace.as_str()
            )
            .as_str(),
        );
        context.set_fill_style(&JsValue::from_str("black"));

        context.set_text_baseline("top");
        context.set_text_align("center");
        context
            .fill_text(
                formatted_coord.as_str(),
                tooltip_x + tooltip_width * 0.5,
                heading_y,
            )
            .unwrap();

        context.set_text_baseline("top");

        for (index, ((data_set, _), formatted_value)) in matches
            .iter()
            .cloned()
            .zip(formatted_values.iter())
            .enumerate()
        {
            let y = start_y + delta_y * index as f64;
            let color = JsValue::from_str(data_set.to_css_color(1.0).as_str());

            context.set_fill_style(&color);
            context.fill_rect(color_x, y, c_color_size, c_color_size);

            context.set_fill_style(&font_color);

            context.set_font(
                format!(
                    "{}{:.0}px {}",
                    if index_with_min_diff_by_value == index {
                        "bold "
                    } else {
                        ""
                    },
                    screen.apx_to_cpx(self.chart_config.font_size_normal),
                    self.chart_config.font_monospace.as_str()
                )
                .as_str(),
            );
            context.set_text_align("left");
            context
                .fill_text(data_set.name.as_str(), name_x, y)
                .unwrap();

            context.set_text_align("right");
            context
                .fill_text(formatted_value.as_str(), value_x, y)
                .unwrap();
        }

        if hidden_lines > 0 {
            let y = start_y + delta_y * matches.len() as f64;
            context.set_text_align("left");
            context.set_fill_style(&font_color);
            context.set_font(
                format!(
                    "{:.0}px {}",
                    screen.apx_to_cpx(self.chart_config.font_size_small),
                    self.chart_config.font_standard.as_str()
                )
                .as_str(),
            );
            context
                .fill_text(format!("{} hidden", hidden_lines).as_str(), name_x, y)
                .unwrap();
        }
    }
}
