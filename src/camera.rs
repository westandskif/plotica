/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::animate::AnimatedNumber;
use crate::grid::{Grid, Tick};
use crate::params::Content;
use crate::params::{ChartConfig, VerboseFormat};
use crate::scale::Scale;
use crate::screen::{Screen, ScreenArea};
use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::animate::ANIMATED_NUMBERS_COUNT;
use std::sync::atomic::Ordering;

pub enum Axis {
    X,
    Y,
}

const COORD_TICKS_DUTY_FACTOR: f64 = 1.5;
const VALUE_TICKS_DUTY_FACTOR: f64 = 5.0;

pub struct Camera<T>
where
    T: Scale,
{
    pub chart_config: Rc<ChartConfig>,
    screen_area: ScreenArea<T>,
    pub scale_time_us: f64,
    pub coord: AnimatedNumber,
    pub coord_range: AnimatedNumber,
    pub coord_ticks_height: f64,
    pub value: AnimatedNumber,
    pub value_range: AnimatedNumber,
    pub value_ticks_width: f64,
    pub coord_grid: Grid,
    pub value_grid: Grid,
    pub dirty: bool,
}
impl<T> Camera<T>
where
    T: Scale,
{
    pub fn new(
        chart_config: Rc<ChartConfig>,
        scale: T,
        coord_ticks_height: f64,
        value_ticks_width: f64,
        content: &mut Content,
        screen: &Screen,
        padding: [f64; 4],
    ) -> Camera<T> {
        let screen_area = ScreenArea::new(scale, screen, padding);
        let coord_grid = Grid::new(
            content.coord_type,
            content.global_coord_min,
            content.global_coord_max,
        );
        let value_grid = Grid::new(
            content.value_type,
            content.global_value_min,
            content.global_value_max,
        );
        let mut camera = Self {
            chart_config,
            screen_area,
            scale_time_us: 0.0,
            coord: AnimatedNumber::new(0.0),
            coord_range: AnimatedNumber::new(0.0),
            coord_ticks_height,
            value: AnimatedNumber::new(0.0),
            value_range: AnimatedNumber::new(0.0),
            value_ticks_width,
            coord_grid,
            value_grid,
            dirty: false,
        };
        camera.update_by_content(content, None);
        camera
    }
    pub fn update_by_content(&mut self, content: &mut Content, time_us: Option<f64>) {
        self.dirty = true;
        let mut coord_min: f64 = f64::MAX;
        let mut coord_max: f64 = f64::MIN;
        let mut value_min: f64 = f64::MAX;
        let mut value_max: f64 = f64::MIN;
        for data_set in content.data_sets.iter_mut() {
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
        self.coord.set_value((coord_max + coord_min) * 0.5, time_us);
        self.coord_range.set_value(coord_max - coord_min, time_us);
        self.value.set_value((value_max + value_min) * 0.5, time_us);
        self.value_range.set_value(value_max - value_min, time_us);
    }
    pub fn zoom_by_coords(
        &mut self,
        content: &mut Content,
        coord_start: f64,
        coord_end: f64,
        time_us: Option<f64>,
    ) {
        if coord_end <= coord_start {
            panic!();
        }

        let mut value_min: f64 = f64::MAX;
        let mut value_max: f64 = f64::MIN;
        let mut number_of_points: usize = 0;
        for data_set in content.data_sets.iter_mut() {
            if data_set.alpha.get_end_value() > 0.0 {
                if let Some(data_points) = data_set.slice_by_coord(coord_start, coord_end) {
                    number_of_points = number_of_points.max(data_points.len());
                    for data_point in data_points.iter() {
                        value_min = value_min.min(data_point.value);
                        value_max = value_max.max(data_point.value);
                    }
                }
            }
        }
        if number_of_points > 1 {
            self.dirty = true;
            self.coord
                .set_value((coord_start + coord_end) * 0.5, time_us);
            self.coord_range.set_value(coord_end - coord_start, time_us);
            self.value_range.set_value(value_max - value_min, time_us);
            self.value.set_value((value_min + value_max) * 0.5, time_us);
        }
    }
    pub fn move_to(&mut self, content: &mut Content, coord_center: f64, time_us: Option<f64>) {
        self.dirty = true;
        self.coord.set_value(coord_center, time_us);
        let coord_half_range = self.coord_range.get_end_value() * 0.5;
        let coord_start = coord_center - coord_half_range;
        let coord_end = coord_center + coord_half_range;
        let mut value_min: f64 = f64::MAX;
        let mut value_max: f64 = f64::MIN;
        for data_set in content.data_sets.iter_mut() {
            if data_set.alpha.get_end_value() > 0.0 {
                if let Some(data_points) = data_set.slice_by_coord(coord_start, coord_end) {
                    for data_point in data_points.iter() {
                        value_min = value_min.min(data_point.value);
                        value_max = value_max.max(data_point.value);
                    }
                }
            }
        }
        self.value_range.set_value(value_max - value_min, time_us);
        self.value.set_value((value_min + value_max) * 0.5, time_us);
    }
    pub fn sync_screen_area(&mut self, screen: &mut Screen, time_us: f64) {
        if self.scale_time_us != time_us {
            self.scale_time_us = time_us;

            if screen.sync_canvas_size() {
                self.screen_area.update(screen);
            }

            let coord = self.coord.get_value(time_us);
            let half_coord_range = self.coord_range.get_value(time_us) * 0.5;
            let value = self.value.get_value(time_us);
            let half_value_range = self.value_range.get_value(time_us) * 0.5;

            self.screen_area.scale.change_focus(
                coord - half_coord_range,
                coord + half_coord_range,
                value - half_value_range,
                value + half_value_range,
            );
        }
    }
    pub fn get_content_screen_area(&self, time_us: f64) -> &ScreenArea<T> {
        if self.scale_time_us != time_us {
            panic!("screen area out of sync");
        }
        &self.screen_area
    }
    pub fn shoot(&mut self, content: &mut Content, screen: &mut Screen, time_us: f64) {
        if !self.dirty {
            self.scale_time_us = time_us;
            return;
        }
        let animated_numbers_before = ANIMATED_NUMBERS_COUNT.load(Ordering::Relaxed);

        self.sync_screen_area(screen, time_us);
        screen.clear();

        if self.coord_ticks_height > 0.0 {
            let ticks = self.get_coord_ticks(
                self.get_content_screen_area(time_us).get_content_cwidth()
                    / (screen.apx_to_cpx(
                        self.chart_config.font_size_small * self.chart_config.font_width_coeff,
                    ) * content.coord_short_verbose_len as f64
                        * COORD_TICKS_DUTY_FACTOR),
                time_us,
            );
            self.draw_grid(screen, ticks.as_slice(), Axis::X, time_us);
            self.draw_ticks(screen, content, ticks.as_slice(), Axis::X, time_us);
        }
        if self.value_ticks_width > 0.0 {
            let ticks = self.get_value_ticks(
                self.get_content_screen_area(time_us).get_content_cheight()
                    / (screen.apx_to_cpx(self.chart_config.font_size_small)
                        * VALUE_TICKS_DUTY_FACTOR),
                time_us,
            );
            self.draw_grid(screen, ticks.as_slice(), Axis::Y, time_us);
            self.draw_ticks(screen, content, ticks.as_slice(), Axis::Y, time_us);
        }

        let context = &screen.context;
        let content_screen_area = self.get_content_screen_area(time_us);
        let mut alpha: f64;
        for data_set in content.data_sets.iter_mut() {
            alpha = data_set.alpha.get_value(time_us);
            if alpha == 0.0 {
                continue;
            }
            if let Some(data_points) = data_set.slice_by_coord(
                content_screen_area.scale.get_coord_min(),
                content_screen_area.scale.get_coord_max(),
            ) {
                let mut it = data_points.iter();
                let data_point = it.next().unwrap();
                context.begin_path();
                context.set_stroke_style(&JsValue::from_str(data_set.to_css_color(alpha).as_str()));
                context.set_line_width(screen.apx_to_cpx(self.chart_config.line_width));

                let mut prev_x = content_screen_area.get_cx(data_point.coord);
                let mut prev_y = content_screen_area.get_cy(data_point.value);
                context.move_to(prev_x, prev_y);
                let mut x: f64;
                let mut y: f64;
                for data_point in it {
                    x = content_screen_area.get_cx(data_point.coord);
                    y = content_screen_area.get_cy(data_point.value);
                    if x - prev_x >= 1.0 || (y - prev_y).abs() >= 1.0 {
                        context.line_to(x, y);
                        prev_x = x;
                        prev_y = y;
                    }
                }
                context.stroke();
            }
        }

        if ANIMATED_NUMBERS_COUNT.load(Ordering::Relaxed) == animated_numbers_before {
            self.dirty = false;
        }
    }
    fn draw_grid(&mut self, screen: &mut Screen, ticks: &[Tick], axis: Axis, time_us: f64) {
        let screen_area = self.get_content_screen_area(time_us);
        let context = &screen.context;
        let mut alpha: f64 = -1.0;
        context.set_line_width(1.0);
        let v = &self.chart_config.color_grid;
        match axis {
            Axis::X => {
                for tick in ticks.iter() {
                    context.begin_path();
                    if tick.alpha != alpha {
                        context.set_stroke_style(&JsValue::from_str(
                            format!("rgb({}, {}, {}, {:.3})", v.0, v.1, v.2, tick.alpha).as_str(),
                        ));
                        alpha = tick.alpha;
                    }
                    context.move_to(screen_area.get_cx(tick.value), screen_area.bottom_cy());
                    context.line_to(screen_area.get_cx(tick.value), screen_area.top_cy());
                    context.stroke();
                }
            }
            Axis::Y => {
                for tick in ticks.iter() {
                    context.begin_path();
                    if tick.alpha != alpha {
                        context.set_stroke_style(&JsValue::from_str(
                            format!("rgb({}, {}, {}, {:.3})", v.0, v.1, v.2, tick.alpha).as_str(),
                        ));
                        alpha = tick.alpha;
                    }
                    context.move_to(screen_area.left_cx(), screen_area.get_cy(tick.value));
                    context.line_to(screen_area.right_cx(), screen_area.get_cy(tick.value));
                    context.stroke();
                }
            }
        }
    }
    fn draw_ticks(
        &mut self,
        screen: &mut Screen,
        content: &Content,
        ticks: &[Tick],
        axis: Axis,
        time_us: f64,
    ) {
        let screen_area = self.get_content_screen_area(time_us);
        let verbose_format: &VerboseFormat;
        let min_value: f64;
        let max_value: f64;
        match axis {
            Axis::X => {
                verbose_format = &content.coord_verbose_format_short;
                min_value = self.screen_area.scale.get_coord_min();
                max_value = self.screen_area.scale.get_coord_max();
            }
            Axis::Y => {
                verbose_format = &content.value_verbose_format_short;
                min_value = self.screen_area.scale.get_value_min();
                max_value = self.screen_area.scale.get_value_max();
            }
        }
        let formatted_ticks = verbose_format.format_values(
            ticks.iter(),
            |tick: &Tick| tick.value,
            min_value,
            max_value,
        );
        let context = &screen.context;

        context.set_font(
            format!(
                "{}px {}",
                screen.apx_to_cpx(self.chart_config.font_size_small),
                self.chart_config.font_standard.as_str()
            )
            .as_str(),
        );

        let tick_color = &self.chart_config.color_tick;

        match axis {
            Axis::X => {
                let mut alpha: f64 = -1.0;
                let y = screen_area.bottom_cy() + screen.apx_to_cpx(self.coord_ticks_height * 0.5);
                context.set_text_align("center");
                context.set_text_baseline("middle");
                for (tick, formatted_tick) in ticks.iter().zip(formatted_ticks.iter()) {
                    if tick.alpha != alpha {
                        context.set_fill_style(&JsValue::from_str(
                            format!(
                                "rgba({}, {}, {}, {:.3})",
                                tick_color.0, tick_color.1, tick_color.2, tick.alpha
                            )
                            .as_str(),
                        ));
                        alpha = tick.alpha;
                    }
                    context
                        .fill_text(formatted_tick.as_str(), screen_area.get_cx(tick.value), y)
                        .unwrap();
                }
            }
            Axis::Y => {
                let mut alpha: f64 = -1.0;
                let x = screen_area.left_cx()
                    - screen.apx_to_cpx(
                        self.chart_config.font_size_small * self.chart_config.font_width_coeff,
                    ) * 0.5;
                context.set_text_align("right");
                context.set_text_baseline("middle");
                for (tick, formatted_tick) in ticks.iter().zip(formatted_ticks.iter()) {
                    if tick.alpha != alpha {
                        context.set_fill_style(&JsValue::from_str(
                            format!(
                                "rgba({}, {}, {}, {:.3})",
                                tick_color.0, tick_color.1, tick_color.2, tick.alpha
                            )
                            .as_str(),
                        ));
                        alpha = tick.alpha;
                    }
                    context
                        .fill_text(formatted_tick.as_str(), x, screen_area.get_cy(tick.value))
                        .unwrap();
                }
            }
        }
    }

    pub fn draw_grip(
        &mut self,
        screen: &mut Screen,
        screen_area: &ScreenArea<T>,
        zoomed_in: bool,
        slide_in_progress: bool,
        time_us: f64,
    ) {
        screen.clear();
        let context = &screen.context;

        let top_y = screen_area.top_cy();
        let bottom_y = screen_area.bottom_cy();
        let height = bottom_y - top_y;

        let left_x = screen_area.left_cx();
        let right_x = screen_area.right_cx();
        let width = right_x - left_x;

        if zoomed_in {
            let (coord, coord_range) = if slide_in_progress {
                (self.coord.get_end_value(), self.coord_range.get_end_value())
            } else {
                (
                    self.coord.get_value(time_us),
                    self.coord_range.get_value(time_us),
                )
            };
            let grip_x_start = screen_area.get_cx(coord - coord_range * 0.5);
            let grip_x_end = screen_area.get_cx(coord + coord_range * 0.5);
            let v = &self.chart_config.color_camera_grip;
            context.set_fill_style(&JsValue::from_str(
                format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3).as_str(),
            ));
            context.fill_rect(grip_x_start, top_y, grip_x_end - grip_x_start, height);
        }

        if !slide_in_progress {
            let v = &self.chart_config.color_preview_overlay;
            context.set_fill_style(&JsValue::from_str(
                format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3,).as_str(),
            ));
            context.fill_rect(left_x, top_y, width, height);

            context.set_text_align("center");
            context.set_text_baseline("middle");
            context.set_font(
                format!(
                    "{}px {}",
                    screen.apx_to_cpx(self.chart_config.font_size_large),
                    self.chart_config.font_standard.as_str()
                )
                .as_str(),
            );
            let v = &self.chart_config.color_preview_hint;
            context.set_fill_style(&JsValue::from_str(
                format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3,).as_str(),
            ));
            context
                .fill_text(
                    if zoomed_in {
                        "Click to zoom out"
                    } else {
                        "Drag here or above to zoom in"
                    },
                    (left_x + right_x) * 0.5,
                    (top_y + bottom_y) * 0.5,
                )
                .unwrap();

            let font_cpx_size = screen.apx_to_cpx(self.chart_config.font_size_small);
            context.set_font(
                format!(
                    "{}px {}",
                    font_cpx_size,
                    self.chart_config.font_standard.as_str()
                )
                .as_str(),
            );
            context.set_text_baseline("bottom");
            context.set_text_align("right");
            context
                .fill_text(
                    "© 2023, Graphima",
                    right_x - font_cpx_size,
                    bottom_y - font_cpx_size * 0.5,
                )
                .unwrap();
        }
    }
    pub fn get_coord_ticks(&mut self, max_ticks: f64, time_us: f64) -> Vec<Tick> {
        let screen_area = self.get_content_screen_area(time_us);
        let min_as_normalized_global = screen_area
            .global_scale
            .normalize_coord(screen_area.scale.get_coord_min());
        let max_as_normalized_global = screen_area
            .global_scale
            .normalize_coord(screen_area.scale.get_coord_max());
        let mut ticks = self.coord_grid.get_ticks(
            time_us,
            min_as_normalized_global,
            max_as_normalized_global,
            max_ticks,
        );
        let screen_area = self.get_content_screen_area(time_us);
        for tick in ticks.iter_mut() {
            tick.value = screen_area
                .global_scale
                .denormalize_coord(tick.normalized_value);
        }
        ticks
    }
    pub fn get_value_ticks(&mut self, max_ticks: f64, time_us: f64) -> Vec<Tick> {
        let screen_area = self.get_content_screen_area(time_us);
        let min_as_normalized_global = screen_area
            .global_scale
            .normalize_value(screen_area.scale.get_value_min());
        let max_as_normalized_global = screen_area
            .global_scale
            .normalize_value(screen_area.scale.get_value_max());
        let mut ticks = self.value_grid.get_ticks(
            time_us,
            min_as_normalized_global,
            max_as_normalized_global,
            max_ticks,
        );
        let screen_area = self.get_content_screen_area(time_us);
        for tick in ticks.iter_mut() {
            tick.value = screen_area
                .global_scale
                .denormalize_value(tick.normalized_value);
        }
        ticks
    }
}
