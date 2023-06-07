/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::utils::js_scroll_coords;
use js_sys::Reflect;
use wasm_bindgen::prelude::*;

pub enum ControlEvent {
    PointerDown { pos: (f64, f64) },
    PointerMoved { pos: (f64, f64) },
    PointerUp,
    PinchStarted { pos1: (f64, f64), pos2: (f64, f64) },
    PinchUpdated { pos1: (f64, f64), pos2: (f64, f64) },
    PinchFinished,
}
pub trait WatchControls {
    fn down(&mut self, event: &JsValue) -> Option<ControlEvent>;
    fn moved(&mut self, event: &JsValue) -> Option<ControlEvent>;
    fn up(&mut self, event: &JsValue) -> Option<ControlEvent>;
    fn left(&mut self, event: &JsValue) -> Option<ControlEvent>;
}

pub struct MouseControls {
    primary_down: Option<(f64, f64)>,
    primary_moved: Option<(f64, f64)>,
}

impl MouseControls {
    pub fn new() -> Self {
        Self {
            primary_down: None,
            primary_moved: None,
        }
    }
    fn get_event_coordinates(event: &JsValue) -> (f64, f64) {
        let (scroll_x, scroll_y) = js_scroll_coords();
        let x = Reflect::get(&event, &JsValue::from_str("clientX"))
            .unwrap()
            .as_f64()
            .unwrap();
        let y = Reflect::get(&event, &JsValue::from_str("clientY"))
            .unwrap()
            .as_f64()
            .unwrap();
        (x + scroll_x, y + scroll_y)
    }
}
impl WatchControls for MouseControls {
    fn down(&mut self, event: &JsValue) -> Option<ControlEvent> {
        let pos = Self::get_event_coordinates(event);
        self.primary_down = Some(pos);
        Some(ControlEvent::PointerDown { pos })
    }
    fn moved(&mut self, event: &JsValue) -> Option<ControlEvent> {
        let pos = Self::get_event_coordinates(event);
        self.primary_moved = Some(pos);
        Some(ControlEvent::PointerMoved { pos })
    }
    fn up(&mut self, _event: &JsValue) -> Option<ControlEvent> {
        self.primary_down = None;
        self.primary_moved = None;
        Some(ControlEvent::PointerUp)
    }
    fn left(&mut self, _event: &JsValue) -> Option<ControlEvent> {
        self.primary_down = None;
        self.primary_moved = None;
        None
    }
}

struct Touch {
    id: f64,
    pos: (f64, f64),
}

struct TouchState {
    id: f64,
    down: (f64, f64),
    moved: (f64, f64),
}

pub struct TouchControls {
    primary: Option<TouchState>,
    secondary: Option<TouchState>,
    secondary_just_left: bool,
}
impl TouchControls {
    pub fn new() -> Self {
        Self {
            primary: None,
            secondary: None,
            secondary_just_left: false,
        }
    }
    fn get_updated_touches(event: &JsValue) -> Vec<Touch> {
        let (scroll_x, scroll_y) = js_scroll_coords();
        let changed_touches = Reflect::get(&event, &JsValue::from_str("changedTouches")).unwrap();
        let number_of_touches = Reflect::get(&changed_touches, &JsValue::from_str("length"))
            .unwrap()
            .as_f64()
            .unwrap() as u32;
        (0..number_of_touches)
            .into_iter()
            .map(|index| {
                let js_touch = Reflect::get_u32(&changed_touches, index).unwrap();
                let x = Reflect::get(&js_touch, &JsValue::from_str("clientX"))
                    .unwrap()
                    .as_f64()
                    .unwrap();
                let y = Reflect::get(&js_touch, &JsValue::from_str("clientY"))
                    .unwrap()
                    .as_f64()
                    .unwrap();
                let id = Reflect::get(&js_touch, &JsValue::from_str("identifier"))
                    .unwrap()
                    .as_f64()
                    .unwrap();
                Touch {
                    id,
                    pos: (scroll_x + x, scroll_y + y),
                }
            })
            .collect()
    }
}
impl WatchControls for TouchControls {
    fn down(&mut self, event: &JsValue) -> Option<ControlEvent> {
        let touches = Self::get_updated_touches(event);
        let mut touches_iter = touches.iter();
        let mut secondary_set = false;
        if self.primary.is_none() {
            if let Some(touch) = touches_iter.next() {
                self.primary = Some(TouchState {
                    id: touch.id,
                    down: touch.pos,
                    moved: touch.pos,
                });
            }
        }
        if self.secondary.is_none() {
            if let Some(touch) = touches_iter.next() {
                self.secondary = Some(TouchState {
                    id: touch.id,
                    down: touch.pos,
                    moved: touch.pos,
                });
                self.secondary_just_left = false;
                secondary_set = true;
            }
        }
        if secondary_set {
            Some(ControlEvent::PinchStarted {
                pos1: self.primary.as_ref().unwrap().down,
                pos2: self.secondary.as_ref().unwrap().down,
            })
        } else {
            Some(ControlEvent::PointerDown {
                pos: self.primary.as_ref().unwrap().down,
            })
        }
    }
    fn moved(&mut self, event: &JsValue) -> Option<ControlEvent> {
        let touches = Self::get_updated_touches(event);
        let mut primary_updated = false;
        let mut updated_touches = 0;
        let mut active_touches = 0;
        if let Some(primary) = self.primary.as_mut() {
            active_touches += 1;
            if let Some(touch) = touches.iter().filter(|touch| touch.id == primary.id).next() {
                primary.moved = touch.pos;
                primary_updated = true;
                updated_touches += 1;
            }
        }
        if let Some(secondary) = self.secondary.as_mut() {
            active_touches += 1;
            if let Some(touch) = touches
                .iter()
                .filter(|touch| touch.id == secondary.id)
                .next()
            {
                secondary.moved = touch.pos;
                updated_touches += 1;
            }
        }
        if active_touches == 2 && updated_touches > 0 {
            Some(ControlEvent::PinchUpdated {
                pos1: self.primary.as_ref().unwrap().moved,
                pos2: self.secondary.as_ref().unwrap().moved,
            })
        } else if primary_updated {
            Some(ControlEvent::PointerMoved {
                pos: self.primary.as_ref().unwrap().moved,
            })
        } else {
            None
        }
    }
    fn up(&mut self, event: &JsValue) -> Option<ControlEvent> {
        let touches = Self::get_updated_touches(event);
        if let Some(secondary) = self.secondary.as_ref() {
            if let Some(_) = touches
                .iter()
                .filter(|touch| touch.id == secondary.id)
                .next()
            {
                self.secondary = None;
                self.secondary_just_left = true;
            }
        }
        if let Some(primary) = self.primary.as_ref() {
            if let Some(_) = touches.iter().filter(|touch| touch.id == primary.id).next() {
                self.primary = None;
                return if self.secondary_just_left {
                    self.secondary_just_left = false;
                    Some(ControlEvent::PinchFinished)
                } else if self.secondary.is_none() {
                    Some(ControlEvent::PointerUp)
                } else {
                    None
                };
            }
        } else if self.secondary_just_left {
            self.secondary_just_left = false;
            return Some(ControlEvent::PinchFinished);
        }
        None
    }
    fn left(&mut self, _: &JsValue) -> Option<ControlEvent> {
        self.primary = None;
        self.secondary = None;
        None
    }
}
