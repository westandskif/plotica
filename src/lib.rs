use std::cmp::min;
use std::time::{Duration, Instant};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// #[derive(Error, Debug)]
// pub enum CanvasInitError {
//
// }
enum BinSearchStrategy {
    LeftBound,
    RightBound,
}

fn bin_search_left_bound(data: &[f64], x: f64) -> Option<usize> {
    if data.is_empty() {
        return None;
    }
    let mut left_idx: usize = 0;
    let mut right_idx: usize = data.len() - 1;

    while left_idx <= right_idx {
        let middle_idx = (left_idx + right_idx) / 2;
        let current = *data.get(middle_idx).unwrap();
        if middle_idx == 0 {
            if current >= x {
                return Some(middle_idx);
            } else {
                left_idx = middle_idx + 1;
            }
        } else {
            if current >= x && *data.get(middle_idx - 1).unwrap() < x {
                return Some(middle_idx);
            }
            if current < x {
                left_idx = middle_idx + 1;
            } else {
                right_idx = middle_idx - 1;
            }
        }
    }
    None
}
fn bin_search_right_bound(data: &[f64], x: f64) -> Option<usize> {
    if data.is_empty() {
        return None;
    }
    let mut left_idx: usize = 0;
    let mut right_idx: usize = data.len() - 1;
    let max_idx = data.len() - 1;

    while left_idx <= right_idx {
        let middle_idx = (left_idx + right_idx) / 2;
        let current = *data.get(middle_idx).unwrap();
        if current <= x && (middle_idx == max_idx || *data.get(middle_idx + 1).unwrap() > x) {
            return Some(middle_idx);
        } else {
            if middle_idx == 0 {
                left_idx = middle_idx + 1;
            } else {
                if current <= x {
                    left_idx = middle_idx + 1;
                } else {
                    right_idx = middle_idx - 1;
                }
            }
        }
    }
    None
}
fn bin_search(data: &[f64], x: f64, strategy: BinSearchStrategy) -> Option<usize> {
    if data.is_empty() {
        return None;
    }
    let mut left_idx: usize = 0;
    let mut right_idx: usize = data.len() - 1;

    while left_idx <= right_idx {
        let middle_idx = (left_idx + right_idx) / 2;
        let current = *data.get(middle_idx).unwrap();

        if x < current {
            if middle_idx == 0 {
                return None;
            }
            right_idx = middle_idx - 1;
        } else if x > current {
            left_idx = middle_idx + 1;
        } else {
            return Some(middle_idx);
        }
    }
    None
}

enum CoordType {
    DateTime { tz_offset_ms: f64 },
    // Date,
}
enum ValueType {
    Number,
    // Money { currency_symbol: String, currency_suffix: String },
}

struct DataPoint {
    coord: f64,
    value: f64,
}
struct DataSet {
    name: String,
    data_points: Vec<DataPoint>,
    rgba: (u8, u8, u8, u8),
}
impl DataSet {
    pub fn slice_by_coord(&self, coord_start: f64, coord_end: f64) {}
}
struct Content {
    name: Option<String>,
    data_sets: Vec<DataSet>,
    coord_type: CoordType,
    value_type: ValueType,
}

trait Animate {
    fn get_value(&mut self) -> f64;
    fn set_value(&mut self, value: f64);
}

struct AnimatedNumber {
    x0: f64,
    x1: f64,
    k: f64,
    c: f64,
    v0: f64,
    t0: Option<Instant>,
    dt1: f64,
    dt2: f64,
}
impl AnimatedNumber {
    pub fn new(initial_value: f64, dt1: f64, dt2: f64) -> Self {
        Self {
            x0: initial_value,
            x1: initial_value,
            k: 0.0,
            c: 0.0,
            v0: 0.0,
            t0: None,
            dt1,
            dt2,
        }
    }
}
impl Animate for AnimatedNumber {
    fn get_value(&mut self) -> f64 {
        match self.t0 {
            Some(t0) => self.x0,
            None => {
                let us = t0.elapsed().as_micros() as f64;
                if us <= self.dt1 {
                    self.k * us * us / 2.0 + self.v0 * us
                } else if us >= self.dt2 + self.dt1 {
                    self.t0 = None;
                    self.x1
                } else {
                    self.k * self.dt1 * self.dt1 / 2.0
                        + self.v0 * self.dt1
                        + self.c * self.dt2.min(us - self.dt1)
                }
            }
        }
    }
    fn set_value(&mut self, new_value: f64) {
        self.v0 = match self.t0 {
            Some(t0) => self.v0 + self.k * self.dt1.min(t0.elapsed().as_micros() as f64),
            None => 0.0,
        };
        self.x0 = self.get_value();
        self.x1 = new_value;
        self.k = 2.0 * (self.x1 - self.x0 - self.v0 * self.dt2) / (2.0 * self.dt2 - self.dt1);
        self.c = self.k * self.dt1 + self.v0;
        self.t0 = Some(Instant::now());
    }
}

