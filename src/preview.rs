/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::params::Content;
use crate::params::{ChartConfig, ClientCaps};
use crate::scale::Scale;
use crate::screen::{CoordSpace, ScreenArea, ScreenPos};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsValue;

pub struct Preview<T>
where
    T: Scale,
{
    pub chart_config: Rc<RefCell<ChartConfig>>,
    pub client_caps: Rc<RefCell<ClientCaps>>,

    pub coord_space: CoordSpace<T>,
    pub control_coord_space: CoordSpace<T>,

    pub grip_hold_coord_offset: Option<f64>,
    pub pointer: Option<ScreenPos>,
    pub pointer_down: Option<ScreenPos>,
    pub pointer_down_time_us: Option<f64>,

    pub time_us: f64,
    pub dirty: bool,
}
impl<T> Preview<T>
where
    T: Scale,
{
    pub fn new(
        chart_config: Rc<RefCell<ChartConfig>>,
        client_caps: Rc<RefCell<ClientCaps>>,
        screen_area: ScreenArea,
        control_screen_area: ScreenArea,
        scale: T,
        content: &mut Content,
    ) -> Self {
        let mut preview = Self {
            chart_config,
            client_caps,

            coord_space: CoordSpace::new(screen_area, scale.clone()),
            control_coord_space: CoordSpace::new(control_screen_area, scale),

            grip_hold_coord_offset: None,
            pointer: None,
            pointer_down: None,
            pointer_down_time_us: None,

            time_us: 0.0,
            dirty: false,
        };
        preview.update_by_content(content, None);
        preview
    }
    pub fn update_by_content(&mut self, content: &mut Content, time_us: Option<f64>) {
        let [coord_min, coord_max, value_min, value_max] = content.get_min_max();
        self.coord_space
            .content_updated(coord_min, coord_max, value_min, value_max, time_us);
        self.control_coord_space
            .content_updated(coord_min, coord_max, value_min, value_max, time_us);
    }
    pub fn draw(&mut self, content: &mut Content, time_us: f64) {
        let coord_space_handle = self.coord_space.get_handle(time_us);
        let screen_area_handle = coord_space_handle.screen_area_handle.as_ref();
        let crc = screen_area_handle.crc.as_ref();

        let chart_config = self.chart_config.borrow();

        let mut alpha: f64;
        for data_set in content.data_sets.iter_mut() {
            alpha = data_set.alpha.get_value(time_us);
            if alpha == 0.0 {
                continue;
            }
            let mut it = data_set.data_points.iter();
            let data_point = it.next().unwrap();
            crc.begin_path();
            crc.set_stroke_style(&JsValue::from_str(data_set.to_css_color(alpha).as_str()));
            crc.set_line_width(chart_config.line_width.to_cpx_height(screen_area_handle));

            let mut prev_x = coord_space_handle.get_cx(data_point.coord);
            let mut prev_y = coord_space_handle.get_cy(data_point.value);
            crc.move_to(prev_x, prev_y);
            let mut x: f64;
            let mut y: f64;
            for data_point in it {
                x = coord_space_handle.get_cx(data_point.coord);
                y = coord_space_handle.get_cy(data_point.value);
                if x - prev_x >= 1.0 || (y - prev_y).abs() >= 1.0 {
                    crc.line_to(x, y);
                    prev_x = x;
                    prev_y = y;
                }
            }
            crc.stroke();
        }
    }

    pub fn draw_grip(&mut self, grip: Option<(f64, f64)>, time_us: f64) {
        let slide_in_progress = self.grip_hold_coord_offset.is_some();
        let zoomed_in = grip.is_some();

        let coord_space_handle = self.control_coord_space.get_handle(time_us);
        let screen_area_handle = coord_space_handle.screen_area_handle.as_ref();
        let crc = screen_area_handle.crc.as_ref();

        let chart_config = self.chart_config.borrow();

        let top_y = screen_area_handle.top_cy();
        let bottom_y = screen_area_handle.bottom_cy();
        let height = bottom_y - top_y;

        let left_x = screen_area_handle.left_cx();
        let right_x = screen_area_handle.right_cx();
        let width = right_x - left_x;

        if let Some((grip_coord, grip_coord_range)) = grip {
            let grip_x_start = coord_space_handle.get_cx(grip_coord - grip_coord_range * 0.5);
            let grip_x_end = coord_space_handle.get_cx(grip_coord + grip_coord_range * 0.5);
            let v = chart_config.color_camera_grip;
            crc.set_fill_style(&JsValue::from_str(
                format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3).as_str(),
            ));
            crc.fill_rect(grip_x_start, top_y, grip_x_end - grip_x_start, height);
        }

        if !slide_in_progress {
            let v = chart_config.color_preview_overlay;
            crc.set_fill_style(&JsValue::from_str(
                format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3,).as_str(),
            ));
            crc.fill_rect(left_x, top_y, width, height);

            crc.set_text_align("center");
            crc.set_text_baseline("middle");
            crc.set_font(
                format!(
                    "{}px {}",
                    chart_config
                        .font_size_large
                        .to_cpx_height(screen_area_handle),
                    chart_config.font_standard.as_str()
                )
                .as_str(),
            );
            let v = chart_config.color_preview_hint;
            crc.set_fill_style(&JsValue::from_str(
                format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3,).as_str(),
            ));
            crc.fill_text(
                if zoomed_in {
                    "Click to zoom out"
                } else {
                    "Drag here or above to zoom in"
                },
                (left_x + right_x) * 0.5,
                (top_y + bottom_y) * 0.5,
            )
            .unwrap();

            let font_cpx_size = chart_config
                .font_size_small
                .to_cpx_height(screen_area_handle);
            crc.set_font(
                format!(
                    "{}px {}",
                    font_cpx_size,
                    chart_config.font_standard.as_str()
                )
                .as_str(),
            );
            crc.set_text_baseline("bottom");
            crc.set_text_align("right");
            crc.fill_text(
                "© Plotica",
                right_x - font_cpx_size,
                bottom_y - font_cpx_size * 0.5,
            )
            .unwrap();
        }
    }
}
