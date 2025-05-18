/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use std::sync::atomic::{AtomicUsize, Ordering};

pub static ANIMATED_NUMBERS_COUNT: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone)]
pub struct AnimatedNumber {
    x0: f64,
    x1: f64,
    k: f64,
    c: f64,
    v0: f64,
    t0: Option<f64>,
    dt1: f64,
    dt2: f64,
}
impl AnimatedNumber {
    pub fn new(initial_value: f64) -> Self {
        Self::custom(initial_value, 100000.0, 100000.0)
    }
    pub fn custom(initial_value: f64, dt1_us: f64, dt2_us: f64) -> Self {
        Self {
            x0: initial_value,
            x1: initial_value,
            k: 0.0,
            c: 0.0,
            v0: 0.0,
            t0: None,
            // dt1: 300000.0,
            // dt2: 200000.0,
            dt1: dt1_us,
            dt2: dt2_us,
        }
    }
    pub fn get_value(&mut self, time_us: f64) -> f64 {
        match self.t0 {
            None => self.x1,
            Some(t0) => {
                ANIMATED_NUMBERS_COUNT.fetch_add(1, Ordering::Relaxed);
                let us = time_us - t0;
                if us <= self.dt1 {
                    (self.k * us * us / 2.0 + self.v0 * us) * (self.x1 - self.x0) + self.x0
                } else if us >= self.dt2 + self.dt1 {
                    self.t0 = None;
                    self.x1
                } else {
                    (self.k * self.dt1 * self.dt1 / 2.0
                        + self.v0 * self.dt1
                        + self.c * self.dt2.min(us - self.dt1))
                        * (self.x1 - self.x0)
                        + self.x0
                }
            }
        }
    }
    pub fn get_end_value(&self) -> f64 {
        self.x1
    }
    pub fn set_value(&mut self, new_value: f64, time_us: Option<f64>) {
        match time_us {
            None => {
                self.t0 = None;
                self.x1 = new_value;
            }
            Some(time_us) => {
                ANIMATED_NUMBERS_COUNT.fetch_add(1, Ordering::Relaxed);
                self.x0 = self.get_value(time_us);
                self.x1 = new_value;
                self.v0 = match self.t0 {
                    Some(t0) => self.v0 + self.k * (self.dt1.min(time_us - t0)),
                    None => 0.0,
                };

                self.k = (1.0 - self.v0 * (self.dt1 + self.dt2))
                    / (self.dt1 * (self.dt1 * 0.5 + self.dt2));

                self.c = self.k * self.dt1 + self.v0;
                self.t0 = Some(time_us);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::animate::AnimatedNumber;

    #[test]
    fn test_animated_number() {
        assert_eq!(AnimatedNumber::new(1.0).get_end_value(), 1.0);
        assert_eq!(AnimatedNumber::new(1.0).get_value(100.0), 1.0);

        let mut n = AnimatedNumber::new(0.0);
        assert_eq!(n.get_value(150.0), 0.0);
        n.set_value(1.0, Some(150.0));
        assert_eq!(n.get_value(150.0), 0.0);

        let mut n = AnimatedNumber::new(1.0);
        n.set_value(0.0, Some(1000000.0));
        assert_eq!(n.get_end_value(), 0.0);
        assert_eq!(n.get_value(1000000.0), 1.0);
        assert_eq!(n.get_value(1200000.0), 0.9215686274509804);
        assert_eq!(n.get_value(1300000.0), 0.8235294117647058);
        assert_eq!(n.get_value(1500000.0), 0.5882352941176471);
        assert_eq!(n.get_value(1700000.0), 0.3529411764705883);
        assert_eq!(n.get_value(1900000.0), 0.11764705882352944);
        assert_eq!(n.get_value(2000000.0), 0.0);
    }
}