struct Screen {
    canvas: web_sys::HtmlCanvasElement,
    context: web_sys::CanvasRenderingContext2d,
    width: usize,
    height: usize,
    scale: f32,

    cam_coord: AnimatedNumber,
    cam_value: AnimatedNumber,
    cam_coord_range: AnimatedNumber,
    cam_value_range: AnimatedNumber,
}
impl Screen {
    pub fn get_drawing_ctx(&self) -> ScreenDrawingCtx {
        let cam_coord = self.cam_coord.get_value();
        let cam_value = self.cam_value.get_value();
        let cam_coord_range = self.cam_coord_range.get_value();
        let cam_value_range = self.cam_value_range.get_value();
        ScreenDrawingCtx {
            cam_coord,
            cam_value,
            cam_coord_range,
            cam_value_range,
            coord_min: cam_coord - cam_coord_range * 0.5,
            coord_max: cam_coord + cam_coord_range * 0.5,
            value_min: cam_value - cam_value_range * 0.5,
            value_max: cam_value + cam_value_range * 0.5,
        }
    }
}

struct ScreenDrawingCtx {
    cam_coord: f64,
    cam_value: f64,
    cam_coord_range: f64,
    cam_value_range: f64,
    coord_min: f64,
    coord_max: f64,
    value_min: f64,
    value_max: f64,
}
impl ScreenDrawingCtx {}

