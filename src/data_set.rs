/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Copyright (C) 2023, Nikita Almakov
 */
use crate::animate::AnimatedNumber;

#[derive(Debug, PartialEq)]
pub struct DataPoint {
    pub coord: f64,
    pub value: f64,
}
#[derive(Debug)]
pub struct DataSetMeta {
    pub min: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub max: f64,
}
impl DataSetMeta {
    pub fn from_data_points(data_points: &[DataPoint]) -> Self {
        let mut total: f64 = 0.0;
        let mut values: Vec<f64> = data_points
            .iter()
            .map(|p| {
                total += p.value;
                p.value
            })
            .collect();
        values.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let max_index = values.len() - 1;
        Self {
            min: *values.get(0).unwrap(),
            p25: DataSetMeta::percentile(values.as_slice(), 0.25, max_index),
            p50: DataSetMeta::percentile(values.as_slice(), 0.5, max_index),
            p75: DataSetMeta::percentile(values.as_slice(), 0.75, max_index),
            max: *values.get(max_index).unwrap(),
        }
    }
    fn percentile(values: &[f64], percentile: f64, max_index: usize) -> f64 {
        let index = max_index as f64 * percentile;
        let left_index = index as usize;
        let left_value = *values.get(left_index).unwrap();
        if left_index == max_index {
            left_value
        } else {
            left_value
                + (*values.get(left_index + 1).unwrap() - left_value) * (index - left_index as f64)
        }
    }
}

pub struct DataSet {
    pub name: String,
    pub data_points: Vec<DataPoint>,
    pub meta: DataSetMeta,
    pub rgb: (u8, u8, u8),
    pub alpha: AnimatedNumber,
}

