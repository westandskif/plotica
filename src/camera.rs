/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::grid::{Grid, Tick};
use crate::params::Content;
use crate::params::{ChartConfig, ClientCaps, VerboseFormat};
use crate::scale::Scale;
use crate::screen::{CoordSpace, Padding, ScreenArea, ScreenPos, Size};
use crate::tooltip::Tooltip;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsValue;

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
    pub chart_config: Rc<RefCell<ChartConfig>>,
    pub client_caps: Rc<RefCell<ClientCaps>>,

    pub coord_space: CoordSpace<T>,
    pub control_coord_space: CoordSpace<T>,

    pub global_scale: T,
    pub content_padding: Padding,

    pub tooltip: Tooltip,
    pub coord_grid: Grid,
    pub value_grid: Grid,

    pub scale_time_us: f64,

    pub pointer_down: Option<ScreenPos>,
    pub pointer_down_time_us: Option<f64>,
    pub pointer: Option<ScreenPos>,
    pub pointer_clicked: Option<ScreenPos>,
    pub pointer_clicked_time_us: Option<f64>,
    pub pinch_coords: Option<(f64, f64)>,
    pub zoomed_in: bool,

    pub dirty: bool,
}
impl<T> Camera<T>
where
    T: Scale,
{
    pub fn new(
        chart_config: Rc<RefCell<ChartConfig>>,
        client_caps: Rc<RefCell<ClientCaps>>,
        screen_area: ScreenArea,
        control_screen_area: ScreenArea,
        scale: T,
        tooltip: Tooltip,
        content: &mut Content,
    ) -> Self {
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
        let content_padding =
            Padding::new([Size::Px(0.0), Size::Px(0.0), Size::Px(0.0), Size::Px(0.0)]);
        let mut camera = Self {
            chart_config,
            client_caps,
            coord_space: CoordSpace::new(
                screen_area.sub_area(content_padding.clone()),
                scale.clone(),
            ),
            control_coord_space: CoordSpace::new(
                control_screen_area.sub_area(content_padding.clone()),
                scale.clone(),
            ),
            global_scale: scale,
            content_padding,

            tooltip,
            coord_grid,
            value_grid,
            scale_time_us: 0.0,
            pointer_down: None,
            pointer_down_time_us: None,
            pointer: None,
            pointer_clicked: None,
            pointer_clicked_time_us: None,
            pinch_coords: None,
            zoomed_in: false,

            dirty: false,
        };
        camera.update_by_content(content, None);
        camera
    }
    pub fn update_by_content(&mut self, content: &mut Content, time_us: Option<f64>) {
        let conf = self.chart_config.borrow();
        *self.content_padding.get_mut() = [
            Size::Px(0.0),
            Size::Px(0.0),
            conf.font_size_small
                .mul(content.coord_short_verbose_len as f64),
            conf.font_size_small
                .mul(content.value_short_verbose_len as f64),
        ];
        let [coord_min, coord_max, value_min, value_max] = content.get_min_max();
        self.coord_space
            .content_updated(coord_min, coord_max, value_min, value_max, time_us);
        self.control_coord_space
            .content_updated(coord_min, coord_max, value_min, value_max, time_us);
    }
    pub fn zoom_by_coords(
        &mut self,
        content: &mut Content,
        coord_start: f64,
        coord_end: f64,
        time_us: f64,
    ) {
        if coord_end <= coord_start {
            return;
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
            self.zoomed_in = !(self.global_scale.get_coord_min() == coord_start
                && self.global_scale.get_coord_max() == coord_end);
            self.coord_space.content_updated(
                coord_start,
                coord_end,
                value_min,
                value_max,
                Some(time_us),
            );
            self.control_coord_space.content_updated(
                coord_start,
                coord_end,
                value_min,
                value_max,
                Some(time_us),
            );
        }
    }
    pub fn zoom_out(&mut self, content: &mut Content, time_us: f64) {
        self.zoomed_in = false;
        self.update_by_content(content, Some(time_us));
    }
    pub fn move_to(&mut self, content: &mut Content, coord_center: f64, time_us: f64) {
        let mut coord_space_handle = self.coord_space.get_handle(time_us);
        let scale = &mut coord_space_handle.scale;

        let coord_half_range = (scale.get_coord_max() - scale.get_coord_min()) * 0.5;

        let coord_min = self.global_scale.get_coord_min();
        let coord_max = self.global_scale.get_coord_max();
        let mut coord_start = coord_center - coord_half_range;
        let mut coord_end = coord_center + coord_half_range;

        if coord_start < coord_min {
            coord_end += coord_min - coord_start;
            coord_start = coord_min;
        } else if coord_end > coord_max {
            coord_start -= coord_end - coord_max;
            coord_end = coord_max;
        }
        self.zoom_by_coords(content, coord_start, coord_end, time_us);
    }
    pub fn draw(&mut self, content: &mut Content, time_us: f64) {
        let coord_space_handle = self.coord_space.get_handle(time_us);
        let screen_area_handle = coord_space_handle.screen_area_handle.as_ref();
        let crc = screen_area_handle.crc.as_ref();

        let ticks = self.get_coord_ticks(content.coord_short_verbose_len as f64, time_us);
        self.draw_grid(ticks.as_slice(), Axis::X, time_us);
        self.draw_ticks(content, ticks.as_slice(), Axis::X, time_us);

        let ticks = self.get_value_ticks(time_us);
        self.draw_grid(ticks.as_slice(), Axis::Y, time_us);
        self.draw_ticks(content, ticks.as_slice(), Axis::Y, time_us);

        let config = self.chart_config.borrow();
        let mut alpha: f64;
        for data_set in content.data_sets.iter_mut() {
            alpha = data_set.alpha.get_value(time_us);
            if alpha == 0.0 {
                continue;
            }
            if let Some(data_points) = data_set.slice_by_coord(
                coord_space_handle.scale.get_coord_min(),
                coord_space_handle.scale.get_coord_max(),
            ) {
                let mut it = data_points.iter();
                let data_point = it.next().unwrap();
                crc.begin_path();
                crc.set_stroke_style(&JsValue::from_str(data_set.to_css_color(alpha).as_str()));
                crc.set_line_width(config.line_width.to_cpx_height(screen_area_handle));

                let mut prev_x = coord_space_handle.get_cx(data_point.coord);
                let mut prev_y = coord_space_handle.get_cy(data_point.value);
                crc.move_to(prev_x, prev_y);

                let mut min_y = f64::MAX;
                let mut max_y = f64::MIN;

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
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
                crc.stroke();
            }
        }

        if self.pointer_down.is_none() {
            self.tooltip.draw(
                content,
                self.control_coord_space.get_handle(time_us),
                self.pointer_clicked.as_ref().or(self.pointer.as_ref()),
                &self.global_scale,
                time_us,
            );
        }
    }
    fn draw_grid(&mut self, ticks: &[Tick], axis: Axis, time_us: f64) {
        let config = self.chart_config.borrow();
        let coord_space_handle = self.coord_space.get_handle(time_us);
        let screen_area_handle = coord_space_handle.screen_area_handle.as_ref();
        let crc = screen_area_handle.crc.as_ref();

        let mut alpha: f64 = -1.0;
        crc.set_line_width(1.0);
        let v = config.color_grid;
        match axis {
            Axis::X => {
                for tick in ticks.iter() {
                    crc.begin_path();
                    if tick.alpha != alpha {
                        crc.set_stroke_style(&JsValue::from_str(
                            format!("rgb({}, {}, {}, {:.3})", v.0, v.1, v.2, tick.alpha).as_str(),
                        ));
                        alpha = tick.alpha;
                    }
                    crc.move_to(
                        coord_space_handle.get_cx(tick.value),
                        screen_area_handle.bottom_cy(),
                    );
                    crc.line_to(
                        coord_space_handle.get_cx(tick.value),
                        screen_area_handle.top_cy(),
                    );
                    crc.stroke();
                }
            }
            Axis::Y => {
                for tick in ticks.iter() {
                    crc.begin_path();
                    if tick.alpha != alpha {
                        crc.set_stroke_style(&JsValue::from_str(
                            format!("rgb({}, {}, {}, {:.3})", v.0, v.1, v.2, tick.alpha).as_str(),
                        ));
                        alpha = tick.alpha;
                    }
                    crc.move_to(
                        screen_area_handle.left_cx(),
                        coord_space_handle.get_cy(tick.value),
                    );
                    crc.line_to(
                        screen_area_handle.right_cx(),
                        coord_space_handle.get_cy(tick.value),
                    );
                    crc.stroke();
                }
            }
        }
    }
    fn draw_ticks(&mut self, content: &Content, ticks: &[Tick], axis: Axis, time_us: f64) {
        let config = self.chart_config.borrow();
        let coord_space_handle = self.coord_space.get_handle(time_us);
        let screen_area_handle = coord_space_handle.screen_area_handle.as_ref();

        let verbose_format: &VerboseFormat;
        let min_value: f64;
        let max_value: f64;
        match axis {
            Axis::X => {
                verbose_format = &content.coord_verbose_format_short;
                min_value = coord_space_handle.scale.get_coord_min();
                max_value = coord_space_handle.scale.get_coord_max();
            }
            Axis::Y => {
                verbose_format = &content.value_verbose_format_short;
                min_value = coord_space_handle.scale.get_value_min();
                max_value = coord_space_handle.scale.get_value_max();
            }
        }
        let formatted_ticks = verbose_format.format_values(
            ticks.iter(),
            |tick: &Tick| tick.value,
            min_value,
            max_value,
        );

        let crc = screen_area_handle.crc.as_ref();
        crc.set_font(
            format!(
                "{}px {}",
                config.font_size_small.to_cpx_height(screen_area_handle),
                config.font_standard.as_str()
            )
            .as_str(),
        );

        let tick_color = config.color_tick;

        match axis {
            Axis::X => {
                let mut alpha: f64 = -1.0;
                let font_height = config.font_size_small.to_cpx_height(screen_area_handle);
                let y = screen_area_handle.bottom_cy() + font_height * 0.5;
                crc.set_text_align("center");
                crc.set_text_baseline("middle");
                for (tick, formatted_tick) in ticks.iter().zip(formatted_ticks.iter()) {
                    if tick.alpha != alpha {
                        crc.set_fill_style(&JsValue::from_str(
                            format!(
                                "rgba({}, {}, {}, {:.3})",
                                tick_color.0, tick_color.1, tick_color.2, tick.alpha
                            )
                            .as_str(),
                        ));
                        alpha = tick.alpha;
                    }
                    crc.fill_text(
                        formatted_tick.as_str(),
                        coord_space_handle.get_cx(tick.value),
                        y,
                    )
                    .unwrap();
                }
            }
            Axis::Y => {
                let mut alpha: f64 = -1.0;
                let ticks_width = config.font_size_small.to_cpx_width(screen_area_handle)
                    * content.value_short_verbose_len as f64;
                let left_cx = screen_area_handle.left_cx();

                let x = left_cx - ticks_width * 0.5;
                crc.set_text_align("center");
                crc.set_text_baseline("middle");
                for (tick, formatted_tick) in ticks.iter().zip(formatted_ticks.iter()) {
                    if tick.alpha != alpha {
                        crc.set_fill_style(&JsValue::from_str(
                            format!(
                                "rgba({}, {}, {}, {:.3})",
                                tick_color.0, tick_color.1, tick_color.2, tick.alpha
                            )
                            .as_str(),
                        ));
                        alpha = tick.alpha;
                    }
                    crc.fill_text(
                        formatted_tick.as_str(),
                        x,
                        coord_space_handle.get_cy(tick.value),
                    )
                    .unwrap();
                }
            }
        }
    }

    fn get_coord_ticks(&mut self, coord_short_verbose_len: f64, time_us: f64) -> Vec<Tick> {
        let config = self.chart_config.borrow();
        let coord_space_handle = self.coord_space.get_handle(time_us);
        let screen_area_handle = coord_space_handle.screen_area_handle.as_ref();
        let max_ticks = screen_area_handle.canvas_content_width
            / (config.font_size_small.to_cpx_width(screen_area_handle)
                * coord_short_verbose_len
                * COORD_TICKS_DUTY_FACTOR);

        let min_as_normalized_global = self
            .global_scale
            .normalize_coord(coord_space_handle.scale.get_coord_min());

        let max_as_normalized_global = self
            .global_scale
            .normalize_coord(coord_space_handle.scale.get_coord_max());

        let mut ticks = self.coord_grid.get_ticks(
            time_us,
            min_as_normalized_global,
            max_as_normalized_global,
            max_ticks,
        );
        for tick in ticks.iter_mut() {
            tick.value = self.global_scale.denormalize_coord(tick.normalized_value);
        }
        ticks
    }

    fn get_value_ticks(&mut self, time_us: f64) -> Vec<Tick> {
        let config = self.chart_config.borrow();
        let coord_space_handle = self.coord_space.get_handle(time_us);
        let screen_area_handle = coord_space_handle.screen_area_handle.as_ref();
        let max_ticks = screen_area_handle.canvas_content_height
            / (config.font_size_small.to_cpx_height(screen_area_handle) * VALUE_TICKS_DUTY_FACTOR);

        let min_as_normalized_global = self
            .global_scale
            .normalize_value(coord_space_handle.scale.get_value_min());
        let max_as_normalized_global = self
            .global_scale
            .normalize_value(coord_space_handle.scale.get_value_max());
        let mut ticks = self.value_grid.get_ticks(
            time_us,
            min_as_normalized_global,
            max_as_normalized_global,
            max_ticks,
        );
        for tick in ticks.iter_mut() {
            tick.value = self.global_scale.denormalize_value(tick.normalized_value);
        }
        ticks
    }
}