// impl Screen {
//     fn new(width_px: usize, height_px: usize) -> Result<Screen, JsValue> {
//         let window = web_sys::window().unwrap();
//         let document = window.document().unwrap();
//
//         let ppi = window.device_pixel_ratio();
//         let width = (width_px as f64 * ppi) as usize;
//         let height = (height_px as f64 * ppi) as usize;
//
//         let canvas = document
//             .create_element("canvas")?
//             .dyn_into::<web_sys::HtmlCanvasElement>()?;
//         document.body().unwrap().append_child(&canvas)?;
//         canvas.set_width(width.try_into().unwrap());
//         canvas.set_height(height.try_into().unwrap());
//         canvas.set_attribute(
//             "style",
//             format!("width: {}px; height: {}px", width_px, height_px).as_str(),
//         )?;
//
//         let context_options = js_sys::Object::new();
//         // js_sys::Reflect::set(
//         //     &context_options,
//         //     &JsValue::from("alpha"),
//         //     &JsValue::from(false),
//         // )?;
//         let context = canvas
//             // .get_context("2d")?
//             .get_context_with_context_options("2d", &context_options)?
//             .unwrap()
//             .dyn_into::<web_sys::CanvasRenderingContext2d>()?;
//
//         let data = vec![0 as u8; width * height * 4];
//         Ok(Screen {
//             canvas,
//             context,
//             width,
//             height,
//             scale: ppi as f32,
//             data,
//         })
//     }
//
//     fn color_pixel(&mut self, x: usize, y: usize, color: &Color, brightness: f32) {
//         let offset = (x + y * self.width) * 4;
//         unsafe {
//             *self.data.get_unchecked_mut(offset) = color.r;
//             *self.data.get_unchecked_mut(offset + 1) = color.g;
//             *self.data.get_unchecked_mut(offset + 2) = color.b;
//             *self.data.get_unchecked_mut(offset + 3) = (color.a as f32 * brightness) as u8;
//             // console_log(&format!("r = {}", *self.data.get_unchecked_mut(offset)));
//         }
//     }
//
//     fn draw_polyline(
//         &mut self,
//         points: &[Point],
//         color: &Color,
//         width: f32,
//         connection_type: LineConnectionType,
//     ) {
//         let last_index = points.len() - 1;
//         if last_index < 1 {
//             return;
//         }
//         let mut point_from = &points[0];
//         let half_width = width / 2.0;
//         let mut points: Vec<Point> = Vec::with_capacity(4);
//         let mut segments: Vec<Segment> = Vec::with_capacity(4);
//
//         for (index, point_to) in points.iter().enumerate().skip(1) {
//             let direction = Vector::from_points(point_from, point_to);
//             let normal = direction.get_normal(width * 0.5);
//
//             points.clear();
//             points.push(point_from.sub_vector(&normal));
//             points.push(points[0].add_vector(&direction));
//             points.push(point_from.add_vector(&normal));
//             points.push(points[2].add_vector(&direction));
//
//             let polygon = Polygon::from_points(points.as_slice()).unwrap();
//
//             for y in polygon.y_min as usize..polygon.y_max as usize {
//                 for (x_min, x_max) in polygon.get_x_intersections(y as f32).into_iter() {}
//                 let mut i1: Option<XIntersection> = None;
//                 let mut i2: Option<XIntersection> = None;
//                 let intersection = s1.get_x_intersection(y);
//                 match intersection {
//                     XIntersection::None => (),
//                     XIntersection::Point => {}
//                 }
//             }
//
//             // https://www.w3schools.com/tags/canvas_miterlimit.asp
//
//             let sharp_angle_depth = if index < last_index {
//                 let point_next = &points[index + 1];
//                 let cos_phi = (point_to.x * point_next.x + point_to.y * point_next.y)
//                     / ((point_to.x * point_to.x + point_to.y * point_to.y)
//                         * (point_next.x * point_next.x + point_next.y * point_next.y))
//                         .sqrt();
//                 if cos_phi > 0.0 {
//                     cos_phi * width
//                 } else {
//                     0.0
//                 }
//             } else {
//                 0.0
//             };
//
//             if dx == 0.0 {
//             } else {
//                 let k = dx / dy;
//                 let b = point_from.x - point_from.y * k;
//                 let x = k * y + b;
//                 if x < point_from.x || x > point_to.x {}
//             }
//             let y_length = (point_from.y - point_to.y).abs()
//                 + if dx == 0.0 {
//                     0.0
//                 } else {
//                     let k = dy / dx;
//                     width * (1.0 / (1.0 + k * k)).sqrt()
//                 };
//             let y_start = point_from.y.min(point_to.y);
//             for y in y_start as usize..((y_start + y_length) as usize) {}
//
//             // let min_delta = dy.abs().min(dx.abs().max(dy.abs()));
//             let min_delta = (dx * dx + dy * dy).sqrt();
//             dx /= min_delta;
//             dy /= min_delta;
//             let length_steps = (point_to.x - point_from.x) / dx;
//
//             let w_dx = dy;
//             let w_dy = -dx;
//             let width_steps = width;
//
//             let mut x = point_from.x;
//             let mut y = point_from.y;
//             console_log(&format!(
//                 "length steps {}; width steps {}",
//                 length_steps, width_steps
//             ));
//             console_log(&format!(
//                 "dx {}; dy {}; w_dx {}; w_dy {}",
//                 dx, dy, w_dx, w_dy,
//             ));
//             for i in 0..(length_steps as usize) {
//                 let (mut local_x, mut local_y) = (x, y);
//
//                 self.color_pixel(
//                     local_x as usize,
//                     local_y as usize,
//                     color,
//                     (1.0 - local_y.fract()) * (1.0 - local_x.fract()),
//                 );
//                 local_x += w_dx;
//                 local_y += w_dy;
//
//                 for _ in 1..((width_steps as usize) - 1) {
//                     self.color_pixel(
//                         local_x.floor() as usize,
//                         local_y.floor() as usize,
//                         color,
//                         1.0,
//                     );
//                     // self.color_pixel(
//                     //     local_x.ceil() as usize,
//                     //     local_y.ceil() as usize,
//                     //     color,
//                     //     1.0,
//                     // );
//                     // self.color_pixel(
//                     //     local_x.ceil() as usize,
//                     //     local_y.floor() as usize,
//                     //     color,
//                     //     1.0,
//                     // );
//                     // self.color_pixel(
//                     //     local_x.floor() as usize,
//                     //     local_y.ceil() as usize,
//                     //     color,
//                     //     1.0,
//                     // );
//                     local_x += w_dx;
//                     local_y += w_dy;
//                 }
//                 self.color_pixel(
//                     local_x as usize,
//                     local_y as usize,
//                     color,
//                     local_y.fract() * local_x.fract(),
//                 );
//
//                 x += dx;
//                 y += dy;
//             }
//
//             point_from = point_to;
//         }
//     }
//
//     fn draw_line(&mut self, color: &Color, mut x0: f32, mut y0: f32, mut x1: f32, mut y1: f32) {
//         // Xiaolin Wu's line algorithm
//         let is_steep = (y1 - y0).abs() > (x1 - x0).abs();
//         if is_steep {
//             (x0, y0, x1, y1) = (y0, x0, y1, x1);
//         }
//         if x0 > x1 {
//             (x0, y0, x1, y1) = (x1, y1, x0, y0);
//         }
//         let dx = x1 - x0;
//         let dy = y1 - y0;
//         let gradient = if dx == 0.0 { 1.0 } else { dy / dx };
//
//         let x = x0.round();
//         let y = y0 + gradient * (x - x0);
//         let x_gap = 1.0 - (x0 + 0.5).fract();
//         let x_px1 = x as usize;
//         let y_px1 = y.floor() as usize;
//
//         if is_steep {
//             self.color_pixel(y_px1, x_px1, color, (1.0 - y.fract()) * x_gap);
//             self.color_pixel(y_px1 + 1, x_px1, color, y.fract() * x_gap);
//         } else {
//             self.color_pixel(x_px1, y_px1, color, (1.0 - y.fract()) * x_gap);
//             self.color_pixel(x_px1, y_px1 + 1, color, y.fract() * x_gap);
//         }
//
//         let mut current_y = y + gradient;
//
//         let x = x1.round();
//         let y = y1 + gradient * (x - x1);
//         let x_gap = (x1 + 0.5).fract();
//         let x_px2 = x as usize;
//         let y_px2 = y.floor() as usize;
//
//         if is_steep {
//             self.color_pixel(y_px2, x_px2, color, (1.0 - y.fract()) * x_gap);
//             self.color_pixel(y_px2 + 1, x_px2, color, y.fract() * x_gap);
//             for x in x_px1 + 1..x_px2 {
//                 let y_fract = current_y.fract();
//                 let y = current_y as usize;
//                 self.color_pixel(y, x, color, 1.0 - y_fract);
//                 self.color_pixel(y + 1, x, color, y_fract);
//                 current_y += gradient;
//             }
//         } else {
//             self.color_pixel(x_px2, y_px2, color, (1.0 - y.fract()) * x_gap);
//             self.color_pixel(x_px2, y_px2 + 1, color, y.fract() * x_gap);
//             for x in x_px1 + 1..x_px2 {
//                 let y_fract = current_y.fract();
//                 let y = current_y as usize;
//                 self.color_pixel(x, y, color, 1.0 - y_fract);
//                 self.color_pixel(x, y + 1, color, y_fract);
//                 current_y += gradient;
//             }
//         }
//     }
// }
//
// // https://en.wikipedia.org/wiki/Xiaolin_Wu%27s_line_algorithm
// // Called when the wasm module is instantiated
// #[wasm_bindgen(start)]
// pub fn main() -> Result<(), JsValue> {
//     let window = web_sys::window().expect("no global `window` exists");
//     let document = window.document().expect("should have a document on window");
//     let body = document.body().expect("document should have a body");
//
//     let val = document.create_element("p")?;
//     val.set_inner_html("Hello from Rust!");
//
//     body.append_child(&val)?;
//
//     console_log(&format!("abs of -1 is {}", Math::abs(-1.0)));
//     console_log(&format!("1 + 2 = {}", add(1, 2)));
//     console_log_js_value(window.inner_width()?);
//
//     let mut screen = Screen::new(640, 480)?;
//     let red = Color::new(255, 0, 0, 255);
//
//     let mut points: Vec<Point> = Vec::new();
//     points.push(Point { x: 50.0, y: 200.0 });
//     points.push(Point {
//         x: 1200.0,
//         y: 800.0,
//     });
//     screen.draw_polyline(points.as_slice(), &red, 200.5, LineConnectionType::Hard);
//     console_log("finishing");
//
//     let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
//         wasm_bindgen::Clamped(&screen.data),
//         screen.width as u32,
//         screen.height as u32,
//     )?;
//     screen.context.put_image_data(&image_data, 0.0, 0.0)?;
//     Ok(())
// }
//
// #[wasm_bindgen]
// pub fn add(a: u32, b: u32) -> u32 {
//     a + b
// }
//
// #[wasm_bindgen]
// extern "C" {
//     #[wasm_bindgen(js_namespace = console, js_name = log)]
//     fn console_log(s: &str);
//
//     #[wasm_bindgen(js_namespace = console, js_name = log)]
//     fn console_log_js_value(v: JsValue);
// }
// // https://rustwasm.github.io/book/game-of-life/code-size.html#how-small-can-we-get-our-game-of-life-wasm-binary-via-build-configuration
// // https://chartio.com/learn/charts/line-chart-complete-guide/
