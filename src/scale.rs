/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::params::Content;

pub trait Scale: Clone {
    fn reframe(&mut self, coord_min: f64, coord_max: f64, value_min: f64, value_max: f64);
    fn get_coord_min(&self) -> f64;
    fn get_coord_max(&self) -> f64;
    fn get_value_min(&self) -> f64;
    fn get_value_max(&self) -> f64;
    fn normalize_coord(&self, coord: f64) -> f64;
    fn normalize_value(&self, value: f64) -> f64;
    fn denormalize_coord(&self, normalized_coord: f64) -> f64;
    fn denormalize_value(&self, normalized_value: f64) -> f64;
}

#[derive(Clone)]
pub struct LinearScale {
    pub coord_min: f64,
    pub coord_max: f64,
    pub coord_range: f64,
    pub coord_range_recip: f64,
    pub value_min: f64,
    pub value_max: f64,
    pub value_range: f64,
    pub value_range_recip: f64,
}
impl LinearScale {
    pub fn new(content: &Content) -> Self {
        let global_coord_min = content.global_coord_min;
        let global_coord_max = content.global_coord_max;
        let global_value_min = content.global_value_min;
        let global_value_max = content.global_value_max;
        let coord_range = global_coord_max - global_coord_min;
        let value_range = global_value_max - global_value_min;
        Self {
            coord_min: global_coord_min,
            coord_max: global_coord_max,
            coord_range,
            coord_range_recip: coord_range.recip(),
            value_min: global_value_min,
            value_max: global_value_max,
            value_range,
            value_range_recip: value_range.recip(),
        }
    }
}

impl Scale for LinearScale {
    fn reframe(&mut self, coord_min: f64, coord_max: f64, value_min: f64, value_max: f64) {
        let coord_range = coord_max - coord_min;
        if coord_range == 0.0 {
            panic!("coord range cannot be zero")
        }
        let value_range = value_max - value_min;
        if value_range == 0.0 {
            panic!("value range cannot be zero")
        }
        self.coord_max = coord_max;
        self.coord_min = coord_min;
        self.coord_range = coord_range;
        self.coord_range_recip = coord_range.recip();
        self.value_max = value_max;
        self.value_min = value_min;
        self.value_range = value_range;
        self.value_range_recip = value_range.recip();
    }
    #[inline]
    fn normalize_coord(&self, coord: f64) -> f64 {
        (coord - self.coord_min) * self.coord_range_recip
    }
    #[inline]
    fn normalize_value(&self, value: f64) -> f64 {
        (value - self.value_min) * self.value_range_recip
    }
    #[inline]
    fn denormalize_coord(&self, normalized_coord: f64) -> f64 {
        normalized_coord * self.coord_range + self.coord_min
    }
    #[inline]
    fn denormalize_value(&self, normalized_value: f64) -> f64 {
        normalized_value * self.value_range + self.value_min
    }
    #[inline]
    fn get_coord_min(&self) -> f64 {
        self.coord_min
    }
    #[inline]
    fn get_coord_max(&self) -> f64 {
        self.coord_max
    }
    #[inline]
    fn get_value_min(&self) -> f64 {
        self.value_min
    }
    #[inline]
    fn get_value_max(&self) -> f64 {
        self.value_max
    }
}

const MIN_VALUE_TO_LOG: f64 = 1000.0;
const MIN_LOG_VALUE: f64 = 3.0;

#[derive(Clone)]
pub struct LogScale {
    pub coord_min: f64,
    pub coord_max: f64,
    pub coord_range: f64,
    pub coord_range_recip: f64,
    pub value_min: f64,
    pub value_max: f64,
    pub value_global_min: f64,
    pub value_log_base: f64,
    pub value_log_range: f64,
    pub value_log_range_recip: f64,
}
impl LogScale {
    pub fn new(content: &Content) -> Self {
        let global_coord_min = content.global_coord_min;
        let global_coord_max = content.global_coord_max;
        let global_value_min = content.global_value_min;
        let global_value_max = content.global_value_max;
        let coord_range = global_coord_max - global_coord_min;

        let value_min_log = MIN_LOG_VALUE;
        let value_max_log = (global_value_max - global_value_min + MIN_VALUE_TO_LOG).log10();
        Self {
            coord_min: global_coord_min,
            coord_max: global_coord_max,
            coord_range,
            coord_range_recip: coord_range.recip(),
            value_min: global_value_min,
            value_max: global_value_max,
            value_global_min: global_value_min,
            value_log_base: value_min_log,
            value_log_range: value_max_log - value_min_log,
            value_log_range_recip: (value_max_log - value_min_log).recip(),
        }
    }
}

impl Scale for LogScale {
    fn reframe(&mut self, coord_min: f64, coord_max: f64, value_min: f64, value_max: f64) {
        let coord_range = coord_max - coord_min;
        if coord_range == 0.0 {
            panic!("coord range cannot be zero")
        }

        let value_range = value_max - value_min;
        if value_range == 0.0 {
            panic!("value range cannot be zero")
        }
        let value_min_log = (self.value_min - self.value_global_min + MIN_VALUE_TO_LOG).log10();
        let value_max_log = (value_max - self.value_global_min + MIN_VALUE_TO_LOG).log10();
        self.coord_max = coord_max;
        self.coord_min = coord_min;
        self.coord_range = coord_range;
        self.coord_range_recip = coord_range.recip();
        self.value_max = value_max;
        self.value_min = value_min;
        self.value_log_base = value_min_log;
        self.value_log_range = value_max_log - value_min_log;
        self.value_log_range_recip = (value_max_log - value_min_log).recip();
    }
    #[inline]
    fn normalize_coord(&self, coord: f64) -> f64 {
        (coord - self.coord_min) * self.coord_range_recip
    }
    #[inline]
    fn normalize_value(&self, value: f64) -> f64 {
        ((value - self.value_global_min + MIN_VALUE_TO_LOG).log10() - self.value_log_base)
            * self.value_log_range_recip
    }
    #[inline]
    fn denormalize_coord(&self, normalized_coord: f64) -> f64 {
        normalized_coord * self.coord_range + self.coord_min
    }
    #[inline]
    fn denormalize_value(&self, normalized_value: f64) -> f64 {
        10.0_f64.powf(normalized_value * self.value_log_range + self.value_log_base)
            - MIN_VALUE_TO_LOG
            + self.value_global_min
    }
    #[inline]
    fn get_coord_min(&self) -> f64 {
        self.coord_min
    }
    #[inline]
    fn get_coord_max(&self) -> f64 {
        self.coord_max
    }
    #[inline]
    fn get_value_min(&self) -> f64 {
        self.value_min
    }
    #[inline]
    fn get_value_max(&self) -> f64 {
        self.value_max
    }
}
