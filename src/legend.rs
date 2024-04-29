/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::params::{ChartConfig, Content};
use crate::screen::ScreenRect;
use crate::screen::{ScreenArea, ScreenPos, Size};
use crate::utils::is_click;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

const SCREEN_PADDING: Size = Size::Px(5.0);
const MARGIN_HORIZONTAL: Size = Size::Px(15.0);
const MARGIN_VERTICAL: Size = Size::Px(5.0);
const LINE_WIDTH: Size = Size::Px(2.0);

pub struct LegendItem {
    pub width: f64,
    pub height: f64,
    pub color: String,
    pub name: String,
}

pub struct Legend {
    pub chart_config: Rc<RefCell<ChartConfig>>,
    pub control_screen_area: ScreenArea,
    pub pointer: Option<ScreenPos>,
    pub pointer_down: Option<ScreenPos>,
    pub pointer_down_time_us: Option<f64>,
    pub items: Option<Rc<Vec<LegendItem>>>,
    pub positions: Vec<ScreenRect>,
    pub arrow_left: Option<ScreenRect>,
    pub arrow_right: Option<ScreenRect>,
    pub last_canvas_height: f64,
    pub last_canvas_width: f64,
    pub offset: usize,
    pub mandatory_right_index: Option<usize>,
    pub approx_per_page: Option<usize>,
    pub has_next: bool,
}

