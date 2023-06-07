/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use wasm_bindgen::prelude::*;
// https://doc.rust-lang.org/reference/macros-by-example.html#scoping-exporting-and-importing

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn console_log(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn console_log_js_value(v: JsValue);
}

#[allow(unused_macros)]
macro_rules! expr_to_debug_literal {
    ($i:literal) => {
        "{}"
    };
    ($i:expr) => {
        concat!(stringify!($i), " => {:?}; ")
    };
}

#[allow(unused_macros)]
macro_rules! console_debug {
    ($($i:expr),*) => {
        crate::debug::console_log(
            format!(
                concat!($(
                    expr_to_debug_literal!($i)
                ), *),
                $( &$i ), *
            ).as_str()
        );
    };
}
