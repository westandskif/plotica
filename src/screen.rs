/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::animate::AnimatedNumber;
use crate::params::ChartConfig;
use crate::params::ClientCaps;
use crate::scale::Scale;
use crate::versioned::Versioned;
use js_sys::Reflect;
use std::cell::{Ref, RefCell};
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};

pub trait DefineSize {
    fn get_css_to_physical_scale(self) -> f64;
    fn get_font_width_to_physical_scale(self) -> f64;
    fn get_font_height_to_physical_scale(self) -> f64;
    fn get_content_width(self) -> f64;
    fn get_content_height(self) -> f64;
}

#[derive(Clone, Debug)]
pub enum Size {
    Px(f64),
    TextLine { font_size: f64, columns: f64 },
    Pct(f64),
}
impl Size {
    pub fn to_cpx_width<T>(&self, size_def: T) -> f64
    where
        T: DefineSize,
    {
        match self {
            Self::Px(v) => *v * size_def.get_css_to_physical_scale(),
            Self::TextLine { font_size, columns } => {
                *font_size * *columns as f64 * size_def.get_font_width_to_physical_scale()
            }
            Self::Pct(v) => *v * size_def.get_content_width(),
        }
    }
    pub fn to_cpx_height<T>(&self, size_def: T) -> f64
    where
        T: DefineSize,
    {
        match self {
            Self::Px(v) => *v * size_def.get_css_to_physical_scale(),
            Self::TextLine { font_size, .. } => {
                *font_size * size_def.get_font_height_to_physical_scale()
            }
            Self::Pct(v) => *v * size_def.get_content_height(),
        }
    }
    pub fn mul(&self, x: f64) -> Self {
        match self {
            Self::Px(v) => Self::Px(v * x),
            Self::TextLine { font_size, columns } => Self::TextLine {
                font_size: *font_size,
                columns: columns * x,
            },
            Self::Pct(v) => Self::Pct(v * x),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct ScreenPos(pub f64, pub f64);

#[derive(Clone)]
pub struct ScreenState {
    pub canvas_width: f64,
    pub canvas_height: f64,
    pub css_to_physical_scale: f64,
    pub font_height_to_physical_scale: f64,
    pub font_width_to_physical_scale: f64,
    sync_requests: usize,
    pub syncs: usize,
}

impl DefineSize for &ScreenState {
    #[inline]
    fn get_content_height(self) -> f64 {
        self.canvas_height
    }
    #[inline]
    fn get_content_width(self) -> f64 {
        self.canvas_width
    }
    #[inline]
    fn get_css_to_physical_scale(self) -> f64 {
        self.css_to_physical_scale
    }
    #[inline]
    fn get_font_height_to_physical_scale(self) -> f64 {
        self.font_height_to_physical_scale
    }
    #[inline]
    fn get_font_width_to_physical_scale(self) -> f64 {
        self.font_width_to_physical_scale
    }
}

pub struct Screen {
    config: Rc<RefCell<ChartConfig>>,
    client_caps: Rc<RefCell<ClientCaps>>,
    pub canvas: web_sys::HtmlCanvasElement,
    pub crc: Rc<web_sys::CanvasRenderingContext2d>,
    state: RefCell<ScreenState>,
}
impl Screen {
    pub fn new(
        container_selector: &str,
        config: Rc<RefCell<ChartConfig>>,
        client_caps: Rc<RefCell<ClientCaps>>,
        style: &str,
    ) -> Result<Screen, String> {
        let document = web_sys::window().unwrap().document().unwrap();
        let container = document
            .query_selector(container_selector)
            .unwrap()
            .ok_or_else(|| "container not found".to_string())?;

        let canvas = document
            .create_element("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        container.append_child(&canvas).unwrap();
        canvas.set_attribute("style", style).unwrap();

        let context_options = js_sys::Object::new();
        let crc = canvas
            .get_context_with_context_options("2d", &context_options)
            .unwrap()
            .ok_or_else(|| "failed to get canvas 2d crc".to_string())?
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();
        let result = Self {
            config: Rc::clone(&config),
            client_caps: Rc::clone(&client_caps),
            canvas,
            crc: Rc::new(crc),
            state: RefCell::new(ScreenState {
                canvas_width: 0.0,
                canvas_height: 0.0,
                css_to_physical_scale: 0.0,
                font_height_to_physical_scale: 0.0,
                font_width_to_physical_scale: 0.0,
                sync_requests: 1,
                syncs: 0,
            }),
        };
        result.sync_canvas_size();
        Ok(result)
    }
    pub fn schedule_canvas_size_sync(&self) {
        let mut state = self.state.borrow_mut();
        state.sync_requests = state.syncs + 1;
    }
    fn sync_canvas_size(&self) {
        let client_caps = self.client_caps.borrow();
        let conf = self.config.borrow();
        let canvas = &self.canvas;

        let rect = Reflect::get(&canvas, &JsValue::from_str("getBoundingClientRect"))
            .unwrap()
            .dyn_into::<js_sys::Function>()
            .unwrap()
            .call0(&canvas)
            .unwrap();

        let width = Reflect::get(&rect, &JsValue::from_str("width"))
            .unwrap()
            .as_f64()
            .unwrap()
            * client_caps.css_to_physical_scale;
        let height = Reflect::get(&rect, &JsValue::from_str("height"))
            .unwrap()
            .as_f64()
            .unwrap()
            * client_caps.css_to_physical_scale;

        canvas.set_width((width as usize).try_into().unwrap());
        canvas.set_height((height as usize).try_into().unwrap());

        let mut state = self.state.borrow_mut();
        state.canvas_width = width;
        state.canvas_height = height;
        state.css_to_physical_scale = client_caps.css_to_physical_scale;
        state.font_height_to_physical_scale = client_caps.css_to_physical_scale;
        state.font_width_to_physical_scale =
            client_caps.css_to_physical_scale * conf.font_width_coeff;
        state.syncs += 1;
    }
    pub fn get_state(&self) -> Ref<ScreenState> {
        let mut state = self.state.borrow();
        if state.sync_requests != state.syncs {
            drop(state);
            self.sync_canvas_size();
            state = self.state.borrow();
        }
        state
    }
    pub fn clear(&self) {
        let state = self.get_state();
        self.crc
            .clear_rect(0.0, 0.0, state.canvas_width, state.canvas_height);
    }
}

pub type Padding = Versioned<[Size; 4]>;

pub struct ScreenArea {
    screen: Rc<Screen>,
    screen_syncs: usize,
    paddings: Vec<Padding>,
    padding_versions: Vec<usize>,
    handle: Option<Rc<ScreenAreaHandle>>,
}

impl ScreenArea {
    pub fn new(screen: Rc<Screen>, padding: Padding) -> Self {
        Self {
            screen,
            screen_syncs: 0,
            paddings: vec![padding],
            padding_versions: vec![0],
            handle: None,
        }
    }
    pub fn sub_area(&self, padding: Padding) -> Self {
        let mut paddings: Vec<Padding> = Vec::with_capacity(self.paddings.len() + 1);
        let mut padding_versions: Vec<usize> = Vec::with_capacity(self.paddings.len() + 1);
        for item in self.paddings.iter() {
            paddings.push(item.clone());
            padding_versions.push(0);
        }
        paddings.push(padding);
        padding_versions.push(0);

        Self {
            screen: Rc::clone(&self.screen),
            screen_syncs: self.screen_syncs,
            paddings,
            padding_versions,
            handle: None,
        }
    }
    pub fn get_handle(&mut self) -> Rc<ScreenAreaHandle> {
        let screen_state = self.screen.get_state();
        if screen_state.syncs == self.screen_syncs
            && self
                .padding_versions
                .iter()
                .cloned()
                .eq(self.paddings.iter().map(|v| v.get().version))
        {
            return Rc::clone(self.handle.as_ref().unwrap());
        }
        let mut current_screen_state = self.screen.get_state().clone();
        let mut total_paddings = [0.0; 4];
        let mut padding_versions: Vec<usize> = Vec::with_capacity(self.paddings.len());
        let mut current_paddings = [0.0; 4];
        for item in self.paddings.iter() {
            let padding = item.get();
            let sizes = &padding.value;
            padding_versions.push(padding.version);
            current_paddings = [
                sizes[0].to_cpx_height(&current_screen_state),
                sizes[1].to_cpx_width(&current_screen_state),
                sizes[2].to_cpx_height(&current_screen_state),
                sizes[3].to_cpx_width(&current_screen_state),
            ];
            total_paddings[0] += current_paddings[0];
            total_paddings[1] += current_paddings[1];
            total_paddings[2] += current_paddings[2];
            total_paddings[3] += current_paddings[3];
            current_screen_state.canvas_width -= current_paddings[1] + current_paddings[3];
            current_screen_state.canvas_height -= current_paddings[0] + current_paddings[2];
        }
        let outer_padding = [
            total_paddings[0] - current_paddings[0],
            total_paddings[1] - current_paddings[1],
            total_paddings[2] - current_paddings[2],
            total_paddings[3] - current_paddings[3],
        ];
        let screen_state = self.screen.get_state();
        let handle = Rc::new(ScreenAreaHandle {
            crc: Rc::clone(&self.screen.crc),
            screen_width: screen_state.canvas_width,
            screen_height: screen_state.canvas_height,

            css_to_physical_scale: screen_state.css_to_physical_scale,
            font_height_to_physical_scale: screen_state.font_height_to_physical_scale,
            font_width_to_physical_scale: screen_state.font_width_to_physical_scale,

            outer_padding,
            canvas_content_width: current_screen_state.canvas_width,
            canvas_content_height: current_screen_state.canvas_height,
            canvas_padding: total_paddings,
        });
        self.handle = Some(Rc::clone(&handle));
        self.screen_syncs = screen_state.syncs;
        self.padding_versions = padding_versions;
        handle
    }
}

pub struct ScreenAreaHandle {
    pub crc: Rc<web_sys::CanvasRenderingContext2d>,
    pub screen_width: f64,
    pub screen_height: f64,

    pub css_to_physical_scale: f64,
    pub font_height_to_physical_scale: f64,
    pub font_width_to_physical_scale: f64,

    pub outer_padding: [f64; 4],
    pub canvas_content_width: f64,
    pub canvas_content_height: f64,
    pub canvas_padding: [f64; 4],
}
impl ScreenAreaHandle {
    #[inline]
    pub fn outer_left_cx(&self) -> f64 {
        self.outer_padding[3]
    }
    #[inline]
    pub fn outer_right_cx(&self) -> f64 {
        self.canvas_content_width + self.canvas_padding[3]
    }
    #[inline]
    pub fn outer_top_cy(&self) -> f64 {
        self.canvas_padding[0]
    }
    #[inline]
    pub fn outer_bottom_cy(&self) -> f64 {
        self.canvas_content_height + self.canvas_padding[0]
    }
    #[inline]
    pub fn outer_width(&self) -> f64 {
        self.canvas_content_width + self.canvas_padding[1] + self.canvas_padding[3]
            - self.outer_padding[1]
            - self.outer_padding[3]
    }
    #[inline]
    pub fn outer_height(&self) -> f64 {
        self.canvas_content_height + self.canvas_padding[0] + self.canvas_padding[2]
            - self.outer_padding[0]
            - self.outer_padding[2]
    }
    #[inline]
    pub fn left_cx(&self) -> f64 {
        self.canvas_padding[3]
    }
    #[inline]
    pub fn right_cx(&self) -> f64 {
        self.canvas_content_width + self.canvas_padding[3]
    }
    #[inline]
    pub fn top_cy(&self) -> f64 {
        self.canvas_padding[0]
    }
    #[inline]
    pub fn bottom_cy(&self) -> f64 {
        self.canvas_content_height + self.canvas_padding[0]
    }
    #[inline]
    pub fn get_cx(&self, pos: &ScreenPos) -> f64 {
        pos.0 * self.css_to_physical_scale
    }
    #[inline]
    pub fn get_cy(&self, pos: &ScreenPos) -> f64 {
        pos.1 * self.css_to_physical_scale
    }
    pub fn contains_pos(&self, pos: &ScreenPos) -> bool {
        let cx = self.get_cx(pos);
        if cx < self.canvas_padding[3] || cx > self.canvas_padding[3] + self.canvas_content_width {
            return false;
        }
        let cy = self.get_cy(pos);
        cy >= self.canvas_padding[0] && cy <= self.canvas_padding[0] + self.canvas_content_height
    }
    pub fn clear(&self) {
        self.crc.clear_rect(
            self.canvas_padding[3],
            self.canvas_padding[0],
            self.canvas_content_width,
            self.canvas_content_height,
        );
    }
    pub fn clear_outer(&self) {
        self.crc.clear_rect(
            self.outer_padding[3],
            self.outer_padding[0],
            self.canvas_content_width + self.canvas_padding[3] - self.outer_padding[3],
            self.canvas_content_height + self.canvas_padding[0] - self.outer_padding[0],
        );
    }
}
impl DefineSize for &ScreenAreaHandle {
    #[inline]
    fn get_css_to_physical_scale(self) -> f64 {
        self.css_to_physical_scale
    }
    #[inline]
    fn get_font_height_to_physical_scale(self) -> f64 {
        self.font_height_to_physical_scale
    }
    #[inline]
    fn get_font_width_to_physical_scale(self) -> f64 {
        self.font_width_to_physical_scale
    }
    #[inline]
    fn get_content_height(self) -> f64 {
        self.canvas_content_height
    }
    #[inline]
    fn get_content_width(self) -> f64 {
        self.canvas_content_width
    }
}

pub struct CoordSpace<T>
where
    T: Scale,
{
    pub screen_area: ScreenArea,
    pub coord_min: AnimatedNumber,
    pub coord_max: AnimatedNumber,
    pub value_min: AnimatedNumber,
    pub value_max: AnimatedNumber,
    scale: Option<Rc<T>>,
    scale_time_us: f64,
}
impl<T> CoordSpace<T>
where
    T: Scale,
{
    pub fn new(screen_area: ScreenArea, scale: T) -> Self {
        Self {
            screen_area,
            coord_min: AnimatedNumber::new(scale.get_coord_min()),
            coord_max: AnimatedNumber::new(scale.get_coord_max()),
            value_min: AnimatedNumber::new(scale.get_value_min()),
            value_max: AnimatedNumber::new(scale.get_value_max()),
            scale: Some(Rc::new(scale)),
            scale_time_us: 0.0,
        }
    }

    pub fn content_updated(
        &mut self,
        coord_min: f64,
        coord_max: f64,
        value_min: f64,
        value_max: f64,
        time_us: Option<f64>,
    ) {
        self.coord_min.set_value(coord_min, time_us);
        self.coord_max.set_value(coord_max, time_us);
        self.value_min.set_value(value_min, time_us);
        self.value_max.set_value(value_max, time_us);
        self.scale_time_us = 0.0;
    }

    pub fn get_handle(&mut self, time_us: f64) -> CoordSpaceHandle<T> {
        if time_us != self.scale_time_us {
            let mut scale = Rc::into_inner(self.scale.take().unwrap()).unwrap();
            scale.reframe(
                self.coord_min.get_value(time_us),
                self.coord_max.get_value(time_us),
                self.value_min.get_value(time_us),
                self.value_max.get_value(time_us),
            );
            self.scale = Some(Rc::new(scale));
            self.scale_time_us = time_us;
        }
        CoordSpaceHandle {
            screen_area_handle: self.screen_area.get_handle(),
            scale: Rc::clone(self.scale.as_ref().unwrap()),
        }
    }
}
pub struct CoordSpaceHandle<T>
where
    T: Scale,
{
    pub screen_area_handle: Rc<ScreenAreaHandle>,
    pub scale: Rc<T>,
}

impl<T> CoordSpaceHandle<T>
where
    T: Scale,
{
    #[inline]
    pub fn get_cx(&self, coord: f64) -> f64 {
        self.scale.normalize_coord(coord) * self.screen_area_handle.canvas_content_width
            + self.screen_area_handle.canvas_padding[3]
    }
    #[inline]
    pub fn get_cy(&self, value: f64) -> f64 {
        (1.0 - self.scale.normalize_value(value)) * self.screen_area_handle.canvas_content_height
            + self.screen_area_handle.canvas_padding[0]
    }

    pub fn get_coord(&self, pos: &ScreenPos) -> Option<f64> {
        let normalized_coord = (pos.0 * self.screen_area_handle.css_to_physical_scale
            - self.screen_area_handle.canvas_padding[3])
            / self.screen_area_handle.canvas_content_width;
        if normalized_coord >= 0.0 && normalized_coord <= 1.0 {
            Some(self.scale.denormalize_coord(normalized_coord))
        } else {
            None
        }
    }
    pub fn get_value(&self, pos: &ScreenPos) -> Option<f64> {
        let normalized_value = 1.0
            - (pos.1 * self.screen_area_handle.css_to_physical_scale
                - self.screen_area_handle.canvas_padding[0])
                / self.screen_area_handle.canvas_content_height;
        if normalized_value >= 0.0 && normalized_value <= 1.0 {
            Some(self.scale.denormalize_value(normalized_value))
        } else {
            None
        }
    }
}
impl<T> DefineSize for &CoordSpaceHandle<T>
where
    T: Scale,
{
    #[inline]
    fn get_css_to_physical_scale(self) -> f64 {
        self.screen_area_handle.css_to_physical_scale
    }
    #[inline]
    fn get_font_height_to_physical_scale(self) -> f64 {
        self.screen_area_handle.font_height_to_physical_scale
    }
    #[inline]
    fn get_font_width_to_physical_scale(self) -> f64 {
        self.screen_area_handle.font_width_to_physical_scale
    }
    #[inline]
    fn get_content_height(self) -> f64 {
        self.screen_area_handle.canvas_content_height
    }
    #[inline]
    fn get_content_width(self) -> f64 {
        self.screen_area_handle.canvas_content_width
    }
}

pub struct ScreenRect {
    pub cx1: f64,
    pub cy1: f64,
    pub cx2: f64,
    pub cy2: f64,
}
impl ScreenRect {
    pub fn from_width(cx: f64, cy: f64, width: f64, height: f64) -> Self {
        Self {
            cx1: cx,
            cy1: cy,
            cx2: cx + width,
            cy2: cy + height,
        }
    }
    pub fn contains(&self, cx: f64, cy: f64) -> bool {
        self.cx1 <= cx && self.cx2 >= cx && self.cy1 <= cy && self.cy2 >= cy
    }
    #[inline]
    pub fn cx_center(&self) -> f64 {
        (self.cx1 + self.cx2) * 0.5
    }
    #[inline]
    pub fn cy_center(&self) -> f64 {
        (self.cy1 + self.cy2) * 0.5
    }
    #[inline]
    pub fn width(&self) -> f64 {
        self.cx2 - self.cx1
    }
    #[inline]
    pub fn height(&self) -> f64 {
        self.cy2 - self.cy1
    }
}