impl Legend {
    pub fn new(chart_config: Rc<RefCell<ChartConfig>>, control_screen_area: ScreenArea) -> Self {
        Self {
            chart_config,
            control_screen_area,
            pointer: None,
            pointer_down: None,
            pointer_down_time_us: None,
            items: None,
            positions: Vec::new(),
            arrow_left: None,
            arrow_right: None,
            last_canvas_height: 0.0,
            last_canvas_width: 0.0,
            offset: 0,
            mandatory_right_index: None,
            approx_per_page: None,
            has_next: false,
        }
    }
    fn get_items(&mut self, content: &Content) -> Rc<Vec<LegendItem>> {
        if self.items.is_none() {
            let screen_area_handle_rc = self.control_screen_area.get_handle();
            let screen_area_handle = screen_area_handle_rc.as_ref();
            let conf = self.chart_config.borrow();
            let c_font_height = conf.font_size_large.to_cpx_height(screen_area_handle);
            let c_font_width = conf.font_size_large.to_cpx_width(screen_area_handle);
            let c_double_padding = c_font_height;
            self.items.replace(Rc::new(
                content
                    .data_sets
                    .iter()
                    .map(|data_set| LegendItem {
                        width: c_font_width * data_set.name.len() as f64 + c_double_padding,
                        height: c_font_height + c_double_padding,
                        color: data_set.to_css_color(1.0),
                        name: data_set.name.clone(),
                    })
                    .collect(),
            ));
        }
        Rc::clone(self.items.as_ref().unwrap())
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

            self.last_canvas_height = 0.0; // forcing resize
        }
    }
    pub fn next_page(&mut self) {
        if let Some(items) = self.items.as_ref() {
            let current_page_length = self.positions.len();
            if self.offset + current_page_length < items.len() {
                self.offset += self.positions.len();
                self.last_canvas_height = 0.0; // forcing resize
            }
        }
    }
    fn resize(&mut self, content: &Content) {
        let screen_area_handle_rc = self.control_screen_area.get_handle();
        let screen_area_handle = screen_area_handle_rc.as_ref();
        if self.last_canvas_height == screen_area_handle.canvas_content_height
            && self.last_canvas_width == screen_area_handle.canvas_content_width
        {
            return;
        }
        self.last_canvas_height = screen_area_handle.canvas_content_height;
        self.last_canvas_width = screen_area_handle.canvas_content_width;

        let conf = self.chart_config.borrow();

        let c_screen_padding = SCREEN_PADDING.to_cpx_height(screen_area_handle);
        let c_margin_horizontal = MARGIN_HORIZONTAL.to_cpx_height(screen_area_handle);
        let c_margin_vertical = MARGIN_VERTICAL.to_cpx_height(screen_area_handle);
        let c_arrow_width = conf.font_size_large.to_cpx_width(screen_area_handle) * 3.0;
        drop(conf);

        let cy_start = screen_area_handle.top_cy() + c_screen_padding;
        let cy_end = screen_area_handle.bottom_cy() - c_screen_padding;

        let mut with_buttons = self.offset > 0;
        let mut cx_start: f64;
        let mut cx_end: f64;
        let mut has_next;
        let mut approx_per_page: Option<usize> = None;
        let mut positions: Vec<ScreenRect>;
        let mut offset = self.offset;
        let mandatory_right_index = self.mandatory_right_index.clone();

        loop {
            if with_buttons {
                cx_start = screen_area_handle.left_cx() + c_arrow_width + c_margin_horizontal;
                cx_end = screen_area_handle.right_cx() - c_arrow_width - c_margin_horizontal;
            } else {
                cx_start = screen_area_handle.left_cx() + c_screen_padding;
                cx_end = screen_area_handle.right_cx() - c_screen_padding;
            }

            let mut cx = cx_start;
            let mut cy = cy_start;
            has_next = false;

            positions = Vec::new();
            let items = self.get_items(content);

            for item in items.iter().skip(offset) {
                if cx + item.width > cx_end {
                    cx = cx_start;
                    cy += item.height + c_margin_vertical;
                }
                if cy + item.height > cy_end {
                    if positions.len() == 0 {
                        break;
                    }
                    has_next = true;
                    approx_per_page = Some(positions.len());
                    break;
                }
                positions.push(ScreenRect::from_width(cx, cy, item.width, item.height));
                cx += item.width + c_margin_horizontal;
            }
            if let Some(mandatory_right_index) = mandatory_right_index {
                let right_index = offset + positions.len() - 1;
                if right_index < mandatory_right_index {
                    offset += mandatory_right_index - right_index;
                    continue;
                }
            }
            if has_next && !with_buttons {
                with_buttons = true;
                continue;
            }
            break;
        }
        self.offset = offset;
        self.positions = positions;
        self.approx_per_page = approx_per_page;
        self.mandatory_right_index = None;
        self.has_next = has_next;

        if self.offset > 0 || has_next {
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

    pub fn draw(&mut self, content: &Content) {
        self.resize(content);
        let screen_area_handle_rc = self.control_screen_area.get_handle();
        let screen_area_handle = screen_area_handle_rc.as_ref();

        let crc = screen_area_handle.crc.as_ref();

        let color_white = JsValue::from_str("white");

        {
            let conf = self.chart_config.borrow();
            crc.set_font(
                format!(
                    "{:.0}px {}",
                    conf.font_size_large.to_cpx_height(screen_area_handle),
                    conf.font_standard.as_str(),
                )
                .as_str(),
            );
        }
        crc.set_text_baseline("middle");
        crc.set_text_align("center");
        crc.set_line_width(LINE_WIDTH.to_cpx_height(screen_area_handle));

        let offset = self.offset;
        let items = self.get_items(content);
        for ((item, position), data_set) in items
            .iter()
            .skip(offset)
            .zip(self.positions.iter())
            .zip(content.data_sets.iter().skip(offset))
        {
            let color = JsValue::from_str(item.color.as_str());
            crc.set_fill_style(&color);
            if data_set.alpha.get_end_value() == 0.0 {
                crc.set_stroke_style(&color);
                crc.stroke_rect(position.cx1, position.cy1, item.width, item.height);
            } else {
                crc.fill_rect(position.cx1, position.cy1, item.width, item.height);
                crc.set_fill_style(&color_white);
            }
            crc.fill_text(
                item.name.as_str(),
                position.cx1 + 0.5 * item.width,
                position.cy1 + 0.5 * item.height,
            )
            .unwrap();
        }

        let conf = self.chart_config.borrow();
        if let (Some(arrow_left), Some(arrow_right)) = (&self.arrow_left, &self.arrow_right) {
            let v = conf.color_preview_overlay;
            crc.set_fill_style(&JsValue::from_str(
                format!(
                    "rgba({}, {}, {}, {})",
                    v.0,
                    v.1,
                    v.2,
                    if self.offset > 0 { v.3 } else { v.3 * 0.5 }
                )
                .as_str(),
            ));
            crc.fill_rect(
                arrow_left.cx1,
                arrow_left.cy1,
                arrow_left.width(),
                arrow_left.height(),
            );
            crc.set_fill_style(&JsValue::from_str(
                format!(
                    "rgba({}, {}, {}, {})",
                    v.0,
                    v.1,
                    v.2,
                    if self.has_next { v.3 } else { v.3 * 0.5 }
                )
                .as_str(),
            ));
            crc.fill_rect(
                arrow_right.cx1,
                arrow_right.cy1,
                arrow_right.width(),
                arrow_right.height(),
            );
            crc.set_font(
                format!(
                    "{:.0}px {}",
                    conf.font_size_large.to_cpx_height(screen_area_handle),
                    conf.font_standard.as_str()
                )
                .as_str(),
            );
            let v = conf.color_preview_hint;
            crc.set_fill_style(&JsValue::from_str(
                format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3).as_str(),
            ));
            let c_width = arrow_left.width();
            let c_height = arrow_left.height();
            let c_size = c_width.min(c_height);
            let c_horizontal = c_size * 0.3;
            let c_vertical = c_size * 0.4;
            let cx_center = arrow_left.cx_center();
            let cy_center = arrow_left.cy_center();
            crc.begin_path();
            crc.move_to(
                cx_center + c_horizontal * 0.35,
                cy_center - c_vertical * 0.5,
            );
            crc.line_to(
                cx_center + c_horizontal * 0.35,
                cy_center + c_vertical * 0.5,
            );
            crc.line_to(cx_center - c_horizontal * 0.65, cy_center);
            crc.close_path();
            crc.fill();
            let cx_center = arrow_right.cx_center();
            let cy_center = arrow_right.cy_center();
            crc.begin_path();
            crc.move_to(
                cx_center - c_horizontal * 0.35,
                cy_center - c_vertical * 0.5,
            );
            crc.line_to(
                cx_center - c_horizontal * 0.35,
                cy_center + c_vertical * 0.5,
            );
            crc.line_to(cx_center + c_horizontal * 0.65, cy_center);
            crc.close_path();
            crc.fill();
        }
    }
    pub fn on_click(&mut self, content: &mut Content, time_us: f64) -> bool {
        let mut made_changes = false;
        if let Some(pos) = self.pointer_down.as_ref() {
            let screen_area_handle = self.control_screen_area.get_handle();

            let cx = screen_area_handle.get_cx(pos);
            let cy = screen_area_handle.get_cy(pos);
            let mut clicked_index: Option<usize> = None;
            for (index, position) in self.positions.iter().enumerate() {
                if position.contains(cx, cy) {
                    clicked_index = Some(index);
                    break;
                }
            }
            if let Some(index) = clicked_index {
                self.toggle_data_set(content, self.offset + index, time_us)
                    .unwrap();
                made_changes = true;
            }
            if let Some(arrow_left) = &self.arrow_left {
                if arrow_left.contains(cx, cy) {
                    self.prev_page();
                    made_changes = true;
                }
            }
            if let Some(arrow_right) = &self.arrow_right {
                if arrow_right.contains(cx, cy) {
                    self.next_page();
                    made_changes = true;
                }
            }
        }
        made_changes
    }
    pub fn on_long_press(&mut self, content: &mut Content, time_us: f64) -> usize {
        if let (Some(pointer_down_time_us), Some(pointer_down), Some(pointer)) = (
            &self.pointer_down_time_us,
            &self.pointer_down,
            &self.pointer,
        ) {
            let conf = self.chart_config.borrow();

            if time_us - *pointer_down_time_us > conf.us_long_press
                && is_click(pointer_down, pointer)
            {
                let screen_area_handle = self.control_screen_area.get_handle();
                let mut clicked_index: Option<usize> = None;
                let cx = screen_area_handle.get_cx(pointer);
                let cy = screen_area_handle.get_cy(pointer);
                for (index, position) in self.positions.iter().enumerate() {
                    if position.contains(cx, cy) {
                        clicked_index = Some(index);
                        break;
                    }
                }
                if let Some(index) = clicked_index {
                    let index_to_show = index + self.offset;
                    for (index, data_set) in content.data_sets.iter_mut().enumerate() {
                        data_set.alpha.set_value(
                            if index == index_to_show { 1.0 } else { 0.0 },
                            Some(time_us),
                        );
                    }
                    self.pointer_down = None;
                    self.pointer_down_time_us = None;
                }
            }
            1
        } else {
            0
        }
    }

    fn toggle_data_set(
        &self,
        content: &mut Content,
        index: usize,
        time_us: f64,
    ) -> Result<(), String> {
        let number_of_data_sets = content.data_sets.len();
        if index >= number_of_data_sets {
            return Err(format!(
                "chart contains {number_of_data_sets} data sets; index is out of bound"
            ));
        }
        let is_visible = content.data_sets[index].alpha.get_end_value() == 1.0;
        if is_visible
            && content
                .data_sets
                .iter()
                .filter(|data_set| data_set.alpha.get_end_value() == 1.0)
                .count()
                == 1
        {
            for (index_, data_set) in content.data_sets.iter_mut().enumerate() {
                if index_ != index {
                    data_set.alpha.set_value(1.0, Some(time_us));
                }
            }
        } else {
            let alpha = &mut content.data_sets[index].alpha;
            alpha.set_value(1.0 - alpha.get_end_value(), Some(time_us));
        }
        Ok(())
    }
}
