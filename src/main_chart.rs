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
use crate::controls::{MouseControls, TouchControls, WatchControls};
use crate::events::JsEventListener;
use crate::legend::Legend;
use crate::params::{ChartConfig, ChartParams, ClientCaps, Content};
use crate::preview::Preview;
use crate::scale::Scale;
use crate::screen::{CoordSpaceHandle, Padding, Screen, ScreenArea, ScreenPos, Size};
use crate::tooltip::Tooltip;
use std::cell::RefCell;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::Ordering;

use wasm_bindgen::prelude::*;

const CSS_DISABLE_DEFAULT_LONG_TOUCH: &'static str =
    "-webkit-touch-callout: none !important; -webkit-user-select: none !important";
const CSS_DISABLE_TOUCH_GESTURES: &'static str = "touch-action: none";
pub trait DrawChart {
    fn on_control_event(&mut self, event: &ControlEvent, time_us: f64);
    fn on_resize(&mut self);
    fn draw(&mut self, time_us: f64);
}
pub struct MainChart<T>
where
    T: Scale,
{
    pub container_selector: String,
    pub client_caps: Rc<RefCell<ClientCaps>>,
    pub config: Rc<RefCell<ChartConfig>>,
    pub content: Content,
    pub content_screen: Rc<Screen>,
    pub control_screen: Rc<Screen>,

    pub preview: Preview<T>,
    pub camera: Camera<T>,

    pub legend: Legend,
    pub dirty: bool,

    control_watcher: Rc<RefCell<Box<dyn WatchControls>>>,
    touch_device: bool,
    pointer_down: Option<JsEventListener>,
    pointer_move: Option<JsEventListener>,
    pointer_out: Option<JsEventListener>,
    pointer_up: Option<JsEventListener>,
    animation_frame_requested: bool,
    request_animation_frame_closure: Option<Closure<dyn Fn(JsValue)>>,
    _pin: PhantomPinned,
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
        touch_device: bool,
    ) -> Result<Pin<Box<Self>>, String> {
        let config = Rc::new(RefCell::new(config));
        let conf = config.borrow();
        let content_screen = Rc::new(Screen::new(
            params.selector.as_str(),
            Rc::clone(&config),
            Rc::clone(&client_caps),
            format!("display: block; width: 100%; height: 100%").as_str(),
        )?);
        let control_screen = Rc::new(Screen::new(
            params.selector.as_str(),
            Rc::clone(&config),
            Rc::clone(&client_caps),
            format!(
                "display: block; width: 100%; height: 100%; position: absolute; left: 0; top: 0; {}; {}",
                CSS_DISABLE_DEFAULT_LONG_TOUCH,
                CSS_DISABLE_TOUCH_GESTURES,
            )
            .as_str(),
        )?);

        let content_padding = Padding::new([
            Size::Pct(0.0),
            Size::Pct(0.0),
            Size::Pct(conf.layout_preview_height + conf.layout_legend_height),
            Size::Pct(0.0),
        ]);
        let preview_padding = Padding::new([
            Size::Pct(conf.layout_content_height),
            Size::Pct(0.0),
            Size::Pct(conf.layout_legend_height),
            Size::Pct(0.0),
        ]);
        let legend_padding = Padding::new([
            Size::Pct(conf.layout_content_height + conf.layout_preview_height),
            Size::Pct(0.0),
            Size::Pct(0.0),
            Size::Pct(0.0),
        ]);

        let camera = Camera::new(
            Rc::clone(&config),
            Rc::clone(&client_caps),
            ScreenArea::new(Rc::clone(&content_screen), content_padding.clone()),
            ScreenArea::new(Rc::clone(&control_screen), content_padding),
            main_scale,
            Tooltip::new(Rc::clone(&config)),
            &mut params.content,
        );

        let preview = Preview::new(
            Rc::clone(&config),
            Rc::clone(&client_caps),
            ScreenArea::new(Rc::clone(&content_screen), preview_padding.clone())
                .sub_area(camera.content_padding.clone()),
            ScreenArea::new(Rc::clone(&control_screen), preview_padding)
                .sub_area(camera.content_padding.clone()),
            preview_scale,
            &mut params.content,
        );

        let legend = Legend::new(
            Rc::clone(&config),
            ScreenArea::new(Rc::clone(&control_screen), legend_padding)
                .sub_area(camera.content_padding.clone()),
        );

        let mut chart = Box::pin(Self {
            container_selector: params.selector.clone(),
            client_caps: Rc::clone(&client_caps),
            config: Rc::clone(&config),
            content: params.content,
            content_screen,
            control_screen,
            preview,
            camera,
            legend,
            dirty: true,
            control_watcher: Rc::new(RefCell::new(if touch_device {
                Box::new(TouchControls::new())
            } else {
                Box::new(MouseControls::new())
            })),
            touch_device,
            pointer_move: None,
            pointer_out: None,
            pointer_down: None,
            pointer_up: None,
            animation_frame_requested: false,
            request_animation_frame_closure: None,
            _pin: PhantomPinned,
        });
        Self::ensure_listeners_are_set_up(chart.as_mut());
        Ok(chart)
    }
    pub fn get_time_us() -> f64 {
        web_sys::window().unwrap().performance().unwrap().now() * 1000.0
    }
    fn ensure_listeners_are_set_up(mut self: Pin<&mut Self>) {
        let control_screen_event_target = self
            .control_screen
            .canvas
            .dyn_ref::<web_sys::EventTarget>()
            .unwrap()
            .clone();
        let is_touch_device = self.touch_device;
        let chart = unsafe { Pin::into_inner_unchecked(self.as_mut()) };
        let chart_ptr = chart as *mut Self as usize;

        chart.pointer_down = Some(JsEventListener::new(
            control_screen_event_target.clone(),
            if is_touch_device {
                "touchstart"
            } else {
                "mousedown"
            },
            Box::new(move |event: JsValue| {
                let mut obj = Box::into_pin(unsafe { Box::from_raw(chart_ptr as *mut Self) });
                let chart = unsafe { Pin::into_inner_unchecked(obj.as_mut()) };
                let event = chart.control_watcher.borrow_mut().down(&event);
                if let Some(control_event) = event {
                    let time_us = Self::get_time_us();
                    chart.on_control_event(&control_event, time_us);
                    chart.request_animation_frame();
                }
                Box::into_raw(unsafe { Pin::into_inner_unchecked(obj) });
            }),
        ));
        chart.pointer_up = Some(JsEventListener::new(
            control_screen_event_target.clone(),
            if is_touch_device {
                "touchend"
            } else {
                "mouseup"
            },
            Box::new(move |event: JsValue| {
                let mut obj = Box::into_pin(unsafe { Box::from_raw(chart_ptr as *mut Self) });
                let chart = unsafe { Pin::into_inner_unchecked(obj.as_mut()) };
                let event = chart.control_watcher.borrow_mut().up(&event);
                if let Some(control_event) = event {
                    let time_us = Self::get_time_us();
                    chart.on_control_event(&control_event, time_us);
                    chart.request_animation_frame();
                }
                Box::into_raw(unsafe { Pin::into_inner_unchecked(obj) });
            }),
        ));
        chart.pointer_move = Some(JsEventListener::new(
            control_screen_event_target.clone(),
            if is_touch_device {
                "touchmove"
            } else {
                "mousemove"
            },
            Box::new(move |event: JsValue| {
                let mut obj = Box::into_pin(unsafe { Box::from_raw(chart_ptr as *mut Self) });
                let chart = unsafe { Pin::into_inner_unchecked(obj.as_mut()) };
                let event = chart.control_watcher.borrow_mut().moved(&event);
                if let Some(control_event) = event {
                    let time_us = Self::get_time_us();
                    chart.on_control_event(&control_event, time_us);
                    chart.request_animation_frame();
                }
                Box::into_raw(unsafe { Pin::into_inner_unchecked(obj) });
            }),
        ));
        chart.pointer_out = Some(JsEventListener::new(
            control_screen_event_target.clone(),
            if is_touch_device {
                "touchcancel"
            } else {
                "mouseout"
            },
            Box::new(move |event: JsValue| {
                let mut obj = Box::into_pin(unsafe { Box::from_raw(chart_ptr as *mut Self) });
                let chart = unsafe { Pin::into_inner_unchecked(obj.as_mut()) };
                let event = chart.control_watcher.borrow_mut().left(&event);
                if let Some(control_event) = event {
                    let time_us = Self::get_time_us();
                    chart.on_control_event(&control_event, time_us);
                    chart.request_animation_frame();
                }
                Box::into_raw(unsafe { Pin::into_inner_unchecked(obj) });
            }),
        ));
        if chart.request_animation_frame_closure.is_none() {
            let closure = Closure::new(Box::new(move |time_ms: JsValue| {
                let time_us = time_ms.as_f64().unwrap() * 1000.0;
                let mut obj = Box::into_pin(unsafe { Box::from_raw(chart_ptr as *mut Self) });
                let chart = unsafe { Pin::into_inner_unchecked(obj.as_mut()) };
                chart.animation_frame_requested = false;
                chart.draw(time_us);
                Box::into_raw(unsafe { Pin::into_inner_unchecked(obj) });
            }));
            chart.request_animation_frame_closure = Some(closure);
        }
        chart.request_animation_frame();
    }
    fn request_animation_frame(&mut self) {
        if !self.animation_frame_requested {
            web_sys::window()
                .unwrap()
                .request_animation_frame(
                    self.request_animation_frame_closure
                        .as_ref()
                        .unwrap()
                        .as_ref()
                        .unchecked_ref(),
                )
                .unwrap();
            self.animation_frame_requested = true;
        }
    }

    fn drag_camera(&mut self, time_us: f64) {
        if let (Some(pos), Some(grip_hold_coord_offset)) =
            (&self.preview.pointer, self.preview.grip_hold_coord_offset)
        {
            let preview_coord_space = self.preview.control_coord_space.get_handle(time_us);
            if let Some(coord) = preview_coord_space.get_coord(pos) {
                self.camera
                    .move_to(&mut self.content, grip_hold_coord_offset + coord, time_us);
            }
        }
    }

    fn calc_selected_coords(
        &self,
        coord_space_handle: CoordSpaceHandle<T>,
        pos1: &ScreenPos,
        pos2: &ScreenPos,
    ) -> Option<(f64, f64)> {
        let screen_area_handle = coord_space_handle.screen_area_handle.as_ref();
        let (pos_left, pos_right) = if pos1.0 < pos2.0 {
            (pos1, pos2)
        } else {
            (pos2, pos1)
        };
        if screen_area_handle.get_cx(pos1) >= screen_area_handle.right_cx()
            || screen_area_handle.get_cx(pos2) <= screen_area_handle.left_cx()
        {
            None
        } else {
            let left_coord = coord_space_handle
                .get_coord(pos_left)
                .unwrap_or_else(|| coord_space_handle.scale.get_coord_min());

            let right_coord = coord_space_handle
                .get_coord(pos_right)
                .unwrap_or_else(|| coord_space_handle.scale.get_coord_max());
            Some((left_coord, right_coord))
        }
    }
    pub fn get_selected_coords(&mut self, time_us: f64) -> Option<(f64, f64)> {
        let mut selected_coords: Option<(f64, f64)> = None;
        if self.camera.pointer_down.is_some() {
            let handle = self.camera.coord_space.get_handle(time_us);
            if let (Some(down_pos), Some(pos)) = (&self.camera.pointer_down, &self.camera.pointer) {
                selected_coords = self.calc_selected_coords(handle, down_pos, pos);
            }
        }
        if self.preview.pointer_down.is_some() {
            if let (Some(down_pos), Some(pos)) = (&self.preview.pointer_down, &self.preview.pointer)
            {
                let handle = self.preview.coord_space.get_handle(time_us);
                selected_coords = self.calc_selected_coords(handle, down_pos, pos);
            }
        }
        selected_coords
    }
    fn draw_selected_area(&mut self, time_us: f64) {
        let selected_coords = self.get_selected_coords(time_us);
        if let Some((left_coord, right_coord)) = selected_coords {
            for coord_space in
                [&mut self.camera.coord_space, &mut self.preview.coord_space].into_iter()
            {
                let coord_space_handle = coord_space.get_handle(time_us);
                let coord_min = coord_space_handle.scale.get_coord_min();
                let coord_max = coord_space_handle.scale.get_coord_max();

                if left_coord >= coord_max || right_coord <= coord_min {
                    continue;
                }
                let left_coord = left_coord.max(coord_min);
                let right_coord = right_coord.min(coord_max);

                let screen_area_handle = coord_space_handle.screen_area_handle.as_ref();
                let crc = screen_area_handle.crc.as_ref();
                let conf = self.config.borrow();
                let v = conf.color_camera_grip;
                let color =
                    JsValue::from_str(format!("rgba({}, {}, {}, {})", v.0, v.1, v.2, v.3).as_str());

                let left_x = coord_space_handle.get_cx(left_coord);
                let right_x = coord_space_handle.get_cx(right_coord);
                let top_y = screen_area_handle.top_cy();
                let bottom_y = screen_area_handle.bottom_cy();
                crc.set_fill_style(&color);
                crc.fill_rect(left_x, top_y, right_x - left_x, bottom_y - top_y);
            }
        }
    }
    fn camera_pointer_up(&mut self, time_us: f64) {
        if let Some((left_coord, right_coord)) = self.get_selected_coords(time_us) {
            self.camera
                .zoom_by_coords(&mut self.content, left_coord, right_coord, time_us);
        }
        self.camera.pointer_down = None;
        self.camera.pointer_down_time_us = None;
    }
    fn preview_pointer_up(&mut self, time_us: f64) {
        if self.preview.grip_hold_coord_offset.is_none() {
            if let Some((left_coord, right_coord)) = self.get_selected_coords(time_us) {
                self.camera
                    .zoom_by_coords(&mut self.content, left_coord, right_coord, time_us);
            }
        } else {
            self.preview.grip_hold_coord_offset = None;
        }
        self.preview.pointer_down = None;
        self.preview.pointer_down_time_us = None;
    }
    fn legend_pointer_up(&mut self, time_us: f64) {
        self.legend.pointer_down = None;
        self.legend.pointer_down_time_us = None;
    }
    fn try_to_grab_camera_grip(&mut self, time_us: f64) {
        let camera_space = self.camera.control_coord_space.get_handle(time_us);
        let camera_coord_min = camera_space.scale.get_coord_min();
        let camera_coord_max = camera_space.scale.get_coord_max();
        let grip_coord = (camera_coord_min + camera_coord_max) * 0.5;

        if let Some(pos) = self.preview.pointer.clone() {
            let preview_space = self.preview.control_coord_space.get_handle(time_us);
            if let Some(coord) = preview_space.get_coord(&pos) {
                if coord <= camera_coord_max && coord >= camera_coord_min {
                    self.preview.grip_hold_coord_offset = Some(grip_coord - coord);
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
                let hit_camera = self
                    .camera
                    .control_coord_space
                    .screen_area
                    .get_handle()
                    .contains_pos(pos);
                if hit_camera {
                    self.camera.pointer = Some(pos.to_owned());
                    self.camera.pointer_down = Some(pos.to_owned());
                    self.camera.pointer_down_time_us = Some(time_us);
                } else {
                    let hit_preview = self
                        .preview
                        .control_coord_space
                        .screen_area
                        .get_handle()
                        .contains_pos(pos);
                    if hit_preview {
                        self.preview.pointer = Some(pos.to_owned());
                        self.preview.pointer_down = Some(pos.to_owned());
                        self.preview.pointer_down_time_us = Some(time_us);
                        if self.camera.zoomed_in {
                            self.try_to_grab_camera_grip(time_us);
                        }
                    } else {
                        let hit_legend = self
                            .legend
                            .control_screen_area
                            .get_handle()
                            .contains_pos(pos);
                        if hit_legend {
                            self.legend.pointer = Some(pos.to_owned());
                            self.legend.pointer_down = Some(pos.to_owned());
                            self.legend.pointer_down_time_us = Some(time_us);
                        }
                    };
                };
            }
            ControlEvent::PointerMoved { pos } => {
                let hit_camera = self
                    .camera
                    .control_coord_space
                    .screen_area
                    .get_handle()
                    .contains_pos(pos);
                if hit_camera {
                    self.camera.pointer = Some(pos.to_owned());
                } else {
                    if self.camera.pointer_down.is_some() {
                        self.camera_pointer_up(time_us);
                    }
                    self.camera.pointer = None;

                    let hit_preview = self
                        .preview
                        .control_coord_space
                        .screen_area
                        .get_handle()
                        .contains_pos(pos);

                    if hit_preview {
                        self.preview.pointer = Some(pos.to_owned());
                        if self.preview.pointer_down.is_some()
                            && self.preview.grip_hold_coord_offset.is_some()
                        {
                            self.drag_camera(time_us);
                        }
                    } else {
                        if self.preview.pointer_down.is_some() {
                            self.preview_pointer_up(time_us);
                        }
                        self.preview.pointer = None;

                        let hit_legend = self
                            .legend
                            .control_screen_area
                            .get_handle()
                            .contains_pos(pos);
                        if hit_legend {
                            self.legend.pointer = Some(pos.to_owned());
                        } else {
                            if self.legend.pointer_down.is_some() {
                                self.legend_pointer_up(time_us);
                            }
                            self.legend.pointer = None;
                        }
                    };
                }
            }
            ControlEvent::PointerUp | ControlEvent::PointerLeft => {
                if self.preview.pointer_down.is_some() {
                    self.preview_pointer_up(time_us);
                } else {
                    if self.camera.pointer_down.is_some() {
                        self.camera_pointer_up(time_us);
                    } else {
                        if self.legend.pointer_down.is_some() {
                            self.legend_pointer_up(time_us);
                        }
                    }
                }
                self.camera.pointer = None;
                self.preview.pointer = None;
                self.legend.pointer = None;
            }
            ControlEvent::PointerClicked => {
                if self.preview.pointer_down.is_some() {
                    if self.camera.zoomed_in {
                        self.camera.zoom_out(&mut self.content, time_us);
                    }
                    self.preview.pointer_down = None;
                    self.preview.pointer_down_time_us = None;
                    self.preview.grip_hold_coord_offset = None;
                } else {
                    if self.camera.pointer_down.is_some() {
                        if self.camera.pointer_clicked.is_some() {
                            self.camera.pointer_clicked = None;
                            self.camera.pointer_clicked_time_us = None;
                        } else {
                            self.camera.pointer_clicked = self.camera.pointer.clone();
                            self.camera.pointer_clicked_time_us = Some(time_us);
                        }
                        self.camera.pointer_down = None;
                        self.camera.pointer_down_time_us = None;
                    } else {
                        if self.legend.pointer_down.is_some() {
                            if self.legend.on_click(&mut self.content, time_us) {
                                self.camera.zoom_by_coords(
                                    &mut self.content,
                                    self.camera.control_coord_space.coord_min.get_end_value(),
                                    self.camera.control_coord_space.coord_max.get_end_value(),
                                    time_us,
                                );
                                self.preview
                                    .update_by_content(&mut self.content, Some(time_us));
                            }
                            self.legend.pointer_down = None;
                            self.legend.pointer_down_time_us = None;
                        }
                    }
                }
            }
            ControlEvent::PinchStarted { pos1, pos2 } => {
                if self.preview.pointer_down.is_some() {
                    self.preview_pointer_up(time_us);
                } else {
                    if self.camera.pointer_down.is_some() {
                        self.camera_pointer_up(time_us);
                    } else {
                        if self.legend.pointer_down.is_some() {
                            self.legend_pointer_up(time_us);
                        }
                    }
                }
                self.camera.pointer = None;
                self.preview.pointer = None;
                self.legend.pointer = None;

                let camera_coord_space_handle = self.camera.control_coord_space.get_handle(time_us);
                let camera_screen_area_handle =
                    camera_coord_space_handle.screen_area_handle.as_ref();
                if camera_screen_area_handle.contains_pos(pos1)
                    && camera_screen_area_handle.contains_pos(pos2)
                    && pos1.0 != pos2.0
                {
                    if let (Some(coord1), Some(coord2)) = (
                        camera_coord_space_handle.get_coord(pos1),
                        camera_coord_space_handle.get_coord(pos2),
                    ) {
                        self.camera.pinch_coords = Some(if coord1 < coord2 {
                            (coord1, coord2)
                        } else {
                            (coord2, coord1)
                        });
                    }

                    self.camera.pinch_coords;
                    self.camera.pointer_down = None;
                    self.camera.pointer_down_time_us = None;
                }
            }
            ControlEvent::PinchUpdated { pos1, pos2 } => {
                if pos1.0 == pos2.0 {
                    return;
                }
                let camera_coord_space_handle = self.camera.control_coord_space.get_handle(time_us);
                let camera_screen_area_handle =
                    camera_coord_space_handle.screen_area_handle.as_ref();
                if let Some((coord_1, coord_2)) = self.camera.pinch_coords.as_ref() {
                    let coord_min = self.camera.global_scale.get_coord_min();
                    let coord_max = self.camera.global_scale.get_coord_max();

                    let left_portion =
                        camera_screen_area_handle.get_cx(if pos1.0 < pos2.0 { pos1 } else { pos2 })
                            / camera_screen_area_handle.canvas_content_width;
                    let right_portion =
                        camera_screen_area_handle.get_cx(if pos1.0 > pos2.0 { pos1 } else { pos2 })
                            / camera_screen_area_handle.canvas_content_width;

                    let coord_to_portion = (coord_2 - coord_1) / (right_portion - left_portion);

                    let new_camera_left_coord =
                        ((0.0 - left_portion) * coord_to_portion + coord_1).max(coord_min);
                    let new_camera_right_coord =
                        ((1.0 - right_portion) * coord_to_portion + coord_2).min(coord_max);

                    self.camera.zoom_by_coords(
                        &mut self.content,
                        new_camera_left_coord,
                        new_camera_right_coord,
                        time_us,
                    );
                }
            }
            ControlEvent::PinchFinished => {
                self.camera.pinch_coords = None;
            }
        }
    }
    fn draw(&mut self, time_us: f64) {
        ANIMATED_NUMBERS_COUNT.store(0, Ordering::Relaxed);
        self.legend.on_long_press(&mut self.content, time_us);
        self.content_screen.clear();
        self.control_screen.clear();

        self.camera.draw(&mut self.content, time_us);
        self.preview.draw(&mut self.content, time_us);
        let grip = if self.camera.zoomed_in {
            let coord_min = self.camera.control_coord_space.coord_min.get_end_value();
            let coord_max = self.camera.control_coord_space.coord_max.get_end_value();
            Some(((coord_min + coord_max) * 0.5, coord_max - coord_min))
        } else {
            None
        };
        self.preview.draw_grip(grip, time_us);
        if self.preview.grip_hold_coord_offset.is_none() {
            self.draw_selected_area(time_us);
        }
        self.legend.draw(&self.content);

        if ANIMATED_NUMBERS_COUNT.load(Ordering::Relaxed) > 0
            || self.legend.pointer_down_time_us.is_some()
        {
            self.request_animation_frame();
        }
    }
    fn on_resize(&mut self) {
        self.content_screen.schedule_canvas_size_sync();
        self.control_screen.schedule_canvas_size_sync();
        self.request_animation_frame();
    }
}

// https://chartio.com/learn/charts/line-chart-complete-guide/
