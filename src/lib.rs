/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
#[macro_use]
mod debug;
mod animate;
mod camera;
mod controls;
mod data_set;
mod events;
mod grid;
mod legend;
mod main_chart;
mod manager;
mod params;
mod scale;
mod screen;
mod tooltip;
mod utils;
use crate::manager::{get_or_create_manager_addr, ChartManager};
use std::pin::Pin;

use wasm_bindgen::prelude::*;

fn get_pinned_manager() -> Pin<Box<ChartManager>> {
    Box::into_pin(unsafe { Box::from_raw(get_or_create_manager_addr() as *mut ChartManager) })
}
fn destruct_pinned_manager(manager: Pin<Box<ChartManager>>) {
    Box::into_raw(unsafe { Pin::into_inner_unchecked(manager) });
}

#[wasm_bindgen(js_name = createMain)]
pub fn create_main(raw_params: JsValue, raw_config: JsValue) -> Result<String, String> {
    let mut pinned_manager = get_pinned_manager();
    let result = pinned_manager.as_mut().create_main(raw_params, raw_config);
    destruct_pinned_manager(pinned_manager);
    result
}

#[wasm_bindgen(js_name = destroyMain)]
pub fn destroy_main(chart_id: JsValue) -> Result<(), String> {
    let mut pinned_manager = get_pinned_manager();
    let result = pinned_manager.as_mut().destroy_main(chart_id);
    destruct_pinned_manager(pinned_manager);
    result
}
