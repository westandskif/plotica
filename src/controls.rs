/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::screen::ScreenPos;
use js_sys::Reflect;
use wasm_bindgen::prelude::*;

pub enum ControlEvent {
    PointerDown { pos: ScreenPos },
    PointerMoved { pos: ScreenPos },
    PointerUp,
    PointerClicked,
    PointerLeft,
    PinchStarted { pos1: ScreenPos, pos2: ScreenPos },
    PinchUpdated { pos1: ScreenPos, pos2: ScreenPos },
    PinchFinished,
}
pub trait WatchControls {
    fn down(&mut self, event: &JsValue) -> Option<ControlEvent>;
    fn moved(&mut self, event: &JsValue) -> Option<ControlEvent>;
    fn up(&mut self, event: &JsValue) -> Option<ControlEvent>;
    fn left(&mut self, event: &JsValue) -> Option<ControlEvent>;
}

pub struct MouseControls {
    primary_down: Option<ScreenPos>,
    primary_moved: Option<ScreenPos>,
}

impl MouseControls {
    pub fn new() -> Self {
        Self {
            primary_down: None,
            primary_moved: None,
        }
    }
    fn get_event_coordinates(event: &JsValue) -> ScreenPos {
        let x = Reflect::get(&event, &JsValue::from_str("offsetX"))
            .unwrap()
            .as_f64()
            .unwrap();
        let y = Reflect::get(&event, &JsValue::from_str("offsetY"))
            .unwrap()
            .as_f64()
            .unwrap();
        ScreenPos(x, y)
    }
}
impl WatchControls for MouseControls {
    fn down(&mut self, event: &JsValue) -> Option<ControlEvent> {
        let pos = Self::get_event_coordinates(event);
        self.primary_down = Some(pos.clone());
        self.primary_moved = None;
        Some(ControlEvent::PointerDown { pos })
    }
    fn moved(&mut self, event: &JsValue) -> Option<ControlEvent> {
        let pos = Self::get_event_coordinates(event);
        self.primary_moved = Some(pos.clone());
        Some(ControlEvent::PointerMoved { pos })
    }
    fn up(&mut self, _event: &JsValue) -> Option<ControlEvent> {
        let result = if self.primary_moved.is_some() {
            Some(ControlEvent::PointerUp)
        } else {
            Some(ControlEvent::PointerClicked)
        };
        self.primary_down = None;
        self.primary_moved = None;
        result
    }
    fn left(&mut self, _event: &JsValue) -> Option<ControlEvent> {
        self.primary_down = None;
        self.primary_moved = None;
        Some(ControlEvent::PointerLeft)
    }
}

struct Touch {
    id: f64,
    pos: ScreenPos,
}

struct TouchState {
    id: f64,
    down: ScreenPos,
    moved: ScreenPos,
}

pub struct TouchControls {
    primary: Option<TouchState>,
    secondary: Option<TouchState>,
}
impl TouchControls {
    pub fn new() -> Self {
        Self {
            primary: None,
            secondary: None,
        }
    }
    fn get_updated_touches(event: &JsValue) -> Vec<Touch> {
        let canvas = Reflect::get(event, &JsValue::from_str("target")).unwrap();
        let rect = Reflect::get(&canvas, &JsValue::from_str("getBoundingClientRect"))
            .unwrap()
            .dyn_into::<js_sys::Function>()
            .unwrap()
            .call0(&canvas)
            .unwrap();
        let canvas_x = Reflect::get(&rect, &JsValue::from_str("x"))
            .unwrap()
            .as_f64()
            .unwrap();
        let canvas_y = Reflect::get(&rect, &JsValue::from_str("y"))
            .unwrap()
            .as_f64()
            .unwrap();

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
                    pos: ScreenPos(x - canvas_x, y - canvas_y),
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
                    down: touch.pos.clone(),
                    moved: touch.pos.clone(),
                });
            }
        }
        if self.secondary.is_none() {
            if let Some(touch) = touches_iter.next() {
                self.secondary = Some(TouchState {
                    id: touch.id,
                    down: touch.pos.clone(),
                    moved: touch.pos.clone(),
                });
                secondary_set = true;
            }
        }
        if secondary_set {
            Some(ControlEvent::PinchStarted {
                pos1: self.primary.as_ref().unwrap().down.clone(),
                pos2: self.secondary.as_ref().unwrap().down.clone(),
            })
        } else {
            Some(ControlEvent::PointerDown {
                pos: self.primary.as_ref().unwrap().down.clone(),
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
                primary.moved = touch.pos.clone();
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
                secondary.moved = touch.pos.clone();
                updated_touches += 1;
            }
        }
        if active_touches == 2 && updated_touches > 0 {
            Some(ControlEvent::PinchUpdated {
                pos1: self.primary.as_ref().unwrap().moved.clone(),
                pos2: self.secondary.as_ref().unwrap().moved.clone(),
            })
        } else if primary_updated {
            Some(ControlEvent::PointerMoved {
                pos: self.primary.as_ref().unwrap().moved.clone(),
            })
        } else {
            None
        }
    }
    fn up(&mut self, event: &JsValue) -> Option<ControlEvent> {
        let touches = Self::get_updated_touches(event);
        let mut secondary_just_left = false;
        if let Some(secondary) = self.secondary.as_ref() {
            if let Some(_) = touches
                .iter()
                .filter(|touch| touch.id == secondary.id)
                .next()
            {
                self.secondary = None;
                secondary_just_left = true;
            }
        }
        let mut result: Option<ControlEvent> = None;
        let mut primary_to_be_dropped = false;
        if let Some(primary) = self.primary.as_ref() {
            if let Some(_) = touches.iter().filter(|touch| touch.id == primary.id).next() {
                primary_to_be_dropped = true;
                result = if secondary_just_left {
                    Some(ControlEvent::PinchFinished)
                } else if self.secondary.is_none() {
                    if primary.moved == primary.down {
                        Some(ControlEvent::PointerClicked)
                    } else {
                        Some(ControlEvent::PointerUp)
                    }
                } else {
                    None
                };
            }
        } else if secondary_just_left {
            result = Some(ControlEvent::PinchFinished);
        }
        if primary_to_be_dropped {
            self.primary = None;
        }
        result
    }
    fn left(&mut self, _: &JsValue) -> Option<ControlEvent> {
        self.primary = None;
        self.secondary = None;
        Some(ControlEvent::PointerLeft)
    }
}
