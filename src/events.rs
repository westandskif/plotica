/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::EventTarget;

pub struct JsEventListener {
    event_target: EventTarget,
    event_name: String,
    closure: Closure<dyn Fn(JsValue)>,
}
impl JsEventListener {
    pub fn new(
        event_target: EventTarget,
        event_name: &str,
        listener: Box<dyn Fn(JsValue)>,
    ) -> Self {
        let closure = Closure::new(listener);
        event_target
            .add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())
            .unwrap();
        Self {
            event_target,
            event_name: event_name.to_string(),
            closure,
        }
    }
}
impl Drop for JsEventListener {
    fn drop(&mut self) {
        self.event_target
            .remove_event_listener_with_callback(
                self.event_name.as_str(),
                self.closure.as_ref().unchecked_ref(),
            )
            .unwrap();
    }
}
