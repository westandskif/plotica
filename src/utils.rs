/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::screen::ScreenPos;

pub fn is_click(pos1: &ScreenPos, pos2: &ScreenPos) -> bool {
    pos1.0 == pos2.0 && pos1.1 == pos2.1
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
