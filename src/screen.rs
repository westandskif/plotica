/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::params::ClientCaps;
use crate::scale::Scale;
use crate::utils::js_coords_to_global;
use js_sys::Reflect;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};

pub struct ScreenArea<T>
where
    T: Scale,
{
    pub global_scale: T,
    pub scale: T,

    pub screen_x: f64,
    pub screen_y: f64,
    pub screen_to_canvas_scale: f64,

    pub canvas_content_width: f64,
    pub canvas_content_height: f64,

    pub canvas_width: f64,
    pub canvas_height: f64,
    pub canvas_padding: [f64; 4],
}
impl<T> ScreenArea<T>
where
    T: Scale,
{
    pub fn new(scale: T, screen: &Screen, padding: [f64; 4]) -> ScreenArea<T> {
        let mut screen_area = ScreenArea {
            global_scale: scale.clone(),
            scale,
            screen_x: 0.0,
            screen_y: 0.0,
            screen_to_canvas_scale: 0.0,
            canvas_content_width: 0.0,
            canvas_content_height: 0.0,
            canvas_width: 0.0,
            canvas_height: 0.0,
            canvas_padding: padding.map(|v| screen.apx_to_cpx(v)),
        };
        screen_area.update(screen);
        screen_area
    }
    pub fn update(&mut self, screen: &Screen) {
        self.screen_x = screen.x;
        self.screen_y = screen.y;
        self.screen_to_canvas_scale = screen.css_px_to_cpx(1.0);
        self.canvas_width = screen.css_px_to_cpx(screen.width);
        self.canvas_height = screen.css_px_to_cpx(screen.height);
        self.canvas_content_width =
            self.canvas_width - self.canvas_padding[1] - self.canvas_padding[3];
        self.canvas_content_height =
            self.canvas_height - self.canvas_padding[0] - self.canvas_padding[2];
    }
    #[inline]
    pub fn get_cx(&self, coord: f64) -> f64 {
        self.scale.normalize_coord(coord) * self.canvas_content_width + self.canvas_padding[3]
    }
    #[inline]
    pub fn get_cy(&self, value: f64) -> f64 {
        (1.0 - self.scale.normalize_value(value)) * self.canvas_content_height
            + self.canvas_padding[0]
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
    pub fn get_content_cwidth(&self) -> f64 {
        self.canvas_content_width
    }
    #[inline]
    pub fn get_content_cheight(&self) -> f64 {
        self.canvas_content_height
    }
    pub fn x_to_coord(&self, x: f64) -> Option<f64> {
        let normalized_coord = ((x - self.screen_x) * self.screen_to_canvas_scale
            - self.canvas_padding[3])
            / self.canvas_content_width;
        if normalized_coord >= 0.0 && normalized_coord <= 1.0 {
            Some(self.scale.denormalize_coord(normalized_coord))
        } else {
            None
        }
    }
    pub fn y_to_value(&self, y: f64) -> Option<f64> {
        let normalized_value = 1.0
            - ((y - self.screen_y) * self.screen_to_canvas_scale - self.canvas_padding[0])
                / self.canvas_content_height;
        if normalized_value >= 0.0 && normalized_value <= 1.0 {
            Some(self.scale.denormalize_value(normalized_value))
        } else {
            None
        }
    }
    pub fn coord_to_x(&self, coord: f64) -> f64 {
        self.get_cx(coord) / self.screen_to_canvas_scale + self.screen_x
    }
    // pub fn value_to_y(&self, value: f64) -> f64 {
    //     self.get_cy(value) / self.screen_to_canvas_scale + self.screen_y
    // }
    pub fn x_to_cx(&self, x: f64) -> f64 {
        (x - self.screen_x) * self.screen_to_canvas_scale
    }
    pub fn y_to_cy(&self, y: f64) -> f64 {
        (y - self.screen_y) * self.screen_to_canvas_scale
    }
}

pub struct Screen {
    pub canvas: web_sys::HtmlCanvasElement,
    pub context: web_sys::CanvasRenderingContext2d,
    client_caps: Rc<RefCell<ClientCaps>>,
    pub css_to_physical_scale: f64,
    device_pixel_ratio: f64,
    pub canvas_size_sync_needed: bool,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub canvas_width: f64,
    pub canvas_height: f64,
}
impl Screen {
    pub fn new(
        container_selector: &str,
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
        let context = canvas
            .get_context_with_context_options("2d", &context_options)
            .unwrap()
            .ok_or_else(|| "failed to get canvas 2d context".to_string())?
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();
        let mut screen = Screen {
            canvas,
            context,
            client_caps,
            css_to_physical_scale: 0.0,
            device_pixel_ratio: 0.0,
            canvas_size_sync_needed: true,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            canvas_width: 0.0,
            canvas_height: 0.0,
        };
        screen.sync_canvas_size();
        Ok(screen)
    }
    pub fn sync_canvas_size(&mut self) -> bool {
        if self.canvas_size_sync_needed {
            let client_caps = self.client_caps.borrow();
            let force_size_update_needed =
                self.css_to_physical_scale != client_caps.css_to_physical_scale;
            if force_size_update_needed {
                self.css_to_physical_scale = client_caps.css_to_physical_scale;
                self.device_pixel_ratio = client_caps.device_pixel_ratio;
            }

            let rect = Reflect::get(&self.canvas, &JsValue::from_str("getBoundingClientRect"))
                .unwrap()
                .dyn_into::<js_sys::Function>()
                .unwrap()
                .call0(&self.canvas)
                .unwrap();
            let x = Reflect::get(&rect, &JsValue::from_str("left"))
                .unwrap()
                .as_f64()
                .unwrap();
            let y = Reflect::get(&rect, &JsValue::from_str("top"))
                .unwrap()
                .as_f64()
                .unwrap();
            (self.x, self.y) = js_coords_to_global(x, y);

            let width = Reflect::get(&rect, &JsValue::from_str("width"))
                .unwrap()
                .as_f64()
                .unwrap();
            let height = Reflect::get(&rect, &JsValue::from_str("height"))
                .unwrap()
                .as_f64()
                .unwrap();

            if self.width != width || force_size_update_needed {
                self.width = width;
                self.canvas_width = self.css_px_to_cpx(self.width);
                self.canvas
                    .set_width((self.canvas_width as usize).try_into().unwrap());
            }
            if self.height != height || force_size_update_needed {
                self.height = height;
                self.canvas_height = self.css_px_to_cpx(self.height);
                self.canvas
                    .set_height((self.canvas_height as usize).try_into().unwrap());
            }
            self.canvas_size_sync_needed = false;
            true
        } else {
            false
        }
    }
    pub fn schedule_canvas_size_sync(&mut self) {
        self.canvas_size_sync_needed = true;
    }
    pub fn clear(&self) {
        self.context
            .clear_rect(0.0, 0.0, self.canvas_width, self.canvas_height);
    }
    pub fn contains_pos(&self, pos: &(f64, f64)) -> bool {
        self.x <= pos.0
            && self.y <= pos.1
            && self.x + self.width >= pos.0
            && self.y + self.height >= pos.1
    }
    #[inline]
    pub fn apx_to_cpx(&self, pixels: f64) -> f64 {
        self.device_pixel_ratio * pixels
    }
    #[inline]
    pub fn css_px_to_cpx(&self, pixels: f64) -> f64 {
        self.css_to_physical_scale * pixels
    }
    pub fn x_to_cx(&self, x: f64) -> f64 {
        (x - self.x) * self.css_to_physical_scale
    }
    pub fn y_to_cy(&self, y: f64) -> f64 {
        (y - self.y) * self.css_to_physical_scale
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