impl DataSet {
    pub fn new(name: &str, rgb: (u8, u8, u8), data_points: Vec<DataPoint>) -> Self {
        let meta = DataSetMeta::from_data_points(data_points.as_slice());
        Self {
            name: name.to_string(),
            data_points,
            meta,
            rgb,
            alpha: AnimatedNumber::new(1.0),
        }
    }
    pub fn slice_by_coord(&self, coord_start: f64, coord_end: f64) -> Option<&[DataPoint]> {
        if let Some(left_idx) = self.bin_search_left_bound(coord_start) {
            if let Some(right_idx) = self.bin_search_right_bound(coord_end) {
                return Some(&self.data_points[left_idx..right_idx + 1]);
            }
        }
        None
    }
    pub fn to_css_color(&self, alpha: f64) -> String {
        format!(
            "rgba({}, {}, {}, {})",
            self.rgb.0, self.rgb.1, self.rgb.2, alpha
        )
    }
    pub fn bin_search_left_bound(&self, left_bound: f64) -> Option<usize> {
        let data = self.data_points.as_slice();
        if data.is_empty() {
            return None;
        }
        let mut left_idx: usize = 0;
        let mut right_idx: usize = data.len() - 1;
        let max_idx = data.len() - 1;

        while left_idx <= right_idx {
            let middle_idx = (left_idx + right_idx) / 2;
            let current = &data.get(middle_idx).unwrap();

            if left_bound <= current.coord
                && (middle_idx == 0 || left_bound > data.get(middle_idx - 1).unwrap().coord)
            {
                return Some(middle_idx);
            } else {
                if left_bound <= current.coord {
                    if middle_idx == 0 {
                        return Some(0);
                    }
                    right_idx = middle_idx - 1;
                } else {
                    if middle_idx == max_idx {
                        return None;
                    }
                    left_idx = middle_idx + 1;
                }
            }
        }
        None
    }
    pub fn bin_search_right_bound(&self, right_bound: f64) -> Option<usize> {
        let data = self.data_points.as_slice();
        if data.is_empty() {
            return None;
        }
        let mut left_idx: usize = 0;
        let mut right_idx: usize = data.len() - 1;
        let max_idx = data.len() - 1;

        while left_idx <= right_idx {
            let middle_idx = (left_idx + right_idx) / 2;
            let current = &data.get(middle_idx).unwrap();

            if right_bound >= current.coord
                && (middle_idx == max_idx || right_bound < data.get(middle_idx + 1).unwrap().coord)
            {
                return Some(middle_idx);
            } else {
                if right_bound >= current.coord {
                    if middle_idx == max_idx {
                        return Some(max_idx);
                    }
                    left_idx = middle_idx + 1;
                } else {
                    if middle_idx == 0 {
                        return None;
                    }
                    right_idx = middle_idx - 1;
                }
            }
        }
        None
    }
    #[allow(dead_code)]
    pub fn bin_search(&self, x: f64) -> Option<usize> {
        let data = self.data_points.as_slice();
        if data.is_empty() {
            return None;
        }
        let mut left_idx: usize = 0;
        let mut right_idx: usize = data.len() - 1;
        let max_idx = data.len() - 1;

        while left_idx <= right_idx {
            let middle_idx = (left_idx + right_idx) / 2;
            let current = &data.get(middle_idx).unwrap();

            if x < current.coord {
                if middle_idx == 0 {
                    return None;
                }
                right_idx = middle_idx - 1;
            } else if x > current.coord {
                if middle_idx == max_idx {
                    return None;
                }
                left_idx = middle_idx + 1;
            } else {
                return Some(middle_idx);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::data_set::{DataPoint, DataSet};
    #[test]
    fn test_bin_search_empty() {
        let empty_data = DataSet::new("test", (255, 255, 255), vec![]);
        assert_eq!(empty_data.bin_search_left_bound(1.0), None);
        assert_eq!(empty_data.bin_search_right_bound(1.0), None);
        assert_eq!(empty_data.bin_search(1.0), None);
        match empty_data.slice_by_coord(1.0, 2.0) {
            Some(_) => assert!(false, "should be empty"),
            None => assert!(true),
        }
    }
    #[test]
    fn test_bin_search_odd_number() {
        let data = DataSet::new(
            "test",
            (255, 255, 255),
            vec![
                DataPoint {
                    coord: 1.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 2.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 2.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 4.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 5.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 6.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 7.0,
                    value: 0.0,
                },
            ],
        );
        assert_eq!(data.bin_search_left_bound(0.0), Some(0));
        assert_eq!(data.bin_search_left_bound(0.5), Some(0));
        assert_eq!(data.bin_search_left_bound(1.0), Some(0));
        assert_eq!(data.bin_search_left_bound(2.0), Some(1));
        assert_eq!(data.bin_search_left_bound(7.0), Some(6));
        assert_eq!(data.bin_search_left_bound(8.0), None);

        assert_eq!(data.bin_search_right_bound(0.0), None);
        assert_eq!(data.bin_search_right_bound(0.5), None);
        assert_eq!(data.bin_search_right_bound(1.0), Some(0));
        assert_eq!(data.bin_search_right_bound(2.0), Some(2));
        assert_eq!(data.bin_search_right_bound(7.0), Some(6));
        assert_eq!(data.bin_search_right_bound(8.0), Some(6));

        assert_eq!(data.bin_search(0.0), None);
        assert_eq!(data.bin_search(0.5), None);
        assert_eq!(data.bin_search(1.0), Some(0));
        match data.bin_search(2.0) {
            Some(x) => assert!(x >= 1 && x <= 2),
            None => assert!(false),
        }
        assert_eq!(data.bin_search(7.0), Some(6));
        assert_eq!(data.bin_search(8.0), None);
    }

    #[test]
    fn test_bin_search_even() {
        let data = DataSet::new(
            "test",
            (255, 255, 255),
            vec![
                DataPoint {
                    coord: 1.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 2.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 2.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 2.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 4.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 5.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 6.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 7.0,
                    value: 0.0,
                },
            ],
        );
        assert_eq!(data.bin_search_left_bound(0.0), Some(0));
        assert_eq!(data.bin_search_left_bound(0.5), Some(0));
        assert_eq!(data.bin_search_left_bound(1.0), Some(0));
        assert_eq!(data.bin_search_left_bound(2.0), Some(1));
        assert_eq!(data.bin_search_left_bound(7.0), Some(7));
        assert_eq!(data.bin_search_left_bound(8.0), None);

        assert_eq!(data.bin_search_right_bound(0.0), None);
        assert_eq!(data.bin_search_right_bound(0.5), None);
        assert_eq!(data.bin_search_right_bound(1.0), Some(0));
        assert_eq!(data.bin_search_right_bound(2.0), Some(3));
        assert_eq!(data.bin_search_right_bound(7.0), Some(7));
        assert_eq!(data.bin_search_right_bound(8.0), Some(7));

        assert_eq!(data.bin_search(0.0), None);
        assert_eq!(data.bin_search(0.5), None);
        assert_eq!(data.bin_search(1.0), Some(0));
        match data.bin_search(2.0) {
            Some(x) => assert!(x >= 1 && x <= 3),
            None => assert!(false),
        }
        assert_eq!(data.bin_search(7.0), Some(7));
        assert_eq!(data.bin_search(8.0), None);
    }

    #[test]
    fn test_slice_by_coord() {
        let data = DataSet::new(
            "test",
            (255, 255, 255),
            vec![
                DataPoint {
                    coord: 1.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 2.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 2.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 4.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 4.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 5.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 6.0,
                    value: 0.0,
                },
                DataPoint {
                    coord: 7.0,
                    value: 0.0,
                },
            ],
        );
        assert_eq!(
            data.slice_by_coord(1.5, 4.2),
            Some(
                vec![
                    DataPoint {
                        coord: 2.0,
                        value: 0.0,
                    },
                    DataPoint {
                        coord: 2.0,
                        value: 0.0,
                    },
                    DataPoint {
                        coord: 4.0,
                        value: 0.0,
                    },
                    DataPoint {
                        coord: 4.0,
                        value: 0.0,
                    },
                ]
                .as_slice()
            )
        );
    }
}
