/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use js_sys::Reflect;
use wasm_bindgen::prelude::*;

pub fn is_click(pos1: &Option<(f64, f64)>, pos2: &Option<(f64, f64)>) -> bool {
    match (pos1, pos2) {
        (Some((x1, y1)), Some((x2, y2))) => {
            *x1 == *x2 && *y1 == *y2
            // let delta_x = (*x1 - *x2).abs();
            // let delta_y = (*y1 - *y2).abs();
            // (delta_x * delta_x + delta_y * delta_y).sqrt() <= 3.0
        }
        _ => false,
    }
}
pub fn js_scroll_coords() -> (f64, f64) {
    let window = web_sys::window().unwrap();
    let scroll_x = Reflect::get(&window, &JsValue::from_str("scrollX"))
        .unwrap()
        .as_f64()
        .unwrap();
    let scroll_y = Reflect::get(&window, &JsValue::from_str("scrollY"))
        .unwrap()
        .as_f64()
        .unwrap();
    (scroll_x, scroll_y)
}
pub fn js_coords_to_global(x: f64, y: f64) -> (f64, f64) {
    let (scroll_x, scroll_y) = js_scroll_coords();
    (scroll_x + x, scroll_y + y)
}
pub fn place_rect_inside(
    desired_x: f64,
    desired_y: f64,
    width: f64,
    height: f64,
    x_min: f64,
    x_max: f64,
    y_max: f64,
    x_shift: f64,
) -> (f64, f64) {
    let x: f64;
    if desired_x + width > x_max {
        x = (desired_x - x_shift - width).max(x_min);
    } else {
        x = (desired_x + x_shift).min(x_max - width);
    };
    let y = desired_y.min(y_max - height);
    (x, y)
}
