/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::params::{ChartConfig, Content};
use crate::screen::{Screen, ScreenRect};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

const SCREEN_PADDING: f64 = 5.0;
const MARGIN_HORIZONTAL: f64 = 15.0;
const MARGIN_VERTICAL: f64 = 5.0;

pub struct LegendItem {
    pub width: f64,
    pub height: f64,
    pub color: String,
    pub name: String,
}

pub struct Legend {
    pub chart_config: Rc<ChartConfig>,
    pub items: Vec<LegendItem>,
    pub positions: Vec<ScreenRect>,
    pub arrow_left: Option<ScreenRect>,
    pub arrow_right: Option<ScreenRect>,
    pub cx_start: f64,
    pub cx_end: f64,
    pub cy_start: f64,
    pub cy_end: f64,
    pub offset: usize,
    pub mandatory_right_index: Option<usize>,
    pub approx_per_page: Option<usize>,
    pub has_next: bool,
}

impl Legend {
    pub fn from_content(chart_config: Rc<ChartConfig>, content: &Content, screen: &Screen) -> Self {
        let items = Self::content_to_items(Rc::clone(&chart_config), content, screen);
        Self {
            chart_config,
            items,
            positions: Vec::new(),
            arrow_left: None,
            arrow_right: None,
            cx_start: -1.0,
            cx_end: -1.0,
            cy_start: -1.0,
            cy_end: -1.0,
            offset: 0,
            mandatory_right_index: None,
            approx_per_page: None,
            has_next: false,
        }
    }
    fn content_to_items(
        chart_config: Rc<ChartConfig>,
        content: &Content,
        screen: &Screen,
    ) -> Vec<LegendItem> {
        let c_font_height = screen.apx_to_cpx(chart_config.font_size_large);
        let c_font_width = c_font_height * chart_config.font_width_coeff;
        let c_double_padding = c_font_height;
        content
            .data_sets
            .iter()
            .map(|data_set| LegendItem {
                width: c_font_width * data_set.name.len() as f64 + c_double_padding,
                height: c_font_height + c_double_padding,
                color: data_set.to_css_color(1.0),
                name: data_set.name.clone(),
            })
            .collect()
    }
    pub fn prev_page(&mut self) {
        if self.offset > 0 {
            self.mandatory_right_index = Some(self.offset - 1);
            let approx_page_length = match self.approx_per_page {
                Some(v) => v,
                None => self.positions.len(),
            };
            self.offset = if self.offset < approx_page_length {
                0
            } else {
                self.offset - approx_page_length
            };

            self.cx_end = 0.0; // forcing resize
        }
    }
    pub fn next_page(&mut self) {
        let current_page_length = self.positions.len();
        if self.offset + current_page_length < self.items.len() {
            self.offset += self.positions.len();
            self.cx_end = 0.0; // forcing resize
        }
    }
    pub fn resize(
        &mut self,
        screen: &Screen,
        cx_start: f64,
        cx_end: f64,
        cy_start: f64,
        cy_end: f64,
    ) {
        if self.cx_start == cx_start
            && self.cx_end == cx_end
            && self.cy_start == cy_start
            && self.cy_end == cy_end
        {
            return;
        }
        self.cx_start = cx_start;
        self.cx_end = cx_end;
        self.cy_start = cy_start;
        self.cy_end = cy_end;
        let c_screen_padding = screen.apx_to_cpx(SCREEN_PADDING);
        let c_margin_horizontal = screen.apx_to_cpx(MARGIN_HORIZONTAL);
        let c_margin_vertical = screen.apx_to_cpx(MARGIN_VERTICAL);
        let c_arrow_width = screen.apx_to_cpx(self.chart_config.font_size_large)
            * self.chart_config.font_width_coeff
            * 3.0;

        let cy_start = self.cy_start + c_screen_padding;
        let cy_end = self.cy_end - c_screen_padding;

        let mut with_buttons = self.offset > 0;
        let mut cx_start: f64;
        let mut cx_end: f64;
        loop {
            if with_buttons {
                cx_start = self.cx_start + c_arrow_width + c_margin_horizontal;
                cx_end = self.cx_end - c_arrow_width - c_margin_horizontal;
            } else {
                cx_start = self.cx_start + c_screen_padding;
                cx_end = self.cx_end - c_screen_padding;
            }

            let mut cx = cx_start;
            let mut cy = cy_start;
            self.has_next = false;

            let mut positions: Vec<ScreenRect> = Vec::new();
            for item in self.items.iter().skip(self.offset) {
                if cx + item.width > cx_end {
                    cx = cx_start;
                    cy += item.height + c_margin_vertical;
                }
                if cy + item.height > cy_end {
                    if positions.len() == 0 {
                        break;
                    }
                    self.has_next = true;
                    self.approx_per_page = Some(positions.len());
                    break;
                }
                positions.push(ScreenRect::from_width(cx, cy, item.width, item.height));
                cx += item.width + c_margin_horizontal;
            }
            if let Some(mandatory_right_index) = self.mandatory_right_index {
                let right_index = self.offset + positions.len() - 1;
                if right_index < mandatory_right_index {
                    self.offset += mandatory_right_index - right_index;
                    continue;
                }
            }
            if self.has_next && !with_buttons {
                with_buttons = true;
                continue;
            }
            self.positions = positions;
            break;
        }
        self.mandatory_right_index = None;

        if self.offset > 0 || self.has_next {
            let arrow_height = self.positions[self.positions.len() - 1].cy2 - self.positions[0].cy1;
            self.arrow_left = Some(ScreenRect::from_width(
                cx_start - c_arrow_width - c_margin_horizontal,
                cy_start,
                c_arrow_width,
                arrow_height,
            ));
            self.arrow_right = Some(ScreenRect::from_width(
                cx_end + c_margin_horizontal,
                cy_start,
                c_arrow_width,
                arrow_height,
            ));
        } else {
            self.arrow_left = None;
            self.arrow_right = None;
        }
    }

