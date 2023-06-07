/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::animate::ANIMATED_NUMBERS_COUNT;
use crate::camera::Camera;
use crate::controls::ControlEvent;
use crate::legend::Legend;
use crate::params::{ChartConfig, ChartParams, ClientCaps, Content};
use crate::scale::Scale;
use crate::screen::Screen;
use crate::tooltip::Tooltip;
use crate::utils::is_click;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::Ordering;

use wasm_bindgen::prelude::*;

fn get_tick_width(chart_config: &ChartConfig, font_size: f64, expected_len: usize) -> f64 {
    chart_config.font_width_coeff * font_size * expected_len as f64
}
fn get_tick_height(_chart_config: &ChartConfig, font_size: f64) -> f64 {
    font_size * 2.0
}
const CSS_DISABLE_DEFAULT_LONG_TOUCH: &'static str =
    "-webkit-touch-callout: none !important; -webkit-user-select: none !important";
const CSS_DISABLE_TOUCH_GESTURES: &'static str = "touch-action: none";
pub trait DrawChart {
    fn on_control_event(&mut self, event: &ControlEvent, time_us: f64);
    fn on_resize(&mut self);
    fn draw(&mut self, time_us: f64) -> usize;
}
pub struct MainChart<T>
where
    T: Scale,
{
    pub container_selector: String,
    pub client_caps: Rc<RefCell<ClientCaps>>,
    pub config: Rc<ChartConfig>,
    pub content: Content,
    pub pointer_position: Option<(f64, f64)>,
    pub camera_grip_x_offset: Option<f64>,
    pub camera_grip_pointer_down_position: Option<(f64, f64)>,
    pub camera_grip_screen: Screen,
    pub main_camera: Camera<T>,
    pub main_screen: Screen,
    pub preview_camera: Camera<T>,
    pub preview_screen: Screen,
    pub tooltip: Tooltip,
    pub tooltip_pointer_down_position: Option<(f64, f64)>,
    pub tooltip_pinch_coords: Option<(f64, f64)>,
    pub tooltip_screen: Screen,
    pub zoomed_in: bool,
    pub legend_screen: Screen,
    pub legend: Legend,
    pub legend_pointer_down_position: Option<(f64, f64)>,
    pub legend_pointer_down_time_us: Option<f64>,
    pub dirty: bool,
}
impl<T> MainChart<T>
where
    T: Scale,
{
    pub fn new(
        mut params: ChartParams,
        config: ChartConfig,
        client_caps: Rc<RefCell<ClientCaps>>,
        main_scale: T,
        preview_scale: T,
    ) -> Result<MainChart<T>, String> {
        let config = Rc::new(config);
        let main_screen = Screen::new(
            params.selector.as_str(),
            Rc::clone(&client_caps),
            format!(
                "display: block; width: 100%; height: {:.1}%",
                config.layout_content_height
            )
            .as_str(),
        )?;
        let tooltip_screen = Screen::new(
            params.selector.as_str(),
            Rc::clone(&client_caps),
            format!(
                "display: block; width: 100%; height: {:.1}%; position: absolute; left: 0; top: 0; {}; {}",
                config.layout_content_height,
                CSS_DISABLE_DEFAULT_LONG_TOUCH,
                CSS_DISABLE_TOUCH_GESTURES,
            )
            .as_str(),
        )?;
        let preview_screen = Screen::new(
            params.selector.as_str(),
            Rc::clone(&client_caps),
            format!(
                "display: block; width: 100%; height: {:.1}%",
                config.layout_preview_height
            )
            .as_str(),
        )?;
        let camera_grip_screen = Screen::new(
            params.selector.as_str(),
            Rc::clone(&client_caps),
            format!(
                "display: block; width: 100%; height: {:.1}%; position: absolute; left: 0; top: {:.1}%; {}; {}",
                config.layout_preview_height, config.layout_content_height, CSS_DISABLE_DEFAULT_LONG_TOUCH, CSS_DISABLE_TOUCH_GESTURES
            )
            .as_str(),
        )?;
        let legend_screen = Screen::new(
            params.selector.as_str(),
            Rc::clone(&client_caps),
            format!(
                "display: block; width: 100%; height: {:.0}%; {}",
                config.layout_legend_height, CSS_DISABLE_DEFAULT_LONG_TOUCH
            )
            .as_str(),
        )?;

        let coord_ticks_height = get_tick_height(config.as_ref(), config.font_size_small);
        let value_ticks_width = get_tick_width(
            config.as_ref(),
            config.font_size_small,
            params.content.value_short_verbose_len,
        );
        let main_camera_padding = [5.0, 0.0, coord_ticks_height, value_ticks_width];
        let preview_camera_padding = [0.0, main_camera_padding[1], 0.0, main_camera_padding[3]];

        let main_camera = Camera::new(
            Rc::clone(&config),
            main_scale,
            coord_ticks_height,
            value_ticks_width,
            &mut params.content,
            &main_screen,
            main_camera_padding,
        );
        let preview_camera = Camera::new(
            Rc::clone(&config),
            preview_scale,
            0.0,
            0.0,
            &mut params.content,
            &preview_screen,
            preview_camera_padding,
        );
        let legend = Legend::from_content(Rc::clone(&config), &params.content, &main_screen);
        let tooltip = Tooltip::new(Rc::clone(&config));
        let chart = MainChart {
            container_selector: params.selector.clone(),
            client_caps,
            config,
            content: params.content,
            pointer_position: None,
            main_camera,
            main_screen,
            tooltip,
            tooltip_pointer_down_position: None,
            tooltip_pinch_coords: None,
            tooltip_screen,
            preview_camera,
            preview_screen,
            camera_grip_x_offset: None,
            // TODO rename mouse to pointer
            camera_grip_pointer_down_position: None,
            camera_grip_screen,
            legend_screen,
            legend,
            legend_pointer_down_position: None,
            legend_pointer_down_time_us: None,
            zoomed_in: false,
            dirty: true,
        };
        Ok(chart)
    }

    fn zoom_out(&mut self, time_us: f64) {
        self.dirty = true;
        self.zoomed_in = false;
        let screen_area = self
            .preview_camera
            .get_content_screen_area(self.preview_camera.scale_time_us);
        self.main_camera.zoom_by_coords(
            &mut self.content,
            screen_area.scale.get_coord_min(),
            screen_area.scale.get_coord_max(),
            Some(time_us),
        );
    }
    fn try_to_grab_camera_grip(&mut self, time_us: f64) {
        self.dirty = true;
        if let Some((x, _)) = self.pointer_position {
            let screen_area = self
                .preview_camera
                .get_content_screen_area(self.preview_camera.scale_time_us);
            if let Some(mouse_coord) = screen_area.x_to_coord(x) {
                let grip_coord = self.main_camera.coord.get_value(time_us);
                let grip_coord_half_range = self.main_camera.coord_range.get_value(time_us) * 0.5;
                if mouse_coord >= grip_coord - grip_coord_half_range
                    && mouse_coord <= grip_coord + grip_coord_half_range
                {
                    self.camera_grip_x_offset = Some(x - screen_area.coord_to_x(grip_coord));
                }
            }
        }
    }

    fn drag_main_camera(&mut self, time_us: f64) {
        self.dirty = true;
        if let (Some((mouse_x, _)), Some(x_offset)) =
            (self.pointer_position, self.camera_grip_x_offset)
        {
            let screen_area = self
                .preview_camera
                .get_content_screen_area(self.preview_camera.scale_time_us);
            if let Some(new_grip_coord) = screen_area.x_to_coord(mouse_x - x_offset) {
                let half_range = self.main_camera.coord_range.get_end_value() * 0.5;
                let coord_min = screen_area.scale.get_coord_min();
                let coord_max = screen_area.scale.get_coord_max();

                let new_camera_coord = if new_grip_coord - half_range < coord_min {
                    coord_min + half_range
                } else if new_grip_coord + half_range > coord_max {
                    coord_max - half_range
                } else {
                    new_grip_coord
                };

                if new_camera_coord != self.main_camera.coord.get_end_value() {
                    self.main_camera
                        .move_to(&mut self.content, new_camera_coord, Some(time_us));
                }
            }
        }
    }

    fn toggle_data_set(&mut self, index: usize, time_us: f64) -> Result<(), String> {
        self.dirty = true;
        let number_of_data_sets = self.content.data_sets.len();
        if index >= number_of_data_sets {
            return Err(format!(
                "chart contains {number_of_data_sets} data sets; index is out of bound"
            ));
        }
        let is_visible = self.content.data_sets[index].alpha.get_end_value() == 1.0;
        if is_visible
            && self
                .content
                .data_sets
                .iter()
                .filter(|data_set| data_set.alpha.get_end_value() == 1.0)
                .count()
                == 1
        {
            for (index_, data_set) in self.content.data_sets.iter_mut().enumerate() {
                if index_ != index {
                    data_set.alpha.set_value(1.0, Some(time_us));
                }
            }
        } else {
            let alpha = &mut self.content.data_sets[index].alpha;
            alpha.set_value(1.0 - alpha.get_end_value(), Some(time_us));
        }
        self.update_cameras(time_us);
        Ok(())
    }
    fn update_cameras(&mut self, time_us: f64) {
        let coord = self.main_camera.coord.get_end_value();
        let coord_half_range = self.main_camera.coord_range.get_end_value() * 0.5;
        self.main_camera.zoom_by_coords(
            &mut self.content,
            coord - coord_half_range,
            coord + coord_half_range,
            Some(time_us),
        );
        self.preview_camera.update_by_content(
            &mut self.content,
            Some(time_us),
        );
    }
    fn handle_legend_click(&mut self, x: f64, y: f64, time_us: f64) {
        self.dirty = true;
        let mut clicked_index: Option<usize> = None;
        let cx = self.legend_screen.x_to_cx(x);
        let cy = self.legend_screen.y_to_cy(y);
        for (index, position) in self.legend.positions.iter().enumerate() {
            if position.contains(cx, cy) {
                clicked_index = Some(index);
                break;
            }
        }
        if let Some(index) = clicked_index {
            self.toggle_data_set(self.legend.offset + index, time_us)
                .unwrap();
        }
        if let Some(arrow_left) = &self.legend.arrow_left {
            if arrow_left.contains(cx, cy) {
                self.dirty = true;
                self.legend.prev_page();
            }
        }
        if let Some(arrow_right) = &self.legend.arrow_right {
            if arrow_right.contains(cx, cy) {
                self.dirty = true;
                self.legend.next_page();
            }
        }
    }
    fn check_legend_long_press(&mut self, time_us: f64) -> usize {
        if let Some(legend_pointer_down_time_us) = &self.legend_pointer_down_time_us {
            if time_us - *legend_pointer_down_time_us > self.config.us_long_press
                && is_click(&self.legend_pointer_down_position, &self.pointer_position)
            {
                let (x, y) = self.legend_pointer_down_position.as_ref().unwrap();

                let mut clicked_index: Option<usize> = None;
                let cx = self.legend_screen.x_to_cx(*x);
                let cy = self.legend_screen.y_to_cy(*y);
                for (index, position) in self.legend.positions.iter().enumerate() {
                    if position.contains(cx, cy) {
                        clicked_index = Some(index);
                        break;
                    }
                }
                if let Some(index) = clicked_index {
                    let index_to_show = index + self.legend.offset;
                    for (index, data_set) in self.content.data_sets.iter_mut().enumerate() {
                        data_set.alpha.set_value(
                            if index == index_to_show { 1.0 } else { 0.0 },
                            Some(time_us),
                        );
                    }
                    self.update_cameras(time_us);
                    self.dirty = true;
                    self.legend_pointer_down_position = None;
                    self.legend_pointer_down_time_us = None;
                }
            }
            1
        } else {
            0
        }
    }

    fn zoom_by_coords(&mut self, left_coord: f64, right_coord: f64, time_us: f64) {
        self.dirty = true;
        self.zoomed_in = true;
        self.main_camera
            .zoom_by_coords(&mut self.content, left_coord, right_coord, Some(time_us));
    }
    fn get_selected_coords(
        &mut self,
        is_main_screen: bool,
        mouse_x1: f64,
        mouse_x2: f64,
    ) -> Option<(f64, f64)> {
        let screen_area = if is_main_screen {
            self.main_camera
                .get_content_screen_area(self.main_camera.scale_time_us)
        } else {
            self.preview_camera
                .get_content_screen_area(self.preview_camera.scale_time_us)
        };

        let left_x = mouse_x1
            .min(mouse_x2)
            .max(screen_area.coord_to_x(screen_area.scale.get_coord_min()));
        let right_x = mouse_x1
            .max(mouse_x2)
            .min(screen_area.coord_to_x(screen_area.scale.get_coord_max()));
        match (
            screen_area.x_to_coord(left_x),
            screen_area.x_to_coord(right_x),
        ) {
            (Some(left_coord), Some(right_coord)) => Some((left_coord, right_coord)),
            _ => None,
        }
    }
    fn draw_selected_area(&mut self, time_us: f64) {
        if let (Some(down_pos), Some(pos)) =
            (self.tooltip_pointer_down_position, self.pointer_position)
        {
            if let Some((left_coord, right_coord)) =
                self.get_selected_coords(true, down_pos.0, pos.0)
            {
                let content_screen_area = self.main_camera.get_content_screen_area(time_us);
                let preview_screen_area = self.preview_camera.get_content_screen_area(time_us);
                let v = &self.config.color_camera_grip;
                let color =
                    JsValue::from_str(format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3).as_str());
                let left_x = content_screen_area.get_cx(left_coord);
                let right_x = content_screen_area.get_cx(right_coord);
                let top_y = content_screen_area.top_cy();
                let bottom_y = content_screen_area.bottom_cy();
                let context = &self.tooltip_screen.context;
                context.set_fill_style(&color);
                context.fill_rect(left_x, top_y, right_x - left_x, bottom_y - top_y);

                let left_x = preview_screen_area.get_cx(left_coord);
                let right_x = preview_screen_area.get_cx(right_coord);
                let top_y = preview_screen_area.top_cy();
                let bottom_y = preview_screen_area.bottom_cy();
                let context = &self.camera_grip_screen.context;
                context.set_fill_style(&color);
                context.fill_rect(left_x, top_y, right_x - left_x, bottom_y - top_y);
            }
            return;
        }

        if let (Some(down_pos), Some(pos)) = (
            self.camera_grip_pointer_down_position,
            self.pointer_position,
        ) {
            if let Some((left_coord, right_coord)) =
                self.get_selected_coords(false, down_pos.0, pos.0)
            {
                let content_screen_area = self.main_camera.get_content_screen_area(time_us);
                let preview_screen_area = self.preview_camera.get_content_screen_area(time_us);
                let v = &self.config.color_camera_grip;
                let color =
                    JsValue::from_str(format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3).as_str());
                let left_x = preview_screen_area.get_cx(left_coord);
                let right_x = preview_screen_area.get_cx(right_coord);
                let top_y = preview_screen_area.top_cy();
                let bottom_y = preview_screen_area.bottom_cy();
                let context = &self.camera_grip_screen.context;
                context.set_fill_style(&color);
                context.fill_rect(left_x, top_y, right_x - left_x, bottom_y - top_y);

                if right_coord >= content_screen_area.scale.get_coord_min()
                    && left_coord <= content_screen_area.scale.get_coord_max()
                {
                    let left_x = content_screen_area
                        .get_cx(left_coord)
                        .max(content_screen_area.left_cx());
                    let right_x = content_screen_area
                        .get_cx(right_coord)
                        .min(content_screen_area.right_cx());
                    let top_y = content_screen_area.top_cy();
                    let bottom_y = content_screen_area.bottom_cy();
                    let context = &self.tooltip_screen.context;
                    context.set_fill_style(&color);
                    context.fill_rect(left_x, top_y, right_x - left_x, bottom_y - top_y);
                }
            }
        }
    }
}
impl<T> DrawChart for MainChart<T>
where
    T: Scale,
{
    fn on_control_event(&mut self, event: &ControlEvent, time_us: f64) {
        match event {
            ControlEvent::PointerDown { pos } => {
                self.pointer_position = Some(pos.clone());
                if self.tooltip_screen.contains_pos(&pos) {
                    self.tooltip_pointer_down_position = Some(pos.clone());
                    self.dirty = true;
                }
                if self.camera_grip_screen.contains_pos(&pos) {
                    self.camera_grip_pointer_down_position = Some(pos.clone());
                    self.dirty = true;
                    if self.zoomed_in {
                        self.try_to_grab_camera_grip(time_us);
                    }
                }
                if self.legend_screen.contains_pos(&pos) {
                    self.legend_pointer_down_position = Some(pos.clone());
                    self.legend_pointer_down_time_us = Some(time_us);
                }
            }
            ControlEvent::PointerMoved { pos } => {
                self.pointer_position = Some(pos.clone());
                if self.tooltip_pointer_down_position.is_some()
                    || (self.tooltip_screen.contains_pos(&pos) || self.tooltip.visible)
                        && self.tooltip.mouse_click_at.is_none()
                {
                    self.dirty = true;
                }
                if self.camera_grip_pointer_down_position.is_some() {
                    self.dirty = true;
                    if self.camera_grip_x_offset.is_some() {
                        self.drag_main_camera(time_us);
                    }
                }
            }
            ControlEvent::PointerUp => {
                if self.tooltip_pointer_down_position.is_some() {
                    if is_click(&self.tooltip_pointer_down_position, &self.pointer_position) {
                        self.tooltip.mouse_click_at = if self.tooltip.mouse_click_at.is_none() {
                            self.pointer_position.clone()
                        } else {
                            None
                        };
                    } else {
                        // MOUSE UP AFTER DRAGGING
                        match (self.tooltip_pointer_down_position, self.pointer_position) {
                            (Some(down_pos), Some(pos)) => {
                                if let Some((left_coord, right_coord)) =
                                    self.get_selected_coords(true, down_pos.0, pos.0)
                                {
                                    self.zoom_by_coords(left_coord, right_coord, time_us);
                                }
                            }
                            _ => {}
                        }
                    }
                    self.tooltip_pointer_down_position = None;
                    self.dirty = true;
                }

                if self.camera_grip_pointer_down_position.is_some() {
                    if is_click(
                        &self.camera_grip_pointer_down_position,
                        &self.pointer_position,
                    ) {
                        self.zoom_out(time_us);
                    } else if self.camera_grip_x_offset.is_none() {
                        if let (Some(down_pos), Some(pos)) = (
                            self.camera_grip_pointer_down_position,
                            self.pointer_position,
                        ) {
                            if let Some((left_coord, right_coord)) =
                                self.get_selected_coords(false, down_pos.0, pos.0)
                            {
                                self.zoom_by_coords(left_coord, right_coord, time_us);
                            }
                        }
                    }
                    self.camera_grip_pointer_down_position = None;
                    self.camera_grip_x_offset = None;
                    self.dirty = true;
                }

                if self.legend_pointer_down_position.is_some() {
                    if is_click(&self.legend_pointer_down_position, &self.pointer_position) {
                        let pos = self.legend_pointer_down_position.as_ref().unwrap();
                        self.handle_legend_click(pos.0, pos.1, time_us);
                    }
                    self.legend_pointer_down_position = None;
                    self.legend_pointer_down_time_us = None;
                }
            }
            ControlEvent::PinchStarted { pos1, pos2 } => {
                if self.tooltip_screen.contains_pos(pos1)
                    && self.tooltip_screen.contains_pos(pos2)
                    && pos1.0 != pos2.0
                {
                    self.tooltip_pointer_down_position = None;
                    let main_screen_area = self
                        .main_camera
                        .get_content_screen_area(self.main_camera.scale_time_us);
                    match (
                        main_screen_area.x_to_coord(pos1.0),
                        main_screen_area.x_to_coord(pos2.0),
                    ) {
                        (Some(coord_1), Some(coord_2)) => {
                            self.tooltip_pinch_coords = if coord_1 < coord_2 {
                                Some((coord_1, coord_2))
                            } else {
                                Some((coord_2, coord_1))
                            };
                        }
                        _ => (),
                    }
                }
            }
            ControlEvent::PinchUpdated { pos1, pos2 } => {
                if pos1.0 != pos2.0 {
                    if let Some((coord_1, coord_2)) = self.tooltip_pinch_coords {
                        let main_screen_area = self
                            .main_camera
                            .get_content_screen_area(self.main_camera.scale_time_us);

                        let coord_min = main_screen_area.global_scale.get_coord_min();
                        let coord_max = main_screen_area.global_scale.get_coord_max();

                        let left_cx = main_screen_area.left_cx();

                        let pos1_x_portion = (main_screen_area.x_to_cx(pos1.0.min(pos2.0))
                            - left_cx)
                            / main_screen_area.canvas_content_width;
                        let pos2_x_portion = (main_screen_area.x_to_cx(pos1.0.max(pos2.0))
                            - left_cx)
                            / main_screen_area.canvas_content_width;

                        let coord_to_portion =
                            (coord_2 - coord_1) / (pos2_x_portion - pos1_x_portion);

                        let new_camera_left_coord =
                            ((0.0 - pos1_x_portion) * coord_to_portion + coord_1).max(coord_min);
                        let new_camera_right_coord =
                            ((1.0 - pos2_x_portion) * coord_to_portion + coord_2).min(coord_max);

                        self.zoom_by_coords(new_camera_left_coord, new_camera_right_coord, time_us);

                        if new_camera_left_coord == coord_min && new_camera_right_coord == coord_max
                        {
                            self.zoomed_in = false;
                        }
                    }
                }
            }
            ControlEvent::PinchFinished => {
                if self.tooltip_pinch_coords.is_some() {
                    self.tooltip_pinch_coords = None;
                }
            }
        }
    }
    fn on_resize(&mut self) {
        self.dirty = true;
        self.main_camera.dirty = true;
        self.preview_camera.dirty = true;
        self.main_screen.schedule_canvas_size_sync();
        self.tooltip_screen.schedule_canvas_size_sync();
        self.preview_screen.schedule_canvas_size_sync();
        self.camera_grip_screen.schedule_canvas_size_sync();
        self.legend_screen.schedule_canvas_size_sync();
    }
    fn draw(&mut self, time_us: f64) -> usize {
        let mut actions: usize = 0;
        actions += self.check_legend_long_press(time_us);
        if !self.dirty {
            return actions;
        }
        actions += 1;
        // console_debug!("DRAWING");
        ANIMATED_NUMBERS_COUNT.store(0, Ordering::Relaxed);

        // cameras sync their own screens themselves
        self.tooltip_screen.sync_canvas_size();
        self.camera_grip_screen.sync_canvas_size();
        self.legend_screen.sync_canvas_size();

        self.main_camera
            .shoot(&mut self.content, &mut self.main_screen, time_us);

        self.preview_camera
            .shoot(&mut self.content, &mut self.preview_screen, time_us);
        self.main_camera.draw_grip(
            &mut self.camera_grip_screen,
            self.preview_camera.get_content_screen_area(time_us),
            self.zoomed_in,
            self.camera_grip_x_offset.is_some(),
            time_us,
        );

        self.tooltip_screen.clear();
        if self.camera_grip_x_offset.is_none() {
            self.draw_selected_area(time_us);
        }
        self.tooltip.draw(
            &mut self.content,
            &mut self.tooltip_screen,
            self.main_camera.get_content_screen_area(time_us),
            if self.client_caps.borrow().touch_device {
                &None
            } else {
                &self.pointer_position
            },
            time_us,
        );

        let content_screen_area = self.main_camera.get_content_screen_area(time_us);
        self.legend.resize(
            &self.legend_screen,
            content_screen_area.left_cx(),
            content_screen_area.right_cx(),
            0.0,
            self.legend_screen.canvas_height,
        );
        self.legend
            .draw(&mut self.content, &mut self.legend_screen, time_us);

        if ANIMATED_NUMBERS_COUNT.load(Ordering::Relaxed) == 0 {
            self.dirty = false;
        }
        actions
    }
}

// https://chartio.com/learn/charts/line-chart-complete-guide/
