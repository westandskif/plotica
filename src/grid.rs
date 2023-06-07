/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::animate::AnimatedNumber;
use crate::params::DataType;
use std::cmp::Ordering;

#[derive(Debug)]
pub struct Tick {
    pub normalized_value: f64,
    pub alpha: f64,
    pub end_alpha: f64,
    pub value: f64,
}

#[derive(Clone)]
pub struct TickGeneration {
    period: f64,
    alpha: AnimatedNumber,
}
#[derive(Clone)]
pub struct Grid {
    pub grid_base: f64,
    pub grid_period: f64,
    pub min_period: Option<f64>,
    pub current_period: f64,
    pub generations: Vec<TickGeneration>,
}
impl Grid {
    pub fn new(data_type: DataType, global_min: f64, global_max: f64) -> Self {
        let min_period = match data_type.get_min_period() {
            Some(min_period) => Some(min_period / (global_max - global_min)),
            None => None,
        };
        let mut grid_period: f64 = 1.0;
        let mut grid_base: f64;

        match min_period {
            Some(min_period) => {
                if grid_period < min_period {
                    grid_period = min_period;
                } else {
                    grid_period -= grid_period % min_period;
                }
                grid_base = grid_period * -0.5;
                grid_base -= grid_base % min_period;
            }
            None => {
                grid_base = grid_period * -0.5;
            }
        }

        Self {
            grid_base,
            grid_period,
            min_period,
            current_period: grid_period,
            generations: vec![TickGeneration {
                period: grid_period,
                alpha: AnimatedNumber::new(1.0),
            }],
        }
    }

    pub fn get_ticks(
        &mut self,
        time_us: f64,
        normalized_min_value: f64,
        normalized_max_value: f64,
        max_ticks: f64,
    ) -> Vec<Tick> {
        let range = normalized_max_value - normalized_min_value;
        let mut period = self.grid_period
            * f64::powi(
                2.0,
                (range / self.grid_period / max_ticks).log2().round() as i32,
            );
        if let Some(min_period) = self.min_period {
            if min_period > period {
                period = min_period;
            }
        }

        if self.current_period != period {
            let mut generation_to_be_created = true;
            let current_ticks_number = range / period;
            for generation in self.generations.iter_mut() {
                if generation.period == period {
                    generation_to_be_created = false;
                    generation.alpha.set_value(1.0, Some(time_us));
                } else {
                    let this_ticks_number = range / generation.period;
                    if this_ticks_number > current_ticks_number * 4.0
                        || this_ticks_number < current_ticks_number as f64 * 0.25
                    {
                        generation.alpha.set_value(0.0, None);
                    } else {
                        generation.alpha.set_value(0.0, Some(time_us));
                    }
                }
            }
            if generation_to_be_created {
                let mut alpha = AnimatedNumber::new(0.4);
                alpha.set_value(1.0, Some(time_us));
                self.generations.push(TickGeneration { period, alpha });
            }
            self.current_period = period;
        } else if self.generations.len() > 1 {
            self.generations
                .retain_mut(|generation| generation.alpha.get_value(time_us) > 0.0);
        }

        let mut ticks: Vec<Tick> = Vec::new();
        for generation in self.generations.iter_mut() {
            let alpha = generation.alpha.get_value(time_us);
            let end_alpha = generation.alpha.get_end_value();
            let period = generation.period;
            let mut normalized_value =
                normalized_min_value - (normalized_min_value - self.grid_base) % period + period;

            let right_bound = normalized_max_value - period * 0.25;
            let left_bound = normalized_min_value + period * 0.25;

            while normalized_value < normalized_max_value {
                ticks.push(Tick {
                    normalized_value,
                    value: 0.0,
                    alpha: if normalized_value < left_bound || normalized_value > right_bound {
                        alpha * 0.5
                    } else {
                        alpha
                    },
                    end_alpha,
                });
                normalized_value += period;
            }
        }
        if self.generations.len() > 1 {
            ticks.sort_unstable_by(|a, b| {
                match a.normalized_value.partial_cmp(&b.normalized_value).unwrap() {
                    Ordering::Equal => b.alpha.partial_cmp(&a.alpha).unwrap(),
                    value => value,
                }
            });
            ticks.dedup_by(|a, b| {
                if a.normalized_value == b.normalized_value {
                    if a.end_alpha == 1.0 || b.end_alpha == 1.0 {
                        b.alpha = 1.0;
                    }
                    true
                } else {
                    false
                }
            });
        }
        ticks
    }
}