    pub fn draw(&mut self, content: &Content, screen: &mut Screen, _time_us: f64) {
        screen.clear();
        let context = &screen.context;

        let color_white = JsValue::from_str("white");

        context.set_font(
            format!(
                "{:.0}px {}",
                screen.apx_to_cpx(self.chart_config.font_size_large),
                self.chart_config.font_standard.as_str()
            )
            .as_str(),
        );
        context.set_text_baseline("middle");
        context.set_text_align("center");
        context.set_line_width(screen.apx_to_cpx(2.0));

        for ((item, position), data_set) in self
            .items
            .iter()
            .skip(self.offset)
            .zip(self.positions.iter())
            .zip(content.data_sets.iter().skip(self.offset))
        {
            let color = JsValue::from_str(item.color.as_str());
            context.set_fill_style(&color);
            if data_set.alpha.get_end_value() == 0.0 {
                context.set_stroke_style(&color);
                context.stroke_rect(position.cx1, position.cy1, item.width, item.height);
            } else {
                context.fill_rect(position.cx1, position.cy1, item.width, item.height);
                context.set_fill_style(&color_white);
            }
            context
                .fill_text(
                    item.name.as_str(),
                    position.cx1 + 0.5 * item.width,
                    position.cy1 + 0.5 * item.height,
                )
                .unwrap();
        }

        if let (Some(arrow_left), Some(arrow_right)) = (&self.arrow_left, &self.arrow_right) {
            let v = &self.chart_config.color_preview_overlay;
            context.set_fill_style(&JsValue::from_str(
                format!(
                    "rgba({}, {}, {}, {})",
                    v.0,
                    v.1,
                    v.2,
                    if self.offset > 0 { v.3 } else { v.3 * 0.5 }
                )
                .as_str(),
            ));
            context.fill_rect(
                arrow_left.cx1,
                arrow_left.cy1,
                arrow_left.width(),
                arrow_left.height(),
            );
            context.set_fill_style(&JsValue::from_str(
                format!(
                    "rgba({}, {}, {}, {})",
                    v.0,
                    v.1,
                    v.2,
                    if self.has_next { v.3 } else { v.3 * 0.5 }
                )
                .as_str(),
            ));
            context.fill_rect(
                arrow_right.cx1,
                arrow_right.cy1,
                arrow_right.width(),
                arrow_right.height(),
            );
            context.set_font(
                format!(
                    "{:.0}px {}",
                    screen.apx_to_cpx(self.chart_config.font_size_large),
                    self.chart_config.font_standard.as_str()
                )
                .as_str(),
            );
            let v = &self.chart_config.color_preview_hint;
            context.set_fill_style(&JsValue::from_str(
                format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3).as_str(),
            ));
            let c_width = arrow_left.width();
            let c_height = arrow_left.height();
            let c_size = c_width.min(c_height);
            let c_horizontal = c_size * 0.3;
            let c_vertical = c_size * 0.4;
            let cx_center = arrow_left.cx_center();
            let cy_center = arrow_left.cy_center();
            context.begin_path();
            context.move_to(
                cx_center + c_horizontal * 0.35,
                cy_center - c_vertical * 0.5,
            );
            context.line_to(
                cx_center + c_horizontal * 0.35,
                cy_center + c_vertical * 0.5,
            );
            context.line_to(cx_center - c_horizontal * 0.65, cy_center);
            context.close_path();
            context.fill();
            let cx_center = arrow_right.cx_center();
            let cy_center = arrow_right.cy_center();
            context.begin_path();
            context.move_to(
                cx_center - c_horizontal * 0.35,
                cy_center - c_vertical * 0.5,
            );
            context.line_to(
                cx_center - c_horizontal * 0.35,
                cy_center + c_vertical * 0.5,
            );
            context.line_to(cx_center + c_horizontal * 0.65, cy_center);
            context.close_path();
            context.fill();
        }
    }
}
